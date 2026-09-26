use std::collections::btree_map::Entry as MapEntry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{LEFTOVER_TEMP_FILE, NAME_NOT_UTF8, Namespace, SYMBOLIC_LINK, config_file};
use crate::error::Error;
use crate::fs::is_temp_name;
use crate::template::{Segment, Step, Template};

/// A collection as the index reads it.
pub struct Member {
    pub name: String,
    pub template: Template,
}

#[derive(Debug)]
pub struct Entry {
    /// The position in the slice of members the index was built from.
    pub collection: usize,
    /// The position in the slice of namespaces the index was built from.
    pub namespace: usize,
    pub file: PathBuf,
    pub key: Option<String>,
}

/// Every document of the project by its path from the project folder, as the names are on disk,
/// and every coded document again by its key within its namespace.
#[derive(Debug, Default)]
pub struct Index {
    entries: BTreeMap<String, Entry>,
    /// Every path a key was bound to, in walk order. More than one is `keys.unique`'s finding;
    /// `key` returns the first, so `get` by key still resolves to one document.
    keys: BTreeMap<(usize, String), Vec<String>>,
    /// A path matched by more than one collection: its namespace and every collection that
    /// matched it, in walk order.
    overlaps: BTreeMap<String, (usize, Vec<String>)>,
    /// Directory entries a `match` reached and the walk could not read, for `files.unreadable`.
    /// Such an entry is in no other map, and is recorded once however many templates reach it.
    unreadable: BTreeMap<String, (usize, &'static str)>,
}

impl Index {
    pub fn get(&self, path: &str) -> Option<&Entry> {
        self.entries.get(path)
    }

    /// Never includes a path matched by more than one collection.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Entry)> {
        self.entries
            .iter()
            .map(|(path, entry)| (path.as_str(), entry))
    }

    /// The first found, when more than one document carries `key`.
    pub fn key(&self, namespace: usize, key: &str) -> Option<&str> {
        self.keys
            .get(&(namespace, key.to_owned()))
            .and_then(|paths| paths.first())
            .map(String::as_str)
    }

    pub fn key_group(&self, namespace: usize, key: &str) -> Option<&[String]> {
        self.keys
            .get(&(namespace, key.to_owned()))
            .map(Vec::as_slice)
    }

    pub fn overlap(&self, path: &str) -> Option<(usize, &[String])> {
        self.overlaps
            .get(path)
            .map(|(namespace, names)| (*namespace, names.as_slice()))
    }

    /// An overlapping path is not in `entries`, so `validate` reports `collections.overlap` from
    /// this.
    pub fn overlaps(&self) -> impl Iterator<Item = (&str, usize, &[String])> {
        self.overlaps
            .iter()
            .map(|(path, (namespace, names))| (path.as_str(), *namespace, names.as_slice()))
    }

    pub fn unreadable(&self) -> impl Iterator<Item = (&str, usize, &str)> {
        self.unreadable
            .iter()
            .map(|(path, (namespace, why))| (path.as_str(), *namespace, *why))
    }

    /// Walks each namespace folder once for each collection, following only the folders its
    /// template names. A file matched by more than one collection is never settled by precedence
    /// (SPC-17): it goes to `overlaps` and is removed from `entries` and its key's group, so no
    /// collection answers a direct read of it.
    pub fn build(
        root: &Path,
        namespaces: &[Namespace],
        members: &[Member],
    ) -> Result<Index, Error> {
        let mut index = Index::default();
        for (namespace, space) in namespaces.iter().enumerate() {
            let base = root.join(&space.folder);
            for (collection, member) in members.iter().enumerate() {
                let mut found = Found::default();
                walk(&base, "", member.template.steps(), &mut found)?;
                let at = |below: &str| {
                    if space.folder.is_empty() {
                        below.to_owned()
                    } else {
                        format!("{}/{below}", space.folder)
                    }
                };
                for (below, why) in found.unreadable {
                    index.unreadable.insert(at(&below), (namespace, why));
                }
                for (below, file) in found.files {
                    let path = at(&below);
                    match index.entries.entry(path.clone()) {
                        MapEntry::Occupied(existing) => {
                            let ns = existing.get().namespace;
                            let first = members[existing.get().collection].name.clone();
                            let (_, names) = index
                                .overlaps
                                .entry(path)
                                .or_insert_with(|| (ns, vec![first]));
                            if !names.contains(&member.name) {
                                names.push(member.name.clone());
                            }
                        }
                        MapEntry::Vacant(slot) => {
                            let key = member.template.key(&below);
                            if let Some(key) = &key {
                                index
                                    .keys
                                    .entry((namespace, key.clone()))
                                    .or_default()
                                    .push(path.clone());
                            }
                            slot.insert(Entry {
                                collection,
                                namespace,
                                file,
                                key,
                            });
                        }
                    }
                }
            }
        }
        for (path, (namespace, _)) in &index.overlaps {
            let Some(entry) = index.entries.remove(path) else {
                continue;
            };
            if let Some(key) = &entry.key
                && let Some(group) = index.keys.get_mut(&(*namespace, key.clone()))
            {
                group.retain(|other| other != path);
                if group.is_empty() {
                    index.keys.remove(&(*namespace, key.clone()));
                }
            }
        }
        Ok(index)
    }
}

