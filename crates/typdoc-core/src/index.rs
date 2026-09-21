use std::collections::btree_map::Entry as MapEntry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Namespace, config_file};
use crate::error::Error;
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
    /// The key this document carries, for a coded collection only.
    pub key: Option<String>,
}

/// Every document of the project by its path from the project folder, as the names are on disk,
/// and every coded document again by its key within its namespace.
#[derive(Debug, Default)]
pub struct Index {
    entries: BTreeMap<String, Entry>,
    /// Every path a key was bound to, in the namespace it was found in, in the order the walk
    /// found them. More than one path for one `(namespace, key)` is `keys.unique`'s finding;
    /// `key` keeps returning the first so `get` by key still resolves to one document.
    keys: BTreeMap<(usize, String), Vec<String>>,
    /// A path matched by more than one collection: its namespace and the name of every
    /// collection that matched it, in the order the walk found them. The design settles this by
    /// no precedence at all, so such a path is removed from `entries` once found (below) — there
    /// is no one collection to answer `get` or `toc` with, and `collections.overlap`'s finding
    /// is built from this map instead of from an entry.
    overlaps: BTreeMap<String, (usize, Vec<String>)>,
    /// Every directory entry a `match` reached and the walk could not read: its path from the
    /// project folder, its namespace and why. `files.unreadable`'s finding is built from this,
    /// and the entry itself is in none of the maps above, since nothing was read from it. The
    /// same entry is reached once per collection whose template covers it and is recorded once.
    unreadable: BTreeMap<String, (usize, &'static str)>,
}

impl Index {
    pub fn get(&self, path: &str) -> Option<&Entry> {
        self.entries.get(path)
    }

    /// Every document of the project, by its path, in no particular order (the caller sorts
    /// what it needs sorted). Never includes a path matched by more than one collection.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Entry)> {
        self.entries
            .iter()
            .map(|(path, entry)| (path.as_str(), entry))
    }

    /// The path of the document that carries `key` in the namespace at `namespace`: the first
    /// found, when more than one does.
    pub fn key(&self, namespace: usize, key: &str) -> Option<&str> {
        self.keys
            .get(&(namespace, key.to_owned()))
            .and_then(|paths| paths.first())
            .map(String::as_str)
    }

    /// Every path that carries `key` in the namespace at `namespace`, in the order found: more
    /// than one is `keys.unique`'s finding.
    pub fn key_group(&self, namespace: usize, key: &str) -> Option<&[String]> {
        self.keys
            .get(&(namespace, key.to_owned()))
            .map(Vec::as_slice)
    }

    /// The namespace and the name of every collection that matched `path`, when more than one
    /// did: `resolve` reads the collections to refuse a direct read of such a path (there is no
    /// one collection to read it with); `validate`'s argument scope reads both, to report
    /// `collections.overlap` for it without a document to check.
    pub fn overlap(&self, path: &str) -> Option<(usize, &[String])> {
        self.overlaps
            .get(path)
            .map(|(namespace, names)| (*namespace, names.as_slice()))
    }

    /// Every overlapping path found, with its namespace and the collections that matched it, for
    /// `validate`'s whole-project scan to turn into `collections.overlap` findings (such a path
    /// is not in `entries`, so the ordinary per-document walk never reaches it).
    pub fn overlaps(&self) -> impl Iterator<Item = (&str, usize, &[String])> {
        self.overlaps
            .iter()
            .map(|(path, (namespace, names))| (path.as_str(), *namespace, names.as_slice()))
    }

    /// Every entry a `match` reached and the walk could not read, by path, with its namespace
    /// and the reason, for `files.unreadable`'s findings.
    pub fn unreadable(&self) -> impl Iterator<Item = (&str, usize, &str)> {
        self.unreadable
            .iter()
            .map(|(path, (namespace, why))| (path.as_str(), *namespace, *why))
    }

