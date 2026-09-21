//! The namespaces of a project: the folders that `namespaces` in `config.json` names.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{
    CONFIG_FILE, NAME_NOT_UTF8, Namespace, Report, SYMBOLIC_LINK, Skipped, TYPDOC_DIR, plain_name,
};
use crate::error::Error;
use crate::schema::reserved_url_scheme;
use crate::template::Segment;

/// What the entries of `namespaces` came to: the namespaces, and the entries a glob reached and
/// skipped.
pub(crate) struct Resolved {
    pub namespaces: Vec<Namespace>,
    pub skipped: Vec<Skipped>,
}

/// The namespace `default`, or the folders that the entries name. A folder that an entry
/// matches and that cannot be a namespace is reported and never skipped; an entry a glob
/// reaches that is a symbolic link or whose name is not valid UTF-8 is skipped and recorded.
pub(crate) fn resolve(
    root: &Path,
    entries: Option<&[String]>,
    report: &mut Report,
) -> Result<Resolved, Error> {
    let Some(entries) = entries else {
        return Ok(Resolved {
            namespaces: vec![Namespace {
                name: "default".to_owned(),
                folder: String::new(),
            }],
            skipped: Vec::new(),
        });
    };
    let mut matched: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut skipped: BTreeSet<Skipped> = BTreeSet::new();
    if !entries.is_empty() {
        let listing = folders(root)?;
        for entry in entries {
            let found = entry_folders(entry, &listing, report);
            matched.extend(found.folders);
            skipped.extend(found.skipped);
        }
    }
    let mut namespaces = Vec::new();
    for (name, folder) in matched {
        let problem = name_problem(&name);
        if let Some(message) = &problem {
            report.add("config.namespace-name", CONFIG_FILE, message.clone());
        }
        let nested = folder.join(TYPDOC_DIR).symlink_metadata().is_ok();
        if nested {
            report.add(
                "config.namespace-nested",
                CONFIG_FILE,
                format!("the folder `{name}` holds its own `{TYPDOC_DIR}`: namespaces do not nest"),
            );
        }
        if problem.is_none() && !nested {
            namespaces.push(Namespace {
                name: name.clone(),
                folder: name,
            });
        }
    }
    Ok(Resolved {
        namespaces,
        skipped: skipped.into_iter().collect(),
    })
}

/// Why a folder cannot be a namespace, as the message of `config.namespace-name`.
fn name_problem(name: &str) -> Option<String> {
    if reserved_url_scheme(name) {
        return Some(format!(
            "the folder `{name}` cannot be a namespace: it is a URL scheme (`http`, `https`, `mailto` and `file` are reserved), and a link that starts with `{name}:` would be told apart wrongly"
        ));
    }
    if name == "default" || !plain_name(name) {
        return Some(format!(
            "the folder `{name}` cannot be a namespace: a name uses only ASCII letters, digits, `-` and `_`, and `default` is reserved"
        ));
    }
    None
}

/// A directory entry of the project folder: its name, its path, and whether it is a folder.
struct Folder {
    name: String,
    name_is_utf8: bool,
    path: PathBuf,
    symlink: bool,
    folder: bool,
}

fn folders(root: &Path) -> Result<Vec<Folder>, Error> {
    let mut listing = Vec::new();
    for entry in fs::read_dir(root).map_err(Error::io_at(root))? {
        let entry = entry.map_err(Error::io_at(root))?;
        let kind = entry.file_type().map_err(Error::io_at(root))?;
        let name = entry.file_name();
        listing.push(Folder {
            name_is_utf8: name.to_str().is_some(),
            name: name.to_string_lossy().into_owned(),
            path: entry.path(),
            symlink: kind.is_symlink(),
            folder: kind.is_dir(),
        });
    }
    Ok(listing)
}

/// Whether a symbolic link points at a folder. A link to a file and a link to nothing are not
/// folders, so a glob leaves them alone as it leaves a regular file.
fn links_to_folder(link: &Path) -> bool {
    fs::metadata(link).is_ok_and(|meta| meta.is_dir())
}

/// What one entry reached: the folders it names, by name, and the entries a glob skipped.
#[derive(Default)]
struct Reached {
    folders: Vec<(String, PathBuf)>,
    skipped: Vec<Skipped>,
}

/// The folders one entry names, by name. An entry that is not one segment, or a name that
/// names no folder, is reported. A folder whose name begins with `.` is reached by an entry of
/// plain text and by no wildcard, the same answer a collection's `match` gives. A link that an
/// entry of plain text names is refused; a link to a folder that a glob reaches is skipped.
fn entry_folders(entry: &str, listing: &[Folder], report: &mut Report) -> Reached {
    let refuse = |report: &mut Report, why: &str| {
        report.add(
            "config.namespaces-entry",
            CONFIG_FILE,
            format!("the entry `{entry}` of `namespaces` {why}"),
        );
        Reached::default()
    };
    if entry.contains('/') || entry.contains("**") {
        return refuse(
            report,
            "is more than one path segment: `/` and `**` are not allowed",
        );
    }
    let segment = match Segment::parse_glob(entry) {
        Ok(segment) => segment,
        Err(e) => return refuse(report, &format!("cannot be read: {e}")),
    };
    let glob = entry.contains('*');
    let mut reached = Reached::default();
    for candidate in listing.iter().filter(|c| segment.matches_folder(&c.name)) {
        if candidate.symlink {
            if !glob {
                return refuse(
                    report,
                    "names a symbolic link, which a run does not follow: name the folder the link points to",
                );
            }
            if links_to_folder(&candidate.path) {
                reached.skipped.push(Skipped {
                    path: candidate.name.clone(),
                    why: SYMBOLIC_LINK,
                });
            }
        } else if candidate.folder && !candidate.name_is_utf8 {
            reached.skipped.push(Skipped {
                path: candidate.name.clone(),
                why: NAME_NOT_UTF8,
            });
        } else if candidate.folder {
            reached
                .folders
                .push((candidate.name.clone(), candidate.path.clone()));
        }
    }
    if reached.folders.is_empty() && !glob {
        return refuse(report, "names a folder that does not exist");
    }
    reached
}
