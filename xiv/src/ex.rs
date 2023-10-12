use crate::{dat::InnerFilePtr, error::XivError, sqpack::SqPack};
use binrw::{binread, BinRead};
use serde::{de, forward_to_deserialize_any, Deserialize, Serialize};
use std::{
    fmt,
    io::{Cursor, Seek, SeekFrom},
    iter::FusedIterator,
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[binread]
#[br(little, repr = u16)]
pub enum Locale {
    None = 0x0,
    Japanese,
    English,
    German,
    French,
    ChineseSimplified,
    ChineseTraditional,
    Korean,
}

impl Locale {
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Japanese => "_ja",
            Self::English => "_en",
            Self::German => "_de",
            Self::French => "_fr",
            Self::ChineseSimplified => "_chs",
            Self::ChineseTraditional => "_cht",
            Self::Korean => "_ko",
        }
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.suffix())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[binread]
#[br(big, repr = u16)]
pub enum ValueType {
    String = 0x0,
    Bool = 0x1,
    Int8 = 0x2,
    UInt8 = 0x3,
    Int16 = 0x4,
    UInt16 = 0x5,
    Int32 = 0x6,
    UInt32 = 0x7,
    Float32 = 0x9,
    Int64 = 0xA,
    UInt64 = 0xB,
    PackedBool0 = 0x19,
    PackedBool1 = 0x1A,
    PackedBool2 = 0x1B,
    PackedBool3 = 0x1C,
    PackedBool4 = 0x1D,
    PackedBool5 = 0x1E,
    PackedBool6 = 0x1F,
    PackedBool7 = 0x20,
}

impl ValueType {
    pub fn type_tag(self) -> &'static str {
        match self {
            Self::String => "str",
            Self::Bool
            | Self::PackedBool0
            | Self::PackedBool1
            | Self::PackedBool2
            | Self::PackedBool3
            | Self::PackedBool4
            | Self::PackedBool5
            | Self::PackedBool6
            | Self::PackedBool7 => "bool",
            Self::Int8 => "i8",
            Self::UInt8 => "u8",
            Self::Int16 => "i16",
            Self::UInt16 => "u16",
            Self::Int32 => "i32",
            Self::UInt32 => "u32",
            Self::Float32 => "f32",
            Self::Int64 => "i64",
            Self::UInt64 => "u64",
        }
    }
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.type_tag())
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    String(Box<str>),
}

impl Value {
    pub fn type_tag(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::Int8(_) => "i8",
            Self::UInt8(_) => "u8",
            Self::Int16(_) => "i16",
            Self::UInt16(_) => "u16",
            Self::Int32(_) => "i32",
            Self::UInt32(_) => "u32",
            Self::Int64(_) => "i64",
            Self::UInt64(_) => "u64",
            Self::Float(_) => "f32",
            Self::String(_) => "str",
        }
    }
}

struct ValueVisitor;

impl<'de> de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "exd row value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        Ok(Value::Int8(v))
    }
    fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
        Ok(Value::Int16(v))
    }
    fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
        Ok(Value::Int32(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Value::Int64(v))
    }
    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(Value::UInt8(v))
    }
    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(Value::UInt16(v))
    }
    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(Value::UInt32(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Value::UInt64(v))
    }
    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok(Value::Float(v))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Value::String(v.into()))
    }
}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

pub type Row = Vec<Value>;

#[derive(Debug)]
#[binread]
#[br(big, magic = b"EXHF")]
pub struct Exh {
    pub unk0: u16, // version?
    pub data_offset: u16,
    pub column_count: u16,
    pub page_count: u16,
    pub language_count: u16,
    pub unk1: u16,
    pub u2: u8, // padding?
    pub variant: ExVariant,
    pub unk2: u16,
    pub row_count: u32,
    pub unk3: u32,
    pub unk4: u32,
    #[br(count = column_count)]
    pub columns: Vec<ExColumn>,
    #[br(count = page_count)]
    pub pages: Vec<ExPage>,
    #[br(count = language_count)]
    pub languages: Vec<Locale>,
}

