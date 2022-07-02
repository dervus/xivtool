use crate::error::XivError;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
};

lazy_static! {
    static ref CATEGORY_NAME_TO_ID: HashMap<&'static str, u8> = {
        let mut m = HashMap::new();
        m.insert("common", 0x00);
        m.insert("bgcommon", 0x01);
        m.insert("bg", 0x02);
        m.insert("cut", 0x03);
        m.insert("chara", 0x04);
        m.insert("shader", 0x05);
        m.insert("ui", 0x06);
        m.insert("sound", 0x07);
        m.insert("vfx", 0x08);
        m.insert("ui_script", 0x09);
        m.insert("exd", 0x0a);
        m.insert("game_script", 0x0b);
        m.insert("music", 0x0c);
        m.insert("_sqpack_test", 0x12);
        m.insert("_debug", 0x13);
        m.shrink_to_fit();
        m
    };
    static ref EXPANSION_REGEX: Regex = Regex::new(r"^ex([1-9])$").unwrap();
    static ref PATCH_REGEX: Regex = Regex::new(r"^([0-9a-f]{2})_").unwrap();
    static ref SQPACK_NAME_REGEX: Regex =
        Regex::new(r"^([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2}).win32.(dat\d|index|index2)$")
            .unwrap();
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackId {
    pub category: u8,
    pub expansion: u8,
    pub patch: u8,
}

impl PackId {
    pub fn new(category: u8, expansion: u8, patch: u8) -> Self {
        Self {
            category,
            expansion,
            patch,
        }
    }

    pub fn from_inner_path(path: impl AsRef<str>) -> Result<Self, XivError> {
        let mut path_nodes = path.as_ref().split('/');
        let mut next_node = || path_nodes.next().ok_or(XivError::PackIdInnerPath);

        let category = *CATEGORY_NAME_TO_ID
            .get(next_node()?)
            .ok_or(XivError::PackIdCategory)?;
        let mut expansion = 0u8; // 0 implies "ffxiv" directory
        let mut patch = 0u8; // it's always 0 for files in "ffxiv" directory

        if let Some(cap) = EXPANSION_REGEX.captures(next_node()?) {
            expansion = u8::from_str_radix(&cap[1], 10).map_err(|_| XivError::PackIdExpansion)?;

            if let Some(cap) = PATCH_REGEX.captures(next_node()?) {
                patch = u8::from_str_radix(&cap[1], 16).map_err(|_| XivError::PackIdPatch)?;
            }
        }

        Ok(Self::new(category, expansion, patch))
    }

    pub fn from_repo_path(path: impl AsRef<Path>) -> Result<Self, XivError> {
        let file_name = path.as_ref().file_name().ok_or(XivError::PackIdRepoFile)?;
        if let Some(cap) = SQPACK_NAME_REGEX.captures(&file_name.to_string_lossy()) {
            let category = u8::from_str_radix(&cap[1], 16).map_err(|_| XivError::PackIdCategory)?;
            let expansion =
                u8::from_str_radix(&cap[2], 16).map_err(|_| XivError::PackIdExpansion)?;
            let patch = u8::from_str_radix(&cap[3], 16).map_err(|_| XivError::PackIdPatch)?;

            Ok(Self::new(category, expansion, patch))
        } else {
            Err(XivError::PackIdRepoFile)
        }
    }

    fn into_repo_path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        if self.expansion == 0 {
            path.push("ffxiv");
        } else {
            path.push(format!("ex{}", self.expansion));
        }

        path.push(format!(
            "{:02x}{:02x}{:02x}",
            self.category, self.expansion, self.patch
        ));
        path
    }

    pub fn into_index2_path(&self) -> PathBuf {
        let mut path = self.into_repo_path();
        path.set_extension("win32.index2");
        path
    }

    pub fn into_dat_path(&self, num: u8) -> PathBuf {
        let mut path = self.into_repo_path();
        path.set_extension(format!("win32.dat{}", num));
        path
    }
}

impl Debug for PackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "PackId({:02x}{:02x}{:02x})",
            self.category, self.expansion, self.patch
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packid_from_path() {
        assert!(PackId::from_inner_path("foobar/ffixv/file").is_err());

        assert_eq!(
            PackId::from_inner_path("exd/test.exd").unwrap(),
            PackId::new(0x0a, 0, 0)
        );
        assert_eq!(
            PackId::from_inner_path("exd/ffxiv/test.exd").unwrap(),
            PackId::new(0x0a, 0, 0)
        );
        assert_eq!(
            PackId::from_inner_path("common/ex2/testdir/testfile").unwrap(),
            PackId::new(0, 2, 0)
        );
        assert_eq!(
            PackId::from_inner_path("sound/01_testfile").unwrap(),
            PackId::new(0x07, 0, 0)
        );
        assert_eq!(
            PackId::from_inner_path("sound/ex1/01_testfile").unwrap(),
            PackId::new(0x07, 1, 0x01)
        );
        assert_eq!(
            PackId::from_inner_path("sound/ex1/1f_testfile").unwrap(),
            PackId::new(0x07, 1, 0x1f)
        );
        assert_eq!(
            PackId::from_inner_path("common/dir/dir/file").unwrap(),
            PackId::new(0, 0, 0)
        );
    }
}
