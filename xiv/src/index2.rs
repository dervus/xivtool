use crate::error::XivError;
use byteorder::{ReadBytesExt, LE};
use crc::{Crc, CRC_32_JAMCRC};
use nohash_hasher::IntMap;
use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Clone, Copy)]
pub struct IndexEntry {
    pub datnum: u8,
    pub offset: u64,
}

#[derive(Clone)]
pub struct Index2 {
    entries: IntMap<u32, IndexEntry>,
}

impl Index2 {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, XivError> {
        const MAGIC: &[u8] = b"SqPack\0\0";

        let mut r = File::open(path.as_ref()).map_err(XivError::IO)?;

        let mut magic = [0u8; MAGIC.len()];
        r.read_exact(&mut magic).map_err(XivError::Index2Header)?;
        if magic != MAGIC {
            return Err(XivError::Index2Header(std::io::ErrorKind::Other.into()));
        }

        r.seek(SeekFrom::Start(0x0C))
            .map_err(XivError::Index2Seek)?;
        let header_offset = r.read_u32::<LE>().map_err(XivError::Index2Header)? as u64;

        r.seek(SeekFrom::Start(header_offset + 8))
            .map_err(XivError::Index2Seek)?;
        let entries_offset = r.read_u32::<LE>().map_err(XivError::Index2Header)? as u64;
        let entries_count = (r.read_u32::<LE>().map_err(XivError::Index2Header)? / 8) as usize;

        r.seek(SeekFrom::Start(entries_offset))
            .map_err(XivError::Index2Seek)?;
        let mut r = BufReader::new(r);
        let mut entries = IntMap::with_capacity_and_hasher(entries_count, Default::default());
        for _ in 0..entries_count {
            let hash = r.read_u32::<LE>().map_err(XivError::Index2Entry)?;
            let location = r.read_u32::<LE>().map_err(XivError::Index2Entry)?;

            let datnum = (location & 0x00000007) >> 1;
            let offset = (location & 0xFFFFFFF8) << 3;

            let entry = IndexEntry {
                datnum: datnum as u8,
                offset: offset as u64,
            };

            entries.insert(hash, entry);
        }

        Ok(Index2 { entries })
    }

    pub fn find(&self, path: impl AsRef<[u8]>) -> Option<IndexEntry> {
        const HASHER: Crc<u32> = Crc::<u32>::new(&CRC_32_JAMCRC);

        let hash = HASHER.checksum(path.as_ref());
        self.entries.get(&hash).cloned()
    }
}

impl Debug for Index2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Index2 {{ {} entries }}", self.entries.len()))
    }
}
