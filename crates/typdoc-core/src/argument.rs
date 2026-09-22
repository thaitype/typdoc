//! An argument that names a document: a path or a key, told apart by its form and never
//! guessed. `Argument::parse` reads only the string; an on-disk path is not turned into a
//! `DocumentArg` until `discover_for` runs, because finding the project is part of what an
//! on-disk path decides, and that needs the environment.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use crate::config::config_file;
use crate::env::Env;
use crate::error::Error;
use crate::project::discover;

/// The argument of a command that names a document, once the project is known. A path or a
/// key may carry a namespace prefix (`story-2:notes/x.md`, `story-2:WF-5`), an import prefix
/// (`memory::precedents/x.md`), or both together when the import has several namespaces
/// (`chief::story-3:WF-5`). A namespace prefix only chooses scope; it never changes what the
/// path is read against, which stays the project folder (see `Project::resolve`; ticket 4
/// already decided a path is not narrowed by scope). An import prefix does change which
/// project the rest is read against entirely (`Project::get`/`toc`/`refs` each check
/// `project_prefix` first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentArg {
    /// Relative to the project the document belongs to (this one, or, with `project` set, the
    /// imported one), with the case of the file kept.
    Path {
        project: Option<String>,
        namespace: Option<String>,
        path: String,
    },
    /// The namespace a prefix on it named, if any (`story-2:WF-5`), and the project an import
    /// prefix named, if any (`memory::WF-5`, `chief::story-3:WF-5`).
    Key {
        project: Option<String>,
        namespace: Option<String>,
        key: String,
    },
}

impl DocumentArg {
    /// The namespace a `namespace:` prefix on the argument named, if it had one — present or
    /// absent independently of `project_prefix` (`chief::story-3:WF-5` carries both).
    pub fn namespace_prefix(&self) -> Option<&str> {
        match self {
            DocumentArg::Path { namespace, .. } | DocumentArg::Key { namespace, .. } => {
                namespace.as_deref()
            }
        }
    }

    /// The alias a `project::` prefix on the argument named, if it had one.
    pub fn project_prefix(&self) -> Option<&str> {
        match self {
            DocumentArg::Path { project, .. } | DocumentArg::Key { project, .. } => {
                project.as_deref()
            }
        }
    }

    /// This argument with its import prefix removed, for resolving the rest directly against the
    /// imported project (`Project::get`/`toc`/`refs`'s own dispatch): the namespace prefix, if
    /// any, is kept, since it may still be needed to choose among that project's namespaces.
    pub fn without_project_prefix(&self) -> DocumentArg {
        match self {
            DocumentArg::Path {
                namespace, path, ..
            } => DocumentArg::Path {
                project: None,
                namespace: namespace.clone(),
                path: path.clone(),
            },
            DocumentArg::Key { namespace, key, .. } => DocumentArg::Key {
                project: None,
                namespace: namespace.clone(),
                key: key.clone(),
            },
        }
    }
}

/// What `parse` classifies an argument as, before the project is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// Absolute, or relative to the current directory: `/`, `./` or `../`.
    OnDisk(PathBuf),
    Named(DocumentArg),
}

