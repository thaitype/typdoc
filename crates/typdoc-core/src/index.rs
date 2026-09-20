use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Collection;
use crate::error::Error;
use crate::glob;

#[derive(Debug)]
pub struct Entry {
    /// The position in the slice the index was built from.
    pub collection: usize,
    pub file: PathBuf,
}

/// Every document of the namespace by its path, as the names are on disk.
#[derive(Debug, Default)]
pub struct Index {
    entries: BTreeMap<String, Entry>,
}

impl Index {
    pub fn get(&self, path: &str) -> Option<&Entry> {
        self.entries.get(path)
    }

    /// Lists the folder itself, without entering any folder inside it.
    pub fn build(folder: &Path, collections: &[Collection]) -> Result<Index, Error> {
        for collection in collections {
            collection.check_pattern()?;
        }
        let mut index = Index::default();
        for entry in fs::read_dir(folder).map_err(Error::io_at(folder))? {
            let entry = entry.map_err(Error::io_at(folder))?;
            let file = entry.path();
            let name = entry.file_name();
            let lossy = name.to_string_lossy();
            let matched: Vec<usize> = (0..collections.len())
                .filter(|&i| glob::matches(&collections[i].pattern, &lossy))
                .collect();
            if matched.is_empty() {
                continue;
            }
            let Some(path) = name.to_str() else {
                return Err(Error::Unreadable {
                    file,
                    message: "the name is not valid UTF-8, so no path can name it".to_owned(),
                });
            };
            let kind = entry.file_type().map_err(Error::io_at(folder))?;
            if kind.is_symlink() {
                return Err(Error::Unreadable {
                    file,
                    message:
                        "a symbolic link is not read: whether a run follows one is not decided"
                            .to_owned(),
                });
            }
            if !kind.is_file() {
                continue;
            }
            if let [first, second, ..] = matched.as_slice() {
                return Err(Error::Config {
                    file: collections[*second].file.clone(),
                    message: format!(
                        "{} is also matched by the collection `{}`",
                        file.display(),
                        collections[*first].name
                    ),
                });
            }
            index.entries.insert(
                path.to_owned(),
                Entry {
                    collection: matched[0],
                    file,
                },
            );
        }
        Ok(index)
    }
}