/// `filename.pattern`'s candidates: every file directly inside a coded collection's folder that
/// fits no collection's `match` there. Coded collections can share a folder (SPC-17), so a file
/// is a candidate only when it fits none of them. A template whose `{key}` is not the last step
/// (`{key}/index.md`) has no one folder to scan and is not checked.
pub(crate) fn stray_files(
    root: &Path,
    namespaces: &[Namespace],
    members: &[Member],
) -> Result<Vec<(String, String)>, Error> {
    let mut found = Vec::new();
    for space in namespaces {
        let base = root.join(&space.folder);
        let mut groups: BTreeMap<Vec<String>, Vec<&Segment>> = BTreeMap::new();
        for member in members {
            if !member.template.key_in_last_step() {
                continue;
            }
            let (Some(prefix), Some(segment)) = (
                member.template.literal_folder(),
                member.template.last_segment(),
            ) else {
                continue;
            };
            groups
                .entry(prefix.iter().map(|part| (*part).to_owned()).collect())
                .or_default()
                .push(segment);
        }
        for (prefix, segments) in groups {
            let dir = prefix.iter().fold(base.clone(), |dir, part| dir.join(part));
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => return Err(Error::Io { file: dir, source }),
            };
            for entry in entries {
                let entry = entry.map_err(Error::io_at(&dir))?;
                let kind = entry.file_type().map_err(Error::io_at(&dir))?;
                if !kind.is_file() {
                    continue;
                }
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                // A leftover is not a stray file: the user did not write it (SPC-10).
                if is_temp_name(&name) {
                    continue;
                }
                // A file name that begins with `.` is not passed over: the leading-dot rule is
                // about the folders a walk enters, and a `*` in a template matches such a name.
                if segments.iter().any(|segment| segment.matches(&name)) {
                    continue;
                }
                let below = if prefix.is_empty() {
                    name
                } else {
                    format!("{}/{name}", prefix.join("/"))
                };
                let path = if space.folder.is_empty() {
                    below
                } else {
                    format!("{}/{below}", space.folder)
                };
                found.push((path, space.name.clone()));
            }
        }
    }
    Ok(found)
}

/// Every `.md` file a run reads below a namespace folder, for `validate --audit`'s `uncollected`
/// list. That list asks which files no collection covers, so it cannot come from the templates:
/// it walks the folders itself, by the same rules as `walk`. A symbolic link, a name that is not
/// valid UTF-8 and a leftover temp file are skipped silently: they are reported only where a
/// `match` reaches them, so this walk adds no finding.
pub(crate) fn all_markdown_files(
    root: &Path,
    namespaces: &[Namespace],
    members: &[Member],
) -> Result<Vec<(String, usize)>, Error> {
    let named: BTreeSet<&str> = members
        .iter()
        .flat_map(|member| member.template.literal_folder_names())
        .collect();
    let mut found = Vec::new();
    for (index, space) in namespaces.iter().enumerate() {
        let base = root.join(&space.folder);
        let namespace = Walked {
            folder: &space.folder,
            index,
            named: &named,
        };
        walk_every_file(&base, "", &namespace, &mut found)?;
    }
    Ok(found)
}

struct Walked<'a> {
    /// The namespace folder, as a path from the project folder; empty for `default`.
    folder: &'a str,
    /// The position of the namespace in the slice the walk was given.
    index: usize,
    /// The name of every folder a `match` writes out as plain text. A folder whose name begins
    /// with `.` is entered exactly when its name is one of these: this walk stands at no
    /// position in any template, so it asks by name, which also keeps it from missing a folder
    /// a template names after a wildcard (`**/.agents/*.md`).
    named: &'a BTreeSet<&'a str>,
}

