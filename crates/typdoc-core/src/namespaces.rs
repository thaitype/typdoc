//! The namespaces of a project: the folders that `namespaces` in `config.json` names.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{CONFIG_FILE, Namespace, Report, TYPDOC_DIR, plain_name};
use crate::error::Error;
use crate::template::Segment;

/// The namespace `default`, or the folders that the entries name. A folder that an entry
/// matches and that cannot be a namespace is reported and never skipped.
pub(crate) fn resolve(
    root: &Path,
    entries: Option<&[String]>,
    report: &mut Report,
) -> Result<Vec<Namespace>, Error> {
    let Some(entries) = entries else {
        return Ok(vec![Namespace {
            name: "default".to_owned(),
            folder: String::new(),
        }]);
    };
    let mut matched: BTreeMap<String, PathBuf> = BTreeMap::new();
    if !entries.is_empty() {
        let listing = folders(root)?;
        for entry in entries {
            let found = entry_folders(entry, &listing, report)?;
            matched.extend(found);
        }
    }
    let mut namespaces = Vec::new();
    for (name, folder) in matched {
        let usable = usable_name(&name);
        if !usable {
            report.add(
                "config.namespace-name",
                CONFIG_FILE,
                format!(
                    "the folder `{name}` cannot be a namespace: a name uses only ASCII letters, digits, `-` and `_`, and `default` is reserved"
                ),
            );
        }
        let nested = folder.join(TYPDOC_DIR).symlink_metadata().is_ok();
        if nested {
            report.add(
                "config.namespace-nested",
                CONFIG_FILE,
                format!("the folder `{name}` holds its own `{TYPDOC_DIR}`: namespaces do not nest"),
            );
        }
        if usable && !nested {
            namespaces.push(Namespace {
                name: name.clone(),
                folder: name,
            });
        }
    }
    Ok(namespaces)
}

fn usable_name(name: &str) -> bool {
    name != "default" && plain_name(name)
}

/// A directory entry of the project folder: its name, its path, and whether it is a folder.
struct Folder {
    name: String,
    path: PathBuf,
    symlink: bool,
    folder: bool,
}

fn folders(root: &Path) -> Result<Vec<Folder>, Error> {
    let mut listing = Vec::new();
    for entry in fs::read_dir(root).map_err(Error::io_at(root))? {
        let entry = entry.map_err(Error::io_at(root))?;
        let kind = entry.file_type().map_err(Error::io_at(root))?;
        listing.push(Folder {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path(),
            symlink: kind.is_symlink(),
            folder: kind.is_dir(),
        });
    }
    Ok(listing)
}

/// The folders one entry names, by name. An entry that is not one segment, or a name that
/// names no folder, is reported. A folder whose name begins with `.` is reached by an entry of
/// plain text and by no wildcard, the same answer a collection's `match` gives.
fn entry_folders(
    entry: &str,
    listing: &[Folder],
    report: &mut Report,
) -> Result<Vec<(String, PathBuf)>, Error> {
    let refuse = |report: &mut Report, why: &str| {
        report.add(
            "config.namespaces-entry",
            CONFIG_FILE,
            format!("the entry `{entry}` of `namespaces` {why}"),
        );
        Ok(Vec::new())
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
    let mut found = Vec::new();
    for candidate in listing.iter().filter(|c| segment.matches_folder(&c.name)) {
        if candidate.symlink {
            return Err(Error::symbolic_link(&candidate.path));
        }
        if candidate.folder {
            found.push((candidate.name.clone(), candidate.path.clone()));
        }
    }
    if found.is_empty() && !entry.contains('*') {
        return refuse(report, "names a folder that does not exist");
    }
    Ok(found)
}
