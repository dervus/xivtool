use crate::error::XivError;
use binrw::{binread, BinRead};
use flate2::read::DeflateDecoder;
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

#[binread]
#[br(little, repr(u32))]
enum FileType {
    Empty = 1,
    Plain = 2,
    Model = 3,
    Image = 4,
}

fn read_block(mut input: impl Read + Seek, mut output: impl Write) -> Result<(), XivError> {
    const BLOCK_HEADER_LEN: u64 = 16;
    const BLOCK_PADDING: u64 = 128;
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
    let read_size = if is_compressed { header.size_compressed } else { header.size_uncompressed } as u64;

    if is_compressed {
        let mut block = input.take(read_size);
        let mut decoder = DeflateDecoder::new(&mut block);
        io::copy(&mut decoder, &mut output).map_err(XivError::DatBlockDecoding)?;
        input = block.into_inner();
    } else {
        let mut block = input.take(read_size);
        io::copy(&mut block, &mut output).map_err(XivError::DatBlockDecoding)?;
        input = block.into_inner();
    }

    let padding = BLOCK_PADDING - (BLOCK_HEADER_LEN + read_size) % BLOCK_PADDING;
    input.seek(SeekFrom::Current(padding as i64)).map_err(XivError::DatSeek)?;

    Ok(())
}

fn read_plain_file(mut input: impl Read + Seek) -> Result<Box<[u8]>, XivError> {
    #[binread]
    #[br(little)]
    struct FileHeader {
        len: u32,
        _file_type: FileType,
        data_len: u32,
        _unk0: u32,
        _unk1: u32,
        #[allow(dead_code)] // linter false positive
        chunks_num: u32,
        #[br(count = chunks_num)]
        chunks: Vec<ChunkHeader>,
    }

    #[binread]
    #[br(little)]
    struct ChunkHeader {
        offset: u32,
        _unk0: u32,
    }

    let offset = input.stream_position().map_err(XivError::DatSeek)?;
    let header = FileHeader::read(&mut input).map_err(XivError::DatFileHeader)?;

    let mut data = Cursor::new(Vec::with_capacity(header.data_len as usize));
    for chunk in header.chunks {
        let chunk_offset = offset + header.len as u64 + chunk.offset as u64;
        input.seek(SeekFrom::Start(chunk_offset)).map_err(XivError::DatSeek)?;
        read_block(&mut input, &mut data)?;
    }

    Ok(data.into_inner().into_boxed_slice())
}

#[allow(dead_code)]
fn read_model_file(mut input: impl Read + Seek) -> Result<(), XivError> {
    const MODEL_CHUNKS_NUM: usize = 11;

    #[binread]
    #[br(little)]
    struct FileHeader {
        len: u32,
        file_type: FileType,
        data_len: u32,
        unk0: u32,
        unk1: u32,
        unk2: u32, // seems to always be 0x1000005
        chunk_size: [u32; MODEL_CHUNKS_NUM],
        chunk_len: [u32; MODEL_CHUNKS_NUM],
        chunk_offset: [u32; MODEL_CHUNKS_NUM],
        block_start: [u16; MODEL_CHUNKS_NUM],
        block_count: [u16; MODEL_CHUNKS_NUM],
        meshes_num: u16,
        materials_num: u16,
        unk3: u32,
        #[br(count = block_count.iter().map(|x| *x as usize).sum::<usize>())]
        block_lens: Vec<u16>,
    }

    let _header = FileHeader::read(&mut input).map_err(XivError::DatFileHeader)?;
    todo!("Reading of model files is not implemented yet")
}

pub type ImageData = Box<[u8]>;

pub struct Image {
    pub format: u32,
    pub width: u16,
    pub height: u16,
    pub layers: u16,
    pub count: u16,
    pub mipmaps: Box<[ImageData]>,
}

fn read_image_file(mut input: impl Read + Seek) -> Result<Image, XivError> {
    #[binread]
    #[br(little)]
    struct FileHeader {
        len: u32,
        _file_type: FileType,
        _data_len: u32,
        _unk0: u32,
        _unk1: u32,
        #[allow(dead_code)] // linter false positive
        mipmaps_num: u32,
        #[br(count = mipmaps_num)]
        mipmaps: Vec<MipmapHeader>,
    }

    #[binread]
    #[br(little)]
    struct MipmapHeader {
        offset: u32,
        len: u32,
        _size: u32,
        _block_start: u32,
        block_count: u32,
    }

    #[binread]
    #[br(little)]
    struct ImageHeader {
        _unk0: u32,
        format: u32,
        width: u16,
        height: u16,
        layers: u16,
        count: u16,
    }

    let offset = input.stream_position().map_err(XivError::DatSeek)?;
    let header = FileHeader::read(&mut input).map_err(XivError::DatFileHeader)?;
    input.seek(SeekFrom::Start(offset + header.len as u64)).map_err(XivError::DatSeek)?;
    let image = ImageHeader::read(&mut input).map_err(XivError::DatFileHeader)?;

    let mut mipmaps = Vec::with_capacity(header.mipmaps.len());
    for mipmap in header.mipmaps {
        let mipmap_offset = offset + header.len as u64 + mipmap.offset as u64;
        input.seek(SeekFrom::Start(mipmap_offset)).map_err(XivError::DatSeek)?;

        let mut data = Cursor::new(Vec::with_capacity(mipmap.len as usize));
        for _block_idx in 0..mipmap.block_count {
            read_block(&mut input, &mut data)?;
        }
        mipmaps.push(data.into_inner().into_boxed_slice());
    }

    Ok(Image {
        format: image.format,
        width: image.width,
        height: image.height,
        layers: image.layers,
        count: image.count,
        mipmaps: mipmaps.into_boxed_slice(),
    })
}

pub struct InnerFilePtr {
    pub path: PathBuf,
    pub offset: u64,
}

impl InnerFilePtr {
    fn open(&self) -> Result<std::fs::File, XivError> {
        let mut fd = std::fs::File::open(&self.path).map_err(XivError::IO)?;
        fd.seek(SeekFrom::Start(self.offset)).map_err(XivError::DatSeek)?;
        Ok(fd)
    }

    pub fn read_plain(&self) -> Result<Box<[u8]>, XivError> {
        self.open().and_then(read_plain_file)
    }

    pub fn read_model(&self) -> Result<(), XivError> {
        self.open().and_then(read_model_file)
    }

    pub fn read_image(&self) -> Result<Image, XivError> {
        self.open().and_then(read_image_file)
    }
}
