//! An argument that names a document: a path or a key, told apart by its form and never
//! guessed (SPC-2). An on-disk path becomes a `DocumentArg` only in `discover_for`, because it
//! also decides which project it is in.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use crate::config::config_file;
use crate::env::Env;
use crate::error::Error;
use crate::project::discover;
use crate::template::valid_slug;

/// The argument of a command that names a document, once the project is known. A path or a
/// key may carry a namespace prefix (`story-2:notes/x.md`, `story-2:WF-5`), an import prefix
/// (`memory::precedents/x.md`), or both (`chief::story-3:WF-5`). A path with a namespace
/// prefix is read from that namespace's folder, as in a ref (SPC-18). An import prefix
/// changes which project the rest is read against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentArg {
    /// Relative to the project the document belongs to (this one, or, with `project` set, the
    /// imported one), with the case of the file kept.
    Path {
        project: Option<String>,
        namespace: Option<String>,
        path: String,
    },
    /// A key, with the namespace and the import its prefixes named, if any.
    Key {
        project: Option<String>,
        namespace: Option<String>,
        key: String,
    },
}

impl DocumentArg {
    /// The namespace a `namespace:` prefix on the argument named, independently of
    /// `project_prefix` (`chief::story-3:WF-5` carries both).
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

    /// This argument without its import prefix, to be resolved against the imported project. The
    /// namespace prefix is kept: it may still choose among that project's namespaces.
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
    /// Classifies `arg` by its form alone (SPC-2): on disk (`/`, `./`, `../`, ending in `.md`), a
    /// project-relative path (ending in `.md`), or a key (`CODE-number`, alone or with a slug).
    /// The path and key forms may carry a `project::` prefix, a `namespace:` prefix, or both.
    /// Nothing here reads the disk or knows which project the argument is in or which aliases are
    /// configured.
    pub fn parse(arg: &OsStr) -> Result<Argument, Error> {
        let text = arg.to_str().ok_or_else(|| {
            Error::BadArgument(format!(
                "{arg:?} is not valid UTF-8, so it names no document"
            ))
        })?;
        // An absolute path on this system is a path before any prefix is read, so Windows'
        // `C:\notes\a.md` is not the namespace `C`. On Unix that is the leading `/` already.
        // Windows writes the three leading forms with its own separator too (SPC-2).
        if text.starts_with('/')
            || text.starts_with("./")
            || text.starts_with("../")
            || Path::new(text).is_absolute()
            || (cfg!(windows)
                && (text.starts_with('\\') || text.starts_with(".\\") || text.starts_with("..\\")))
        {
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
                path: project_path_from_argument(rest),
            }));
        }
        // A written slug is dropped: an argument names the key's document whatever its slug,
        // and a command has no findings to report a stale one in (SPC-2).
        if let Some((key, _slug)) = read_key(rest) {
            return Ok(Argument::Named(DocumentArg::Key {
                project,
                namespace,
                key: key.to_owned(),
            }));
        }
        Err(Error::BadArgument(format!(
            "`{text}` is neither a path, which ends in `.md`, nor a key, `CODE-number`"
        )))
    }
}

/// A path relative to the project as it is typed. On Windows `\` cannot be part of a file name, so
/// it is the separator and is read as `/`, the one every project path is held and printed with;
/// on Unix it is a character a name may hold, and is kept (SPC-2). Paths written inside documents
/// never come through here.
pub fn project_path_from_argument(text: &str) -> String {
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text.to_owned()
    }
}

