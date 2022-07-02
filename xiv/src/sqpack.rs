use crate::{dat::InnerFilePtr, error::XivError, index2::Index2, packid::PackId};
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug)]
pub struct SqPack {
    base_path: PathBuf,
    indexes: HashMap<PackId, OnceCell<Arc<Index2>>>,
}

impl SqPack {
    pub fn open(base_path: impl AsRef<Path>) -> Result<Arc<Self>, XivError> {
        let base_path = base_path.as_ref().to_owned();
        let mut indexes = HashMap::new();

        for repo_entry in std::fs::read_dir(&base_path).map_err(XivError::IO)? {
            let repo_entry = repo_entry.map_err(XivError::IO)?;
            if repo_entry.file_type().map_err(XivError::IO)?.is_dir() {
                for exp_entry in std::fs::read_dir(repo_entry.path()).map_err(XivError::IO)? {
                    let exp_entry = exp_entry.map_err(XivError::IO)?;
                    let file_name = exp_entry
                        .file_name()
                        .into_string()
                        .map_err(|_| XivError::PackIdRepoFile)?;
                    if file_name.ends_with(".index2") {
                        if let Ok(packid) = PackId::from_repo_path(file_name) {
                            indexes.insert(packid, OnceCell::new());
                        }
                    }
                }
            }
        }

        Ok(Arc::new(Self { base_path, indexes }))
    }

    fn index_for(&self, packid: PackId) -> Result<Option<Arc<Index2>>, XivError> {
        self.indexes
            .get(&packid)
            .map(|cell| {
                cell.get_or_try_init(|| {
                    let index = Index2::load(&self.base_path.join(packid.into_index2_path()))?;
                    Ok(Arc::new(index))
                })
                .cloned()
            })
            .transpose()
    }

    pub fn find(&self, path: &str) -> Result<Option<InnerFilePtr>, XivError> {
        let packid = PackId::from_inner_path(path)?;
        let index = self.index_for(packid)?;

        Ok(index.and_then(|index| {
            index.find(path).map(|entry| InnerFilePtr {
                path: self.base_path.join(packid.into_dat_path(entry.datnum)),
                offset: entry.offset,
            })
        }))
    }
}