#[derive(Debug, PartialEq, Eq)]
#[binread]
#[br(big, repr = u8)]
pub enum ExVariant {
    Normal = 1,
    SubRows = 2,
}

#[derive(Debug)]
#[binread]
#[br(big)]
pub struct ExColumn {
    pub vtype: ValueType,
    pub offset: u16,
}

#[derive(Debug)]
#[binread]
#[br(big)]
pub struct ExPage {
    pub start_id: u32,
    pub row_count: u32,
}

#[binread]
#[br(big, magic = b"EXDF")]
struct ExdHeader {
    pub _version: u16,
    pub _unk0: u16,
    pub _index_size: u32,
    pub _unk1: u32,
    pub _unk2: u32,
    pub _unk3: u32,
    pub _unk4: u32,
    pub _unk5: u32,
    #[br(count = _index_size / 8)]
    pub rows: Vec<ExdRowPtr>,
}

#[binread]
#[br(big)]
struct ExdRowPtr {
    pub id: u32,
    pub offset: u32,
}

struct ExdRowReader {
    exh: Rc<Exh>,
    exd_data: Rc<[u8]>,
    id: u32,
    id_expected: bool,
    subid: u16,
    subid_expected: bool,
    offset: u64,
    column_idx: usize,
}

impl ExdRowReader {
    pub fn new(exh: Rc<Exh>, exd_data: Rc<[u8]>, id: u32, subid: Option<u16>, offset: u64) -> Self {
        Self {
            exh,
            exd_data,
            id,
            id_expected: true,
            subid: subid.unwrap_or(0),
            subid_expected: subid.is_some(),
            offset,
            column_idx: 0,
        }
    }
}

#[derive(Debug)]
struct ExdDeserializerError(Box<str>);

impl fmt::Display for ExdDeserializerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExdDeserializerError {}

impl de::Error for ExdDeserializerError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(msg.to_string().into_boxed_str())
    }
}

impl From<std::io::Error> for ExdDeserializerError {
    fn from(source: std::io::Error) -> Self {
        Self(source.to_string().into_boxed_str())
    }
}

impl From<binrw::Error> for ExdDeserializerError {
    fn from(source: binrw::Error) -> Self {
        Self(source.to_string().into_boxed_str())
    }
}

impl<'de> de::Deserializer<'de> for &mut ExdRowReader {
    type Error = ExdDeserializerError;

    fn is_human_readable(&self) -> bool {
        false
    }