    /// Walks each namespace folder once for each collection, following only the folders its
    /// template names. A file matched by more than one collection is never settled by
    /// precedence, as the design asks: it is recorded in `overlaps` and, once every collection
    /// has been walked, removed from `entries` (and from its key's group, if it carried one), so
    /// there is no collection left to answer a direct read of it with.
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

/// `filename.pattern`'s candidates: the path (relative to the project folder) and namespace name
/// of every file directly inside a coded collection's folder that fits no collection's `match`
/// there. Several coded collections can share one folder (the design's own example, `WF` and
/// `RFC` both under `tickets/{key}.md`), so a file is a candidate only when it fits none of the
/// collections that share the folder. Left unchecked, by decision: a coded template whose
/// `{key}` is not the last step (`{key}/index.md`) has no one folder to scan, and generalising
/// to that shape is not done here (`Template::key_in_last_step`).
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

/// Every `.md` file below a namespace folder that a run reads, by its path from the project
/// folder and the namespace it belongs to: for `validate --audit`'s `uncollected` list, which
/// asks which files no collection covers, and so cannot be built from what a collection's own
/// `match` template already names — it has to be an independent walk of the folder itself. It
/// answers "which files a run reads" the same way the per-collection `walk` below does, so the
/// two read one way everywhere in this crate: a folder whose name begins with `.` is entered
/// only where a `match` writes that name out as plain text (`literal_folder_names`, gathered
/// from `members` here), a file whose name begins with `.` is listed like any other, a symbolic
/// link is neither
/// followed nor listed, a name that is not valid UTF-8 is skipped, and a folder that holds its
/// own `.typdoc/config.json` is a separate project and is never entered. A symbolic link and a
/// name that is not UTF-8 are skipped silently here and reported only where a `match` reaches
/// them (`files.unreadable`, whose row in the design's table is about an entry a `match`
/// reaches), so this walk adds no finding of its own.
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

/// The namespace `walk_every_file` is walking: what stays the same all the way down the
/// recursion, so that only the folder and its path below the namespace change from call to call.
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
        } else if entry.file && entry.name.ends_with(".md") {
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

/// What one walk of one template gathers: the files its last step matched, by their path below
/// the namespace folder, and the entries it reached and could not read, by the same path.
#[derive(Default)]
struct Found {
    files: BTreeMap<String, PathBuf>,
    unreadable: BTreeMap<String, &'static str>,
}

const SYMBOLIC_LINK: &str = "a symbolic link is not read: a run does not follow one out of the project, or read one file \
     twice under two names";

const NAME_NOT_UTF8: &str = "the name is not valid UTF-8, so no path can name it; it is written here \
                        with a replacement character for each byte that cannot be read";

/// Adds the files below `dir` that the rest of a template matches, by their path below the
/// namespace folder. What a run reads is decided here: a folder whose name begins with `.` is
/// entered by a plain-text segment and by no wildcard, a `*` in the last step matches a leading
/// dot in a file name, a folder that holds its own project is not entered, and an entry the
/// template reaches that is a symbolic link or whose name is not valid UTF-8 is skipped and
/// recorded for `files.unreadable` rather than stopping the run.
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

/// The path of `entry` below the namespace folder. A name that is not valid UTF-8 is written
/// with replacement characters, which is what `Listed::name` already holds: such a path only
/// ever names an entry in a `files.unreadable` finding, and nothing is opened by it.
fn below(prefix: &str, entry: &Listed) -> String {
    if prefix.is_empty() {
        entry.name.clone()
    } else {
        format!("{prefix}/{}", entry.name)
    }
}

/// Records an entry the template reached and the walk is not reading, with the reason.
fn skip(entry: &Listed, prefix: &str, why: &'static str, found: &mut Found) {
    found.unreadable.insert(below(prefix, entry), why);
}

/// A file that the last step of a template matched. A folder is not a document.
fn take(entry: &Listed, prefix: &str, found: &mut Found) {
    if entry.symlink {
        skip(entry, prefix, SYMBOLIC_LINK, found);
    } else if !entry.utf8 {
        skip(entry, prefix, NAME_NOT_UTF8, found);
    } else if entry.file {
        found.files.insert(below(prefix, entry), entry.path.clone());
    }
}

/// A folder that a step before the last matched. It asks the two reasons in a different order
/// from `take`, and the check between them is why: a matched entry that is not a folder at all
/// is left alone rather than reported, since nothing would have been read from it either way.
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
