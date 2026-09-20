use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Namespace, config_file};
use crate::error::Error;
use crate::template::{Step, Template};

/// A collection as the index reads it.
pub struct Member {
    pub name: String,
    pub template: Template,
    /// The collection file, for messages.
    pub file: PathBuf,
}

#[derive(Debug)]
pub struct Entry {
    /// The position in the slice of members the index was built from.
    pub collection: usize,
    /// The position in the slice of namespaces the index was built from.
    pub namespace: usize,
    pub file: PathBuf,
}

/// Every document of the project by its path from the project folder, as the names are on disk.
#[derive(Debug, Default)]
pub struct Index {
    entries: BTreeMap<String, Entry>,
}

impl Index {
    pub fn get(&self, path: &str) -> Option<&Entry> {
        self.entries.get(path)
    }

    /// Walks each namespace folder once for each collection, following only the folders its
    /// template names.
    pub fn build(
        root: &Path,
        namespaces: &[Namespace],
        members: &[Member],
    ) -> Result<Index, Error> {
        let mut index = Index::default();
        for (namespace, space) in namespaces.iter().enumerate() {
            let base = root.join(&space.folder);
            for (collection, member) in members.iter().enumerate() {
                let mut found = BTreeMap::new();
                walk(&base, "", member.template.steps(), &mut found)?;
                for (below, file) in found {
                    let path = if space.folder.is_empty() {
                        below
                    } else {
                        format!("{}/{below}", space.folder)
                    };
                    if let Some(earlier) = index.entries.get(&path) {
                        return Err(Error::Config {
                            file: member.file.clone(),
                            message: format!(
                                "{} is also matched by the collection `{}`",
                                file.display(),
                                members[earlier.collection].name
                            ),
                        });
                    }
                    index.entries.insert(
                        path,
                        Entry {
                            collection,
                            namespace,
                            file,
                        },
                    );
                }
            }
        }
        Ok(index)
    }
}

/// An entry of a folder, as `read_dir` gives it.
struct Listed {
    name: String,
    utf8: bool,
    path: PathBuf,
    symlink: bool,
    folder: bool,
    file: bool,
}

fn list(dir: &Path) -> Result<Vec<Listed>, Error> {
    let mut listed = Vec::new();
    for entry in fs::read_dir(dir).map_err(Error::io_at(dir))? {
        let entry = entry.map_err(Error::io_at(dir))?;
        let kind = entry.file_type().map_err(Error::io_at(dir))?;
        let name = entry.file_name();
        listed.push(Listed {
            utf8: name.to_str().is_some(),
            name: name.to_string_lossy().into_owned(),
            path: entry.path(),
            symlink: kind.is_symlink(),
            folder: kind.is_dir(),
            file: kind.is_file(),
        });
    }
    Ok(listed)
}

/// Adds the files below `dir` that the rest of a template matches, by their path below the
/// namespace folder. What a run reads is decided here: a wildcard never matches a name that
/// starts with `.`, a folder that holds its own project is not entered, and a symbolic link
/// that a template reaches or a name that is not UTF-8 stops the run rather than being skipped.
fn walk(
    dir: &Path,
    prefix: &str,
    steps: &[Step],
    found: &mut BTreeMap<String, PathBuf>,
) -> Result<(), Error> {
    let Some((step, rest)) = steps.split_first() else {
        return Ok(());
    };
    let listed = list(dir)?;
    match step {
        Step::Name(segment) => {
            for entry in listed.iter().filter(|entry| segment.matches(&entry.name)) {
                if rest.is_empty() {
                    take(entry, prefix, found)?;
                } else {
                    enter(entry, prefix, rest, found)?;
                }
            }
        }
        Step::Folders => {
            if !rest.is_empty() {
                walk(dir, prefix, rest, found)?;
            }
            for entry in listed.iter().filter(|entry| !entry.name.starts_with('.')) {
                if entry.folder || is_link_to_folder(entry) {
                    enter(entry, prefix, steps, found)?;
                } else if rest.is_empty() {
                    take(entry, prefix, found)?;
                }
            }
        }
    }
    Ok(())
}

fn is_link_to_folder(entry: &Listed) -> bool {
    entry.symlink && fs::metadata(&entry.path).is_ok_and(|meta| meta.is_dir())
}

fn below(prefix: &str, entry: &Listed) -> Result<String, Error> {
    if !entry.utf8 {
        return Err(Error::Unreadable {
            file: entry.path.clone(),
            message: "the name is not valid UTF-8, so no path can name it".to_owned(),
        });
    }
    Ok(if prefix.is_empty() {
        entry.name.clone()
    } else {
        format!("{prefix}/{}", entry.name)
    })
}

/// A file that the last step of a template matched. A folder is not a document.
fn take(entry: &Listed, prefix: &str, found: &mut BTreeMap<String, PathBuf>) -> Result<(), Error> {
    if entry.symlink {
        return Err(Error::symbolic_link(&entry.path));
    }
    if entry.file {
        found.insert(below(prefix, entry)?, entry.path.clone());
    }
    Ok(())
}

/// A folder that a step before the last matched.
fn enter(
    entry: &Listed,
    prefix: &str,
    rest: &[Step],
    found: &mut BTreeMap<String, PathBuf>,
) -> Result<(), Error> {
    if entry.symlink {
        return Err(Error::symbolic_link(&entry.path));
    }
    if !entry.folder || config_file(&entry.path).is_file() {
        return Ok(());
    }
    walk(&entry.path, &below(prefix, entry)?, rest, found)
}