impl Argument {
    /// Classifies `arg` by its form alone: on disk (`/`, `./`, `../`, and it must end in
    /// `.md`), a project-relative path (ends in `.md`), or a key (`CODE-number`); any of the
    /// three may carry a `project::` prefix (an import), and the path and key forms may also
    /// carry a `namespace:` prefix after it ("Choosing a namespace" names both a key and a path
    /// argument as candidates for one; the design's own `project::path` and
    /// `project::namespace:key` forms nest the two). A path and a key can never be confused,
    /// since a key never ends in `.md`. Nothing here reads the disk, knows what project the
    /// argument is in, or knows whether an alias it names is actually configured — that is
    /// `Project::get`/`toc`/`refs`'s job, once the project is known.
    pub fn parse(arg: &OsStr) -> Result<Argument, Error> {
        let text = arg.to_str().ok_or_else(|| {
            Error::BadArgument(format!(
                "{arg:?} is not valid UTF-8, so it names no document"
            ))
        })?;
        if text.starts_with('/') || text.starts_with("./") || text.starts_with("../") {
            return if text.ends_with(".md") {
                Ok(Argument::OnDisk(PathBuf::from(text)))
            } else {
                Err(Error::BadArgument(format!(
                    "`{text}` is not a path: a path ends in `.md`"
                )))
            };
        }
        let (project, text) = match text.split_once("::") {
            Some((alias, rest)) => {
                if alias.is_empty() || rest.contains("::") {
                    return Err(Error::BadArgument(format!(
                        "`{text}` is not a valid import prefix: imports of imports are not read"
                    )));
                }
                (Some(alias.to_owned()), rest)
            }
            None => (None, text),
        };
        let (namespace, rest) = match text.split_once(':') {
            Some((namespace, rest)) => (Some(namespace.to_owned()), rest),
            None => (None, text),
        };
        if rest.ends_with(".md") {
            return Ok(Argument::Named(DocumentArg::Path {
                project,
                namespace,
                path: rest.to_owned(),
            }));
        }
        if looks_like_key(rest) {
            return Ok(Argument::Named(DocumentArg::Key {
                project,
                namespace,
                key: rest.to_owned(),
            }));
        }
        Err(Error::BadArgument(format!(
            "`{text}` is neither a path, which ends in `.md`, nor a key, `CODE-number`"
        )))
    }
}

/// `^[A-Z][A-Z0-9]*-\d+$`, written by hand so the crate takes on no regex engine for it. Shared
/// with `refs`, which tells a bare key apart from a relative path the same way an argument does
/// (the key shape itself is one fact about the world; where the two readings differ — a prefix,
/// scope — each module keeps its own rule, per ticket 7's report).
pub(crate) fn looks_like_key(text: &str) -> bool {
    let Some((code, digits)) = text.split_once('-') else {
        return false;
    };
    let mut chars = code.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Finds the project for a command with a document argument. An on-disk path names the
/// project by itself, walking up from its own folder, and wins over `TYPDOC_DIR`; any other
/// argument is read against the project found the usual way (see `discover`).
pub fn discover_for(arg: Argument, env: &dyn Env) -> Result<(PathBuf, DocumentArg), Error> {
    match arg {
        Argument::OnDisk(given) => {
            let cwd = env.current_dir().map_err(Error::io_at(Path::new(".")))?;
            let absolute = normalize(&if given.is_absolute() {
                given.clone()
            } else {
                cwd.join(&given)
            });
            let dir = absolute.parent().unwrap_or(&absolute).to_owned();
            let root = dir
                .ancestors()
                .find(|candidate| config_file(candidate).is_file())
                .map(Path::to_owned)
                .ok_or_else(|| Error::NoProject { from: dir.clone() })?;
            #[expect(
                clippy::expect_used,
                reason = "`root` is one of `dir.ancestors()`, and `dir` is the parent of `absolute` (or \
                          `absolute` itself when it has none), so `root` is a leading run of the \
                          components of `absolute`, which is what `strip_prefix` asks for"
            )]
            let relative = absolute
                .strip_prefix(&root)
                .expect("root came from walking up the file's own folder, so it is a prefix");
            let path = to_project_path(relative, &given)?;
            // An on-disk path is never written with a `namespace:` prefix (that syntax applies
            // only to the project-relative and key forms); it takes no scope of its own.
            Ok((
                root,
                DocumentArg::Path {
                    project: None,
                    namespace: None,
                    path,
                },
            ))
        }
        Argument::Named(document) => Ok((discover(env)?, document)),
    }
}

