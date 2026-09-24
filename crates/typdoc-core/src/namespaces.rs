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
            let (negate, pattern) = match entry.strip_prefix('!') {
                Some(rest) => (true, rest),
                None => (false, entry.as_str()),
            };
            let found = entry_folders(entry, pattern, negate, &listing, report);
            if negate {
                for (name, _) in &found.folders {
                    matched.remove(name);
                }
            } else {
                matched.extend(found.folders);
            }
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
///
/// `display` is the full original config entry (`!story-9`, not `story-9`), used only for the
/// wording of a report — matching itself runs against `pattern`, the entry with any leading `!`
/// already stripped by the caller. `negate` forks only the empty-match report at the bottom: a
/// `!` entry that names no folder is always silent, exact name or glob alike, unlike a plain
/// entry naming an exact name that matches nothing, which is still reported.
fn entry_folders(
    display: &str,
    pattern: &str,
    negate: bool,
    listing: &[Folder],
    report: &mut Report,
) -> Reached {
    let refuse = |report: &mut Report, why: &str| {
        report.add(
            "config.namespaces-entry",
            CONFIG_FILE,
            format!("the entry `{display}` of `namespaces` {why}"),
        );
        Reached::default()
    };
    if pattern.contains('/') || pattern.contains("**") {
        return refuse(
            report,
            "is more than one path segment: `/` and `**` are not allowed",
        );
    }
    let segment = match Segment::parse_glob(pattern) {
        Ok(segment) => segment,
        Err(e) => return refuse(report, &format!("cannot be read: {e}")),
    };
    let glob = pattern.contains('*');
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
    if reached.folders.is_empty() && !glob && !negate {
        return refuse(report, "names a folder that does not exist");
    }
    reached
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::Report;

    /// The checked-in `fixtures/valid/namespace-exclusion/` project root: real `story-1`,
    /// `story-2` and `story-3` folders on disk, so `resolve` has something real to
    /// `fs::read_dir`. Reused rather than a folder created for the test at run time: this crate
    /// changes nothing directly, not even in a test (`clippy.toml`'s own `disallowed-methods`
    /// list has no exception for one), and a folder made through the injected `Fs` seam cannot
    /// be reached from a unit test inside this crate's own build in the first place — the seam's
    /// real implementation, `typdoc_fs::SystemFs`, comes from a dev-dependency that itself
    /// depends on this crate, so its `Fs` does not unify with this crate's own (the same
    /// conflict `namespace_lock.rs`'s test section documents for `Fs`/`Clock`-backed tests).
    /// `resolve` itself only reads (`fs::read_dir`, never through `Fs`), so the fixed, static
    /// fixture folder is all a test here ever needs, with no seam or fake involved.
    fn fixture_root() -> PathBuf {
        typdoc_testkit::fixtures::path("valid/namespace-exclusion")
    }

    fn names_of(resolved: &Resolved) -> Vec<String> {
        let mut names: Vec<String> = resolved.namespaces.iter().map(|n| n.name.clone()).collect();
        names.sort();
        names
    }

    fn entries(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_later_exclusion_removes_what_an_earlier_wildcard_included() {
        let root = fixture_root();
        let mut report = Report::default();
        let resolved = resolve(
            &root,
            Some(&entries(&["story-*", "!story-1", "!story-2"])),
            &mut report,
        )
        .unwrap();
        assert_eq!(names_of(&resolved), vec!["story-3"]);
        assert!(
            report.errors().is_empty(),
            "excluding matched folders is not itself an error"
        );
    }

    #[test]
    fn a_later_plain_entry_re_includes_what_an_earlier_exclusion_removed() {
        let root = fixture_root();
        let mut report = Report::default();
        let resolved = resolve(
            &root,
            Some(&entries(&["story-*", "!story-1", "story-1"])),
            &mut report,
        )
        .unwrap();
        assert_eq!(
            names_of(&resolved),
            vec!["story-1", "story-2", "story-3"],
            "the last pattern that matches a folder decides, so the later plain `story-1` wins"
        );
        assert!(report.errors().is_empty());
    }

    #[test]
    fn an_exclusion_matching_no_folder_by_exact_name_is_silent() {
        let root = fixture_root();
        let mut report = Report::default();
        let resolved = resolve(&root, Some(&entries(&["!story-9"])), &mut report).unwrap();
        assert!(names_of(&resolved).is_empty());
        assert!(
            report.errors().is_empty(),
            "a `!` entry naming an exact name that matches nothing is always silent"
        );
    }

    #[test]
    fn an_exclusion_matching_no_folder_by_glob_is_silent() {
        let root = fixture_root();
        let mut report = Report::default();
        let resolved = resolve(&root, Some(&entries(&["!old-*"])), &mut report).unwrap();
        assert!(names_of(&resolved).is_empty());
        assert!(
            report.errors().is_empty(),
            "a `!` entry naming a glob that matches nothing is silent, same as a plain glob"
        );
    }

    #[test]
    fn a_plain_entry_matching_no_folder_by_exact_name_is_still_reported() {
        let root = fixture_root();
        let mut report = Report::default();
        let resolved = resolve(&root, Some(&entries(&["story-9"])), &mut report).unwrap();
        assert!(names_of(&resolved).is_empty());
        assert_eq!(report.errors().len(), 1);
        assert_eq!(report.errors()[0].id, "config.namespaces-entry");
        assert!(report.errors()[0].message.contains("story-9"));
    }

    #[test]
    fn a_plain_entry_matching_no_folder_by_glob_is_still_silent() {
        let root = fixture_root();
        let mut report = Report::default();
        let resolved = resolve(&root, Some(&entries(&["old-*"])), &mut report).unwrap();
        assert!(names_of(&resolved).is_empty());
        assert!(
            report.errors().is_empty(),
            "unchanged today's behavior: a glob matching nothing is silent"
        );
    }

    #[test]
    fn the_exclusion_error_message_quotes_the_full_entry_with_its_bang() {
        let root = fixture_root();
        let mut report = Report::default();
        // A malformed exclusion (more than one path segment) must still report, quoting the
        // original `!a/b` text and not the bare `a/b` pattern, so a user can find the offending
        // config line.
        let resolved = resolve(&root, Some(&entries(&["!a/b"])), &mut report).unwrap();
        assert!(names_of(&resolved).is_empty());
        assert_eq!(report.errors().len(), 1);
        assert!(report.errors()[0].message.contains("!a/b"));
    }
}