/// A key, `^[A-Z][A-Z0-9]*-\d+$`, alone or followed by `-` and a slug (SPC-17): the key and the
/// slug as written, or `None` when `text` is neither. The number is every digit after the code's
/// `-`, and the slug is the rest; text there that breaks the slug character rule makes `text` no
/// key at all, since SPC-14 reads a key written with a slug only when the rest is a slug. A key
/// never ends in `.md` (SPC-2): `WF-5-x.md` is a path, even where `x.md` would pass as a slug.
/// Written by hand so the crate takes on no regex engine for it. `refs` shares it; where the two readings
/// differ (a prefix, scope), each module keeps its own rule.
pub(crate) fn read_key(text: &str) -> Option<(&str, Option<&str>)> {
    if text.ends_with(".md") {
        return None;
    }
    let (code, after) = text.split_once('-')?;
    let mut chars = code.chars();
    let code_fits = chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    if !code_fits || digits == 0 {
        return None;
    }
    let key = &text[..code.len() + 1 + digits];
    match &after[digits..] {
        "" => Some((key, None)),
        rest => {
            let slug = rest.strip_prefix('-')?;
            valid_slug(slug).then_some((key, Some(slug)))
        }
    }
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
            // An on-disk path takes no `namespace:` prefix, so no scope of its own.
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

/// An on-disk path argument, read against a root an earlier argument of the same command already
/// fixed. One outside it is not found: a path on disk names no document of another project.
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
    use crate::env::ProcessStatus;

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

        fn hostname(&self) -> String {
            "fixed-host".to_owned()
        }

        fn process_status(&self, _pid: u32) -> ProcessStatus {
            ProcessStatus::Unknown
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

    #[cfg(windows)]
    #[test]
    fn a_leading_backslash_dot_backslash_or_dot_dot_backslash_is_on_disk_on_windows() {
        for text in [r"\a.md", r".\a.md", r"..\a.md"] {
            assert_eq!(parse(text), Argument::OnDisk(PathBuf::from(text)), "{text}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_drive_relative_windows_path_is_read_as_a_namespace_prefix() {
        assert_eq!(
            parse("C:note.md"),
            Argument::Named(DocumentArg::Path {
                project: None,
                namespace: Some("C".to_owned()),
                path: "note.md".to_owned()
            })
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_project_path_written_with_backslashes_is_read_with_slashes_after_any_prefix() {
        for (text, project, namespace, path) in [
            (r"notes\a.md", None, None, "notes/a.md"),
            (
                r"story-1:story-1\notes\a.md",
                None,
                Some("story-1"),
                "story-1/notes/a.md",
            ),
            (
                r"memory::learnings\a.md",
                Some("memory"),
                None,
                "learnings/a.md",
            ),
        ] {
            assert_eq!(
                parse(text),
                Argument::Named(DocumentArg::Path {
                    project: project.map(str::to_owned),
                    namespace: namespace.map(str::to_owned),
                    path: path.to_owned(),
                }),
                "{text}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_backslash_inside_a_project_path_is_part_of_the_name_on_unix() {
        assert_eq!(
            parse(r"notes\a.md"),
            Argument::Named(DocumentArg::Path {
                project: None,
                namespace: None,
                path: r"notes\a.md".to_owned()
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_leading_backslash_is_not_on_disk_on_unix() {
        assert_eq!(
            parse(r"\a.md"),
            Argument::Named(DocumentArg::Path {
                project: None,
                namespace: None,
                path: r"\a.md".to_owned()
            })
        );
    }

    #[cfg(windows)]
    #[test]
    fn an_absolute_windows_path_is_on_disk_and_not_a_namespace_prefix() {
        for text in [r"C:\notes\a.md", "C:/notes/a.md", r"\\server\share\a.md"] {
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
            assert_eq!(read_key(text), Some((text, None)), "{text}");
        }
        for text in ["wf-3", "3F-3", "WF-3a", "WF--3", "WF", "WF-", "-3", ""] {
            assert_eq!(read_key(text), None, "{text}");
        }
    }

    #[test]
    fn a_key_with_a_slug_ends_at_the_last_digit_and_the_slug_runs_to_the_end() {
        assert_eq!(read_key("WF-12-x"), Some(("WF-12", Some("x"))));
        assert_eq!(read_key("WF-1-2x"), Some(("WF-1", Some("2x"))));
        assert_eq!(read_key("WF-1-v1.2"), Some(("WF-1", Some("v1.2"))));
        assert_eq!(read_key("WF-1-a-b"), Some(("WF-1", Some("a-b"))));
        assert_eq!(read_key("WF-1-ลำดับ"), Some(("WF-1", Some("ลำดับ"))));
    }

    #[test]
    fn text_after_the_key_that_is_not_a_slug_makes_no_key() {
        for text in [
            "WF-1-",
            "WF-1-a b",
            "WF-1-a#b",
            "WF-1-a:b",
            "WF-1-a/b",
            "WF-1x",
            "WF-1-x.md",
        ] {
            assert_eq!(read_key(text), None, "{text}");
        }
    }

    #[test]
    fn a_key_argument_written_with_a_slug_is_the_key_alone_in_every_prefix_form() {
        for (text, project, namespace) in [
            ("WF-5-x", None, None),
            ("story-2:WF-5-x", None, Some("story-2")),
            ("chief::story-3:WF-5-x", Some("chief"), Some("story-3")),
        ] {
            assert_eq!(
                parse(text),
                Argument::Named(DocumentArg::Key {
                    project: project.map(str::to_owned),
                    namespace: namespace.map(str::to_owned),
                    key: "WF-5".to_owned()
                }),
                "{text}"
            );
        }
    }

    #[test]
    fn normalize_drops_current_dir_and_resolves_parent_dir_lexically() {
        assert_eq!(normalize(Path::new("a/./b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize(Path::new("/a/../b")), PathBuf::from("/b"));
    }
}