    fn deserialize_any<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
        if self.id_expected {
            self.id_expected = false;
            v.visit_u32(self.id)
        } else if self.subid_expected {
            self.subid_expected = false;
            v.visit_u16(self.subid)
        } else {
            let column = self
                .exh
                .columns
                .get(self.column_idx)
                .ok_or_else(|| ExdDeserializerError("not enough columns in exd file".into()))?;

            let mut cursor = Cursor::new(&self.exd_data);
            cursor.seek(SeekFrom::Start(self.offset + column.offset as u64))?;
            self.column_idx += 1;

            match column.vtype {
                ValueType::Int8 => v.visit_i8(i8::read_be(&mut cursor)?),
                ValueType::Int16 => v.visit_i16(i16::read_be(&mut cursor)?),
                ValueType::Int32 => v.visit_i32(i32::read_be(&mut cursor)?),
                ValueType::Int64 => v.visit_i64(i64::read_be(&mut cursor)?),
                ValueType::UInt8 => v.visit_u8(u8::read_be(&mut cursor)?),
                ValueType::UInt16 => v.visit_u16(u16::read_be(&mut cursor)?),
                ValueType::UInt32 => v.visit_u32(u32::read_be(&mut cursor)?),
                ValueType::UInt64 => v.visit_u64(u64::read_be(&mut cursor)?),
                ValueType::Float32 => v.visit_f32(f32::read_be(&mut cursor)?),
                ValueType::Bool => v.visit_bool(u8::read_be(&mut cursor)? != 0),
                ValueType::PackedBool0 => v.visit_bool(u8::read_be(&mut cursor)? & 1 != 0),
                ValueType::PackedBool1 => v.visit_bool(u8::read_be(&mut cursor)? & 2 != 0),
                ValueType::PackedBool2 => v.visit_bool(u8::read_be(&mut cursor)? & 3 != 0),
                ValueType::PackedBool3 => v.visit_bool(u8::read_be(&mut cursor)? & 4 != 0),
                ValueType::PackedBool4 => v.visit_bool(u8::read_be(&mut cursor)? & 5 != 0),
                ValueType::PackedBool5 => v.visit_bool(u8::read_be(&mut cursor)? & 6 != 0),
                ValueType::PackedBool6 => v.visit_bool(u8::read_be(&mut cursor)? & 7 != 0),
                ValueType::PackedBool7 => v.visit_bool(u8::read_be(&mut cursor)? & 8 != 0),
                ValueType::String => {
                    let str_offset = u32::read_be(&mut cursor)?;
                    let abs_offset = self.offset + self.exh.data_offset as u64 + str_offset as u64;
                    cursor.seek(SeekFrom::Start(abs_offset))?;
                    v.visit_string(binrw::NullString::read(&mut cursor)?.to_string())
                }
            }
        }
    }

    #[inline]
    fn deserialize_seq<V: de::Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
        v.visit_seq(self)
    }

    #[inline]
    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(v)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct map enum identifier ignored_any
    }
}

impl<'de> de::SeqAccess<'de> for ExdRowReader {
    type Error = ExdDeserializerError;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.column_idx < self.exh.columns.len() {
            seed.deserialize(self).map(Some)
        } else {
            Ok(None)
        }
    }
}

struct ExdPageReader<T> {
    row_type: PhantomData<T>,
    exh: Rc<Exh>,
    exd_fileptr: InnerFilePtr,
    exd_data: Option<Rc<[u8]>>,
    exd_header: Option<Rc<ExdHeader>>,
    row_index: usize,
    subrow_index: u16,
    subrow_count: u16,
    done: bool,
}