fn walk_every_file(
    dir: &Path,
    prefix: &str,
    namespace: &Walked,
    found: &mut Vec<(String, usize)>,
) -> Result<(), Error> {
    for entry in list(dir)? {
        if entry.symlink || !entry.utf8 {
            continue;
        }
        let here = below(prefix, &entry);
        if entry.folder {
            if (entry.name.starts_with('.') && !namespace.named.contains(entry.name.as_str()))
                || config_file(&entry.path).is_file()
            {
                continue;
            }
            walk_every_file(&entry.path, &here, namespace, found)?;
        } else if entry.file && entry.name.ends_with(".md") && !is_temp_name(&entry.name) {
            let path = if namespace.folder.is_empty() {
                here
            } else {
                format!("{}/{here}", namespace.folder)
            };
            found.push((path, namespace.index));
        }
    }
    Ok(())
}

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

/// What one walk of one template gathers, by path below the namespace folder.
#[derive(Default)]
struct Found {
    files: BTreeMap<String, PathBuf>,
    unreadable: BTreeMap<String, &'static str>,
}

/// Adds the files below `dir` that the rest of a template matches. Which files a run reads is
/// decided here (SPC-17).
fn walk(dir: &Path, prefix: &str, steps: &[Step], found: &mut Found) -> Result<(), Error> {
    let Some((step, rest)) = steps.split_first() else {
        return Ok(());
    };
    let listed = list(dir)?;
    match step {
        Step::Name(segment) => {
            for entry in &listed {
                if rest.is_empty() {
                    if segment.matches(&entry.name) {
                        take(entry, prefix, found);
                    }
                } else if segment.matches_folder(&entry.name) {
                    enter(entry, prefix, rest, found)?;
                }
            }
        }
        Step::Folders => {
            if !rest.is_empty() {
                walk(dir, prefix, rest, found)?;
            }
            for entry in &listed {
                if entry.folder || is_link_to_folder(entry) {
                    // `**` is a wildcard, so it enters no folder whose name begins with `.`.
                    if !entry.name.starts_with('.') {
                        enter(entry, prefix, steps, found)?;
                    }
                } else if rest.is_empty() {
                    take(entry, prefix, found);
                }
            }
        }
    }
    Ok(())
}

fn is_link_to_folder(entry: &Listed) -> bool {
    entry.symlink && fs::metadata(&entry.path).is_ok_and(|meta| meta.is_dir())
}

/// A name that is not valid UTF-8 holds replacement characters here: such a path only names an
/// entry in a `files.unreadable` finding, and nothing is opened by it.
fn below(prefix: &str, entry: &Listed) -> String {
    if prefix.is_empty() {
        entry.name.clone()
    } else {
        format!("{prefix}/{}", entry.name)
    }
}

fn skip(entry: &Listed, prefix: &str, why: &'static str, found: &mut Found) {
    found.unreadable.insert(below(prefix, entry), why);
}

/// The temp-file shape is checked first, whatever the step's glob matched: `*` matches a leading
/// dot, and a leftover is never a document (SPC-10).
fn take(entry: &Listed, prefix: &str, found: &mut Found) {
    if entry.file && is_temp_name(&entry.name) {
        skip(entry, prefix, LEFTOVER_TEMP_FILE, found);
    } else if entry.symlink {
        skip(entry, prefix, SYMBOLIC_LINK, found);
    } else if !entry.utf8 {
        skip(entry, prefix, NAME_NOT_UTF8, found);
    } else if entry.file {
        found.files.insert(below(prefix, entry), entry.path.clone());
    }
}

/// A matched entry that is not a folder is left alone rather than reported: nothing would have
/// been read from it either way.
fn enter(entry: &Listed, prefix: &str, rest: &[Step], found: &mut Found) -> Result<(), Error> {
    if entry.symlink {
        skip(entry, prefix, SYMBOLIC_LINK, found);
        return Ok(());
    }
    if !entry.folder || config_file(&entry.path).is_file() {
        return Ok(());
    }
    if !entry.utf8 {
        skip(entry, prefix, NAME_NOT_UTF8, found);
        return Ok(());
    }
    walk(&entry.path, &below(prefix, entry), rest, found)
}
