use crate::error::XivError;
use binrw::{binread, BinRead};
use flate2::read::DeflateDecoder;
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileType {
    Empty,
    Plain,
    Model,
    Image,
    Unknown(u32),
}

impl From<u32> for FileType {
    fn from(v: u32) -> FileType {
        match v {
            1 => FileType::Empty,
            2 => FileType::Plain,
            3 => FileType::Model,
            4 => FileType::Image,
            x => FileType::Unknown(x),
        }
    }
}

impl binrw::BinRead for FileType {
    type Args<'a> = ();
    fn read_options<'a, R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        _args: Self::Args<'a>,
    ) -> binrw::BinResult<Self> {
        use binrw::BinReaderExt;
        let id: u32 = reader.read_le()?;
        Ok(id.into())
    }
}

#[derive(Debug)]
#[binread]
#[br(little)]
pub struct FileHeader {
    pub header_size: u32,
    pub file_type: FileType,
    pub uncompressed_size: u32,
    pub unk0: u32,       // buffer1?
    pub unk1: u32,       // buffer2?
    pub chunks_num: u32, // seems to always be 0x1000005 on model files
    #[br(args(file_type, chunks_num))]
    pub addon: FileHeaderAddon,
}

#[derive(Debug)]
#[binread]
#[br(little, import(file_type: FileType, chunks_num: u32))]
pub enum FileHeaderAddon {
    #[br(pre_assert(file_type == FileType::Plain))]
    Plain {
        #[br(count = chunks_num)]
        chunks: Vec<PlainChunkHeader>,
    },
    #[br(pre_assert(file_type == FileType::Model))]
    Model {
        chunks: ModelChunkHeaderArray,
        #[br(args(chunks.block_count.iter().map(|x| *x as usize).sum()))]
        model: ModelFileHeader,
    },
    #[br(pre_assert(file_type == FileType::Image))]
    Image {
        #[br(count = chunks_num)]
        chunks: Vec<ImageChunkHeader>,
        image: ImageFileHeader,
    },
    None,
}

#[derive(Debug)]
#[binread]
#[br(little)]
pub struct PlainChunkHeader {
    pub chunk_offset: u32,
    pub unk0: u32,
}

pub const MODEL_CHUNKS_NUM: usize = 11;

#[derive(Debug)]
#[binread]
#[br(little)]
pub struct ModelChunkHeaderArray {
    pub size: [u32; MODEL_CHUNKS_NUM],
    pub len: [u32; MODEL_CHUNKS_NUM],
    pub offset: [u32; MODEL_CHUNKS_NUM],
    pub block_start: [u16; MODEL_CHUNKS_NUM],
    pub block_count: [u16; MODEL_CHUNKS_NUM],
}

#[derive(Debug)]
#[binread]
#[br(little)]
pub struct ImageChunkHeader {
    pub offset: u32,
    pub len: u32,
    pub size: u32,
    pub block_start: u32,
    pub block_count: u32,
}

#[derive(Debug)]
#[binread]
#[br(little, import(block_count_total: usize))]
pub struct ModelFileHeader {
    pub mesh_count: u16,
    pub material_count: u16,
    pub unk0: u32,
    #[br(count = block_count_total)]
    pub block_sizes: Vec<u16>,
}

#[derive(Debug)]
#[binread]
#[br(little)]
pub struct ImageFileHeader {
    pub format: u32,
    pub width: u16,
    pub height: u16,
    pub layers: u16,
    pub count: u16,
}

fn read_block(mut input: impl Read + Seek, mut output: impl Write) -> Result<u64, XivError> {
    const _BLOCK_PADDING: usize = 128; // might be required for some files; ignore for now
    const COMPRESSION_THRESHOLD: u32 = 32000;

    #[binread]
    #[br(little, magic = 0x00000010u32)]
    struct BlockHeader {
        _unk0: u32,
        size_compressed: u32,
        size_uncompressed: u32,
    }

    let header = BlockHeader::read(&mut input).map_err(XivError::DatBlockHeader)?;
    let is_compressed = header.size_compressed < COMPRESSION_THRESHOLD;

    if is_compressed {
        let mut block = input.take(header.size_compressed as u64);
        let mut decoder = DeflateDecoder::new(&mut block);
        io::copy(&mut decoder, &mut output).map_err(XivError::DatBlockDecoding)
    } else {
        let mut block = input.take(header.size_uncompressed as u64);
        io::copy(&mut block, &mut output).map_err(XivError::DatBlockDecoding)
    }
}

pub struct InnerFilePtr {
    pub path: PathBuf,
    pub offset: u64,
}

pub struct InnerFile {
    pub header: FileHeader,
    pub contents: Box<[u8]>,
}

impl InnerFilePtr {
    pub fn read(&self) -> Result<InnerFile, XivError> {
        let mut fd = std::fs::File::open(&self.path).map_err(XivError::IO)?;

        fd.seek(SeekFrom::Start(self.offset))
            .map_err(XivError::DatSeek)?;
        let header = FileHeader::read(&mut fd).map_err(XivError::DatFileHeader)?;

        let mut data = Cursor::new(Vec::with_capacity(header.uncompressed_size as usize));

        match &header.addon {
            FileHeaderAddon::Plain { chunks } => {
                for chunk in chunks {
                    fd.seek(SeekFrom::Start(
                        self.offset + header.header_size as u64 + chunk.chunk_offset as u64,
                    ))
                    .map_err(XivError::DatSeek)?;
                    read_block(&mut fd, &mut data)?;
                }
            }
            FileHeaderAddon::Model { chunks, .. } => {
                for chunk_idx in 0..MODEL_CHUNKS_NUM {
                    fd.seek(SeekFrom::Start(
                        self.offset + header.header_size as u64 + chunks.offset[chunk_idx] as u64,
                    ))
                    .map_err(XivError::DatSeek)?;
                    for _block_idx in 0..chunks.block_count[chunk_idx] {
                        read_block(&mut fd, &mut data)?;
                    }
                }
            }
            FileHeaderAddon::Image { chunks, .. } => {
                for chunk in chunks {
                    fd.seek(SeekFrom::Start(
                        self.offset + header.header_size as u64 + chunk.offset as u64,
                    ))
                    .map_err(XivError::DatSeek)?;
                    for _block_idx in 0..chunk.block_count {
                        read_block(&mut fd, &mut data)?;
                    }
                }
            }
            FileHeaderAddon::None => {
                // no data to read
            }
        }

        let contents = data.into_inner().into_boxed_slice();
        Ok(InnerFile { header, contents })
    }
}
