use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XivError {
    #[error("SqPack repository file name is invalid")]
    PackIdRepoFile,
    #[error("SqPack inner file path is invalid")]
    PackIdInnerPath,
    #[error("SqPack inner file path contains invalid category identifier")]
    PackIdCategory,
    #[error("SqPack inner file path contains invalid expansion identifier")]
    PackIdExpansion,
    #[error("SqPack inner file path contains invalid patch identifier")]
    PackIdPatch,

    #[error("Failed to seek within .index2 file")]
    Index2Seek(#[source] io::Error),
    #[error("Failed to read .index2 header")]
    Index2Header(#[source] io::Error),
    #[error("Failed to read .index2 entry")]
    Index2Entry(#[source] io::Error),

    #[error("Failed to seek within .dat file")]
    DatSeek(#[source] io::Error),
    #[error("Failed to read .dat inner file header")]
    DatFileHeader(#[source] binrw::Error),
    #[error("Failed to read .dat inner file block header")]
    DatBlockHeader(#[source] binrw::Error),
    #[error("Failed to decode .dat inner file block")]
    DatBlockDecoding(#[source] io::Error),

    #[error("Failed to read .exh file")]
    Exh(#[source] binrw::Error),
    #[error("Unable to find {0}")]
    ExhNotFound(Box<str>),
    #[error("Unable to find {0}")]
    ExdNotFound(Box<str>),
    #[error("Failed to seek within .exd file")]
    ExdSeek(#[source] io::Error),
    #[error("Failed to read .exd file header")]
    ExdFileHeader(#[source] binrw::Error),
    #[error("Failed to read .exd row header")]
    ExdRowHeader(#[source] binrw::Error),
    #[error("Failed to read .exd subrow header")]
    ExdSubRowHeader(#[source] binrw::Error),
    #[error("Failed to deserialize .exd row ({0})")]
    ExdDeserialization(Box<str>),

    #[error("Unable to export an image with format={0}, which is not implemented yet")]
    TexFormat(u32),
    #[error("Image's pixel data is invalid or corrupted")]
    TexData,

    #[error(transparent)]
    IO(io::Error),
}