/// An on-disk path argument, resolved against a project root already known (unlike
/// `discover_for`, which finds the root by walking up from the file itself). Used when several
/// document arguments are given to one command and the root was already fixed by the first of
/// them: every later on-disk argument is read against that same root, and one outside it is
/// not found, since a path on disk names no document of a different project.
pub fn resolve_on_disk(root: &Path, given: &Path, env: &dyn Env) -> Result<DocumentArg, Error> {
    let cwd = env.current_dir().map_err(Error::io_at(Path::new(".")))?;
    let absolute = normalize(&if given.is_absolute() {
        given.to_owned()
    } else {
        cwd.join(given)
    });
    let relative = absolute.strip_prefix(root).map_err(|_| Error::NotFound {
        path: given.to_string_lossy().into_owned(),
        hint: false,
    })?;
    let path = to_project_path(relative, given)?;
    Ok(DocumentArg::Path {
        project: None,
        namespace: None,
        path,
    })
}

/// `path` with `.` dropped and each `..` taken against the segment before it, lexically: no
/// symbolic link is read and nothing named here has to exist.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `relative` joined with `/`, the separator every path in `--json` uses. `original` is the
/// argument as written, for the message when a component is not UTF-8.
fn to_project_path(relative: &Path, original: &Path) -> Result<String, Error> {
    let parts: Option<Vec<&str>> = relative
        .components()
        .map(|c| c.as_os_str().to_str())
        .collect();
    parts.map(|parts| parts.join("/")).ok_or_else(|| {
        Error::BadArgument(format!(
            "{original:?} is not valid UTF-8, so it names no document"
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io;

    use super::*;

    struct FixedEnv {
        cwd: PathBuf,
    }

    impl Env for FixedEnv {
        fn var(&self, _name: &str) -> Option<OsString> {
            None
        }

        fn current_dir(&self) -> io::Result<PathBuf> {
            Ok(self.cwd.clone())
        }

        fn hostname(&self) -> io::Result<String> {
            Ok("test-host".to_owned())
        }
    }

    #[test]
    fn resolve_on_disk_reads_a_path_against_a_root_already_known_and_not_by_walking_up() {
        let root = Path::new("/proj");
        let env = FixedEnv {
            cwd: PathBuf::from("/proj/sub"),
        };

        assert_eq!(
            resolve_on_disk(root, Path::new("./a.md"), &env).unwrap(),
            DocumentArg::Path {
                project: None,
                namespace: None,
                path: "sub/a.md".to_owned()
            }
        );
        assert_eq!(
            resolve_on_disk(root, Path::new("/proj/b.md"), &env).unwrap(),
            DocumentArg::Path {
                project: None,
                namespace: None,
                path: "b.md".to_owned()
            }
        );
    }

    #[test]
    fn resolve_on_disk_outside_the_known_root_is_not_found() {
        let root = Path::new("/proj");
        let env = FixedEnv {
            cwd: PathBuf::from("/elsewhere"),
        };

        let error = resolve_on_disk(root, Path::new("./a.md"), &env).unwrap_err();

        assert!(matches!(error, Error::NotFound { .. }), "{error}");
    }

    fn parse(text: &str) -> Argument {
        Argument::parse(OsStr::new(text)).unwrap()
    }

    fn parse_err(text: &str) -> String {
        Argument::parse(OsStr::new(text)).unwrap_err().to_string()
    }

    #[test]
    fn a_path_ends_in_md_and_a_key_never_does() {
        assert_eq!(
            parse("notes/a.md"),
            Argument::Named(DocumentArg::Path {
                project: None,
                namespace: None,
                path: "notes/a.md".to_owned()
            })
        );
        assert_eq!(
            parse("WF-3"),
            Argument::Named(DocumentArg::Key {
                project: None,
                namespace: None,
                key: "WF-3".to_owned()
            })
        );
    }

    #[test]
    fn a_namespace_prefix_is_read_from_a_key_argument_and_from_a_path_argument_alike() {
        assert_eq!(
            parse("story-2:WF-5"),
            Argument::Named(DocumentArg::Key {
                project: None,
                namespace: Some("story-2".to_owned()),
                key: "WF-5".to_owned()
            })
        );
        assert_eq!(
            parse("story-2:notes/x.md"),
            Argument::Named(DocumentArg::Path {
                project: None,
                namespace: Some("story-2".to_owned()),
                path: "notes/x.md".to_owned()
            })
        );
    }

    #[test]
    fn a_leading_slash_dot_slash_or_dot_dot_slash_is_on_disk() {
        for text in ["/a.md", "./a.md", "../a.md"] {
            assert_eq!(parse(text), Argument::OnDisk(PathBuf::from(text)), "{text}");
        }
    }

    #[test]
    fn a_name_that_starts_with_two_dots_but_is_not_a_parent_reference_is_a_plain_path() {
        assert_eq!(
            parse("..two/a.md"),
            Argument::Named(DocumentArg::Path {
                project: None,
                namespace: None,
                path: "..two/a.md".to_owned()
            })
        );
    }

    #[test]
    fn an_on_disk_argument_that_does_not_end_in_md_is_bad_arguments() {
        for text in ["/note", "./note.txt", "../note"] {
            assert!(parse_err(text).contains("path"), "{text}");
        }
    }

    #[test]
    fn a_double_colon_is_an_import_prefix_read_the_same_way_as_a_path_or_a_key() {
        assert_eq!(
            parse("memory::precedents/x.md"),
            Argument::Named(DocumentArg::Path {
                project: Some("memory".to_owned()),
                namespace: None,
                path: "precedents/x.md".to_owned()
            })
        );
        assert_eq!(
            parse("memory::LRN-5"),
            Argument::Named(DocumentArg::Key {
                project: Some("memory".to_owned()),
                namespace: None,
                key: "LRN-5".to_owned()
            })
        );
    }

    #[test]
    fn an_import_prefix_and_a_namespace_prefix_nest_for_a_key() {
        assert_eq!(
            parse("chief::story-3:WF-5"),
            Argument::Named(DocumentArg::Key {
                project: Some("chief".to_owned()),
                namespace: Some("story-3".to_owned()),
                key: "WF-5".to_owned()
            })
        );
    }

    #[test]
    fn a_double_colon_with_an_empty_alias_is_bad_arguments() {
        assert!(parse_err("::WF-3").contains("import"));
    }

    #[test]
    fn imports_of_imports_are_not_read_as_an_argument() {
        let error = parse_err("chief::other::WF-3");

        assert!(error.contains("imports of imports"), "{error}");
    }

    #[test]
    fn project_prefix_and_without_project_prefix_read_a_parsed_argument_apart() {
        let with_both = DocumentArg::Key {
            project: Some("chief".to_owned()),
            namespace: Some("story-3".to_owned()),
            key: "WF-5".to_owned(),
        };

        assert_eq!(with_both.project_prefix(), Some("chief"));
        assert_eq!(with_both.namespace_prefix(), Some("story-3"));
        assert_eq!(
            with_both.without_project_prefix(),
            DocumentArg::Key {
                project: None,
                namespace: Some("story-3".to_owned()),
                key: "WF-5".to_owned(),
            }
        );
    }

    #[test]
    fn neither_a_path_nor_a_key_is_bad_arguments() {
        for text in ["note", "note.txt", "wf-3", "-3", "WF-", "WF3", ""] {
            assert!(Argument::parse(OsStr::new(text)).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_key_is_one_or_more_letters_or_digits_starting_with_a_letter_then_a_dash_and_digits() {
        for text in ["WF-3", "A1-30", "AB-007"] {
            assert!(looks_like_key(text), "{text}");
        }
        for text in ["wf-3", "3F-3", "WF-3a", "WF--3", "WF", "WF-"] {
            assert!(!looks_like_key(text), "{text}");
        }
    }

    #[test]
    fn normalize_drops_current_dir_and_resolves_parent_dir_lexically() {
        assert_eq!(normalize(Path::new("a/./b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize(Path::new("/a/../b")), PathBuf::from("/b"));
    }
}