impl<'de, T> ExdPageReader<T>
where
    T: Sized + Deserialize<'de>,
{
    pub fn new(exh: Rc<Exh>, exd_fileptr: InnerFilePtr) -> Self {
        Self {
            row_type: PhantomData,
            exh,
            exd_fileptr,
            exd_data: None,
            exd_header: None,
            row_index: 0,
            subrow_index: 0,
            subrow_count: 0,
            done: false,
        }
    }

    fn lazy_exd_data(&mut self) -> Result<Rc<[u8]>, XivError> {
        assert!(!self.done);
        if self.exd_data.is_none() {
            let exd_file = self.exd_fileptr.read_plain()?;
            self.exd_data = Some(exd_file.into());
        }
        Ok(self.exd_data.as_ref().unwrap().clone())
    }

    fn lazy_exd_header(&mut self) -> Result<Rc<ExdHeader>, XivError> {
        assert!(!self.done);
        if self.exd_header.is_none() {
            let header = ExdHeader::read(&mut Cursor::new(self.lazy_exd_data()?))
                .map_err(XivError::ExdFileHeader)?;
            self.exd_header = Some(Rc::new(header));
        }
        Ok(self.exd_header.as_ref().unwrap().clone())
    }

    fn read_next_row(&mut self) -> Result<Option<T>, XivError> {
        if self.done {
            return Ok(None);
        }
        if let Some(row_ptr) = self.lazy_exd_header()?.rows.get(self.row_index) {
            let mut cursor = Cursor::new(self.lazy_exd_data()?);
            cursor
                .seek(SeekFrom::Start(row_ptr.offset as u64))
                .map_err(XivError::ExdSeek)?;

            let _size = u32::read_be(&mut cursor).map_err(XivError::ExdRowHeader)?;
            self.subrow_count = u16::read_be(&mut cursor).map_err(XivError::ExdRowHeader)?;

            let row = self.read_next_subrow()?;
            if self.subrow_index >= self.subrow_count {
                self.row_index += 1;
                self.subrow_index = 0;
                self.subrow_count = 0;
            };
            Ok(row)
        } else {
            self.done = true;
            self.exd_data = None;
            self.exd_header = None;
            Ok(None)
        }
    }

    fn read_next_subrow(&mut self) -> Result<Option<T>, XivError> {
        if self.done {
            return Ok(None);
        }
        if let Some(row_ptr) = self.lazy_exd_header()?.rows.get(self.row_index) {
            if self.subrow_index >= self.subrow_count {
                return Ok(None);
            }

            let subrow_offset = match self.exh.variant {
                ExVariant::Normal => 0,
                ExVariant::SubRows => (2 + self.exh.data_offset) * self.subrow_index,
            } as u64;

            let mut cursor = Cursor::new(self.lazy_exd_data()?);
            let row_header_size = 6;
            cursor
                .seek(SeekFrom::Start(
                    row_ptr.offset as u64 + row_header_size + subrow_offset,
                ))
                .map_err(XivError::ExdSeek)?;

            let subid = match self.exh.variant {
                ExVariant::Normal => None,
                ExVariant::SubRows => {
                    Some(u16::read_be(&mut cursor).map_err(XivError::ExdSubRowHeader)?)
                }
            };

            let row = T::deserialize(&mut ExdRowReader::new(
                self.exh.clone(),
                self.lazy_exd_data()?,
                row_ptr.id,
                subid,
                cursor.position(),
            ))
            .map_err(|e| XivError::ExdDeserialization(e.0))?;

            self.subrow_index += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}

impl<'de, T> Iterator for ExdPageReader<T>
where
    T: Sized + Deserialize<'de>,
{
    type Item = Result<T, XivError>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_rows = self.exh.row_count as usize - self.row_index;
        match self.exh.variant {
            ExVariant::Normal => (remaining_rows, Some(remaining_rows)),
            ExVariant::SubRows => (
                remaining_rows,
                remaining_rows.checked_mul(u16::MAX as usize),
            ),
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.read_next_row().transpose()
    }
}

impl<'de, T> FusedIterator for ExdPageReader<T> where T: Sized + Deserialize<'de> {}

pub fn read_exh(repo: Arc<SqPack>, base_path: &str) -> Result<Exh, XivError> {
    let base_path = base_path.to_lowercase();
    let exh_path = format!("exd/{base_path}.exh").into_boxed_str();
    let exh_file = repo
        .find(&exh_path)?
        .ok_or(XivError::ExhNotFound(exh_path))?
        .read_plain()?;

    Exh::read(&mut Cursor::new(exh_file)).map_err(XivError::Exh)
}

pub fn read_exd<'de, T>(
    repo: Arc<SqPack>,
    base_path: &str,
    locale: Locale,
) -> Result<impl Iterator<Item = Result<T, XivError>>, XivError>
where
    T: Sized + Serialize + Deserialize<'de> + 'static,
{
    let base_path = base_path.to_lowercase();
    let exh = Rc::new(read_exh(repo.clone(), &base_path)?);
    let exd_locale = exh
        .languages
        .iter()
        .cloned()
        .find(|l| *l == locale)
        .or(exh.languages.first().cloned())
        .unwrap_or(Locale::None);

    let mut fileptrs = Vec::with_capacity(exh.pages.len());
    for page in &exh.pages {
        let start_id = page.start_id;
        let exd_path = format!("exd/{base_path}_{start_id}{exd_locale}.exd").into_boxed_str();
        let exd_fileptr = repo
            .find(&exd_path)?
            .ok_or(XivError::ExdNotFound(exd_path))?;
        fileptrs.push(exd_fileptr);
    }

    Ok(fileptrs
        .into_iter()
        .flat_map(move |fileptr| ExdPageReader::new(exh.clone(), fileptr)))
}
