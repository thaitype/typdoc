//! Ref forms in frontmatter, resolved through the index of names as they are on disk, and the
//! rules that check them: `refs.resolve`, `refs.target`, `refs.codedByPath`, `refs.moved` and
//! `refs.slug` (one ref at a time, from `resolve_one`) and `refs.acyclic` (a whole-project graph,
//! built from the same `resolve_one`, and turned into findings by `cyclic_findings`).
//! `names.shadowed` lives in `project.rs`, since it is a fact about the config and never reads a
//! document.
//!
//! A ref is read by the name grammar in `name.rs` (SPC-18), where the reading rules are tested
//! without a file system; the part of `resolve_one` that reads the disk is tested through
//! `validate` in the binary's tests.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::config::{Namespace, RefBase};
use crate::imports::Absence;
use crate::index::Index;
use crate::name::{self, InDocument, Place, Reading, Scene, Unresolved};
use crate::project::ImportState;
use crate::schema::{Target, normalize};

/// Why a ref did not resolve: the `unresolved` ids of SPC-12. `ImportAbsent` carries why the
/// import is absent, so `imports.absent`'s message can name the variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reason {
    NotFound,
    /// A key no document in the namespaces it was looked up in has: `not-found`, with the key
    /// named in the message.
    NoKey(String),
    BadPrefix,
    ImportAbsent(Absence),
    /// An absolute path, which a ref never follows (SPC-18).
    Absolute,
}

/// Whether a ref was written as a key or as a path: `refs.codedByPath` cares only about the
/// written form, never about which one a reader would have preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Via {
    Key,
    Path,
}

/// `collection` is `None` for a file outside every collection. When `project` names an import,
/// `collection` indexes that project's collections, not this one's, so a caller that looks up a
/// schema checks `project` first. `slug` is set for a ref written as a key with a slug and looked
/// up in an index; the cycle scan's shortcut to a write's own candidate leaves it `None`, since
/// only `refs.slug` reads it and that check never takes the shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub path: String,
    pub collection: Option<usize>,
    pub via: Via,
    pub project: Option<String>,
    pub slug: Option<WrittenSlug>,
}

/// A ref written as a key with a slug: the key, that slug, and the slug the target's file name
/// carries now (`None` when it has none), which `refs.slug` compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WrittenSlug {
    pub key: String,
    pub written: String,
    pub current: Option<String>,
}

impl WrittenSlug {
    pub fn is_stale(&self) -> bool {
        self.current.as_deref() != Some(self.written.as_str())
    }
}

pub(crate) type Outcome = Result<Resolved, Reason>;

/// The name and code of a collection's schema, so the ref rules can read them without
/// `project.rs`'s private `Loaded` type.
#[derive(Clone, Copy)]
pub(crate) struct SchemaInfo<'a> {
    pub name: &'a str,
    pub code: Option<&'a str>,
}

pub(crate) struct Ctx<'a> {
    pub doc_namespace: usize,
    pub doc_path: &'a str,
    pub ref_base: RefBase,
    pub namespaces: &'a [Namespace],
    pub index: &'a Index,
    pub root: &'a Path,
    pub imports: &'a BTreeMap<String, ImportState>,
}

/// An `Import` path is already joined against the imported project's folder, so it is resolved
/// against that project's index and root. A URL scheme is `Skip`, never a finding.
pub(crate) enum BodyDestination {
    Path(String),
    Import { alias: String, path: String },
    BadPrefix,
    ImportAbsent(Absence),
    Absolute,
    Skip,
}

impl Ctx<'_> {
    pub(crate) fn document(&self) -> InDocument<'_> {
        InDocument {
            path: self.doc_path,
            namespace: self.doc_namespace,
            ref_base: self.ref_base,
        }
    }

    fn scene(&self) -> Scene<'_> {
        Scene {
            namespaces: self.namespaces,
            index: self.index,
            root: self.root,
            imports: self.imports,
        }
    }
}

/// A frontmatter ref read by the name grammar (SPC-18). A ref never reads as a URL, and a key
/// with no prefix is looked up in the document's own namespace alone, so neither of the grammar's
/// other outcomes reaches here.
pub(crate) fn resolve_one(written: &str, ctx: &Ctx) -> Outcome {
    name::resolve(written, Place::Ref(ctx.document()), &ctx.scene()).map_err(|unresolved| {
        match unresolved {
            Unresolved::Reason(reason) => reason,
            Unresolved::Absolute => Reason::Absolute,
            Unresolved::NotARef | Unresolved::Ambiguous(_) => Reason::NotFound,
        }
    })
}

/// The identity a write's candidate is about to carry. `new`'s is not in `Ctx::index` or on disk
/// yet, though its path and any key are already decided.
pub(crate) struct Candidate<'a> {
    pub namespace: usize,
    pub key: Option<&'a str>,
    pub path: &'a str,
}

/// Like [`resolve_one`], except a ref naming `candidate`'s own key or path resolves to it without
/// reading the index or the disk. The write-time cycle check (SPC-2) needs this for the refs of
/// every other document: without it, nothing on disk could resolve to a `new` candidate, and
/// `new` could never be seen to close a cycle.
pub(crate) fn resolve_one_for_candidate(
    written: &str,
    ctx: &Ctx,
    candidate: &Candidate,
) -> Outcome {
    let via = match name::read(written, Place::Ref(ctx.document()), ctx.namespaces) {
        Reading::Key {
            namespaces, key, ..
        } if namespaces == [candidate.namespace] && candidate.key == Some(key.as_str()) => {
            Some(Via::Key)
        }
        Reading::Path(path) if path == candidate.path => Some(Via::Path),
        _ => None,
    };
    if let Some(via) = via {
        return Ok(Resolved {
            path: candidate.path.to_owned(),
            collection: None,
            via,
            project: None,
            slug: None,
        });
    }
    resolve_one(written, ctx)
}

/// `target` is percent-decoded with any `#anchor` split off, and read by the name grammar as a body
/// link (SPC-18): always a path, never a key, and a URL is `Skip`, never a finding. An `Import`
/// path is from the imported project's folder, to be resolved against that project's index.
pub(crate) fn classify_body(target: &str, ctx: &Ctx) -> BodyDestination {
    match name::locate(target, Place::BodyLink(ctx.document()), &ctx.scene()) {
        Ok(located) => {
            let Reading::Path(path) = located.reading else {
                return BodyDestination::BadPrefix;
            };
            match located.import {
                Some(imported) => BodyDestination::Import {
                    alias: imported.alias,
                    path,
                },
                None => BodyDestination::Path(path),
            }
        }
        Err(Unresolved::Reason(Reason::ImportAbsent(absence))) => {
            BodyDestination::ImportAbsent(absence)
        }
        Err(Unresolved::Reason(Reason::Absolute) | Unresolved::Absolute) => {
            BodyDestination::Absolute
        }
        Err(Unresolved::NotARef) => BodyDestination::Skip,
        Err(
            // `locate` looks nothing up, so only `BadPrefix` reaches here; the others read as it.
            Unresolved::Reason(Reason::BadPrefix | Reason::NotFound | Reason::NoKey(_))
            | Unresolved::Ambiguous(_),
        ) => BodyDestination::BadPrefix,
    }
}

#[expect(
    clippy::expect_used,
    reason = "the one caller, `Project::mention_missing`, passes the part after the last `:` of \
              a `Mention.written`, which `links::mention_shape` accepts only when \
              `looks_like_key_shape` holds for that same part, and the key shape needs a dash"
)]
pub(crate) fn code_of(key: &str) -> &str {
    key.split_once('-')
        .map(|(code, _)| code)
        .expect("a key has a dash")
}

/// The name in `path` that carries `key`, for a message: the last segment where `key` is
/// followed by no further digit, or the last segment when none is.
pub(crate) fn name_with_key<'a>(path: &'a str, key: &str) -> &'a str {
    let carries = |segment: &str| {
        segment
            .match_indices(key)
            .any(|(at, _)| !segment[at + key.len()..].starts_with(|c: char| c.is_ascii_digit()))
    };
    path.rsplit('/')
        .find(|segment| carries(segment))
        .or_else(|| path.rsplit('/').next())
        .unwrap_or(path)
}

/// A file on disk that matches no collection resolves with no collection; only `target: "*"`
/// accepts it (SPC-15).
///
/// The fallback must be as case-exact as the index (SPC-14): `Path::is_file` finds a wrongly
/// cased path on a case-insensitive file system such as macOS's APFS, so `case_exact_file`
/// compares real directory entries.
pub(crate) fn resolve_path(path: &str, index: &Index, root: &Path) -> Outcome {
    if let Some(entry) = index.get(path) {
        return Ok(Resolved {
            path: path.to_owned(),
            collection: Some(entry.collection),
            via: Via::Path,
            project: None,
            slug: None,
        });
    }
    if case_exact_file(root, path) {
        return Ok(Resolved {
            path: path.to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
            slug: None,
        });
    }
    Err(Reason::NotFound)
}

/// Every component is compared, not only the last: a file system that folds case does so for
/// each component of a lookup.
///
/// Normalizes first, so `.` and `..` are never searched for as names, whatever path a caller
/// passes.
///
/// A missing or unreadable directory answers `false`. A symbolic link is followed: only the
/// spelling of each name is checked.
fn case_exact_file(root: &Path, path: &str) -> bool {
    let normalized = normalize(path);
    let relative = normalized.strip_prefix('/').unwrap_or(&normalized);
    if relative.is_empty() {
        return false;
    }

    let mut current = root.to_path_buf();
    let mut components = relative.split('/').peekable();
    while let Some(component) = components.next() {
        let Some(entry_path) = exact_entry(&current, component) else {
            return false;
        };
        if components.peek().is_none() {
            return entry_path.is_file();
        }
        current = entry_path;
    }
    false
}

/// `None` also when `dir` cannot be read. An entry the iterator cannot read is skipped, so one
/// unreadable sibling does not hide a real match.
fn exact_entry(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        if entry.file_name() == OsStr::new(name) {
            return Some(entry.path());
        }
    }
    None
}

/// No `target` allows any file (SPC-15). `Target::Other` is already reported by `schema.valid`,
/// so it restricts nothing here rather than being reported twice. A bare name allows only a
/// schema of this project and a qualified name only one of that import (SPC-15).
pub(crate) fn target_allowed(
    target: Option<&Target>,
    resolved: &Resolved,
    info: Option<&SchemaInfo>,
) -> bool {
    match target.unwrap_or(&Target::Any) {
        Target::Any | Target::Other(_) => true,
        Target::Schemas(names) => match info {
            None => false,
            Some(info) => match &resolved.project {
                None => names.iter().any(|name| name == info.name),
                Some(alias) => names
                    .iter()
                    .any(|name| *name == format!("{alias}::{}", info.name)),
            },
        },
    }
}

/// `refs.acyclic` reports once per node returned, never once per cycle, since a node can sit on
/// more than one.
pub(crate) fn cyclic_nodes(edges: &[(String, String)]) -> BTreeSet<String> {
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for (source, target) in edges {
        outgoing
            .entry(source.as_str())
            .or_default()
            .push(target.as_str());
        nodes.insert(source.as_str());
        nodes.insert(target.as_str());
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    // Iterative, with one frame per node on the current path, so a long chain grows the heap
    // rather than the thread stack.
    fn visit<'a>(
        start: &'a str,
        outgoing: &BTreeMap<&'a str, Vec<&'a str>>,
        color: &mut BTreeMap<&'a str, Color>,
        stack: &mut Vec<&'a str>,
        cyclic: &mut BTreeSet<String>,
    ) {
        struct Frame<'a> {
            node: &'a str,
            next_child: usize,
        }

        color.insert(start, Color::Gray);
        stack.push(start);
        let mut frames: Vec<Frame<'a>> = vec![Frame {
            node: start,
            next_child: 0,
        }];

        while let Some(frame) = frames.last_mut() {
            let children: &[&str] = match outgoing.get(frame.node) {
                Some(children) => children.as_slice(),
                None => &[],
            };
            match children.get(frame.next_child) {
                Some(&child) => {
                    frame.next_child += 1;
                    match color.get(child).copied().unwrap_or(Color::White) {
                        Color::White => {
                            color.insert(child, Color::Gray);
                            stack.push(child);
                            frames.push(Frame {
                                node: child,
                                next_child: 0,
                            });
                        }
                        Color::Gray => {
                            if let Some(at) = stack.iter().position(|n| *n == child) {
                                for n in &stack[at..] {
                                    cyclic.insert((*n).to_owned());
                                }
                            }
                        }
                        Color::Black => {}
                    }
                }
                None => {
                    let finished = frame.node;
                    frames.pop();
                    stack.pop();
                    color.insert(finished, Color::Black);
                }
            }
        }
    }

    let mut color: BTreeMap<&str, Color> = BTreeMap::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut cyclic: BTreeSet<String> = BTreeSet::new();
    for &node in &nodes {
        if !matches!(color.get(node), Some(Color::Black)) {
            visit(node, &outgoing, &mut color, &mut stack, &mut cyclic);
        }
    }
    cyclic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_with_key_is_the_segment_that_carries_the_key() {
        assert_eq!(
            name_with_key("story-2/tickets/WF-5-json-shapes.md", "WF-5"),
            "WF-5-json-shapes.md"
        );
        assert_eq!(name_with_key("WF-5-x/README.md", "WF-5"), "WF-5-x");
        assert_eq!(name_with_key("WF-5/WF-50.md", "WF-5"), "WF-5");
    }

    #[test]
    fn target_any_or_no_target_at_all_allows_any_resolved_ref() {
        let file = Resolved {
            path: "README.md".to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
            slug: None,
        };

        assert!(target_allowed(None, &file, None));
        assert!(target_allowed(Some(&Target::Any), &file, None));
    }

    #[test]
    fn a_target_list_of_schema_names_refuses_a_file_with_no_schema() {
        let file = Resolved {
            path: "README.md".to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
            slug: None,
        };

        assert!(!target_allowed(
            Some(&Target::Schemas(vec!["wayfinder".to_owned()])),
            &file,
            None
        ));
    }

    fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(s, t)| (s.to_string(), t.to_string()))
            .collect()
    }

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_two_node_cycle_reports_both_nodes() {
        let found = cyclic_nodes(&edges(&[("a", "b"), ("b", "a")]));

        assert_eq!(found, set(&["a", "b"]));
    }

    #[test]
    fn a_self_loop_reports_its_one_node() {
        let found = cyclic_nodes(&edges(&[("a", "a")]));

        assert_eq!(found, set(&["a"]));
    }

    #[test]
    fn a_chain_with_no_cycle_reports_nothing() {
        let found = cyclic_nodes(&edges(&[("a", "b"), ("b", "c")]));

        assert_eq!(found, BTreeSet::new());
    }

    #[test]
    fn a_cycle_with_a_tail_does_not_report_the_tail() {
        let found = cyclic_nodes(&edges(&[("a", "b"), ("b", "a"), ("b", "c")]));

        assert_eq!(
            found,
            set(&["a", "b"]),
            "c only receives an edge, it starts none"
        );
    }

    /// A walk that recursed once per node would overflow the small stack this runs on.
    #[test]
    fn a_very_long_one_way_chain_with_no_cycle_does_not_overflow_the_stack() {
        const CHAIN_LENGTH: usize = 200_000;
        // Set explicitly, so the result does not depend on the machine or the harness.
        const SMALL_STACK_BYTES: usize = 1024 * 1024;

        let edges: Vec<(String, String)> = (0..CHAIN_LENGTH)
            .map(|i| (format!("n{i}"), format!("n{}", i + 1)))
            .collect();

        let handle = std::thread::Builder::new()
            .stack_size(SMALL_STACK_BYTES)
            .spawn(move || cyclic_nodes(&edges))
            .expect("spawning a thread with an explicit stack size does not itself fail");

        let found = handle
            .join()
            .expect("cyclic_nodes must not overflow the stack on a long acyclic chain");

        assert_eq!(
            found,
            BTreeSet::new(),
            "a one-way chain with no cycle reports no cyclic nodes, however long it is"
        );
    }

    #[test]
    fn a_target_list_of_schema_names_allows_exactly_the_named_schemas() {
        let info = SchemaInfo {
            name: "wayfinder",
            code: Some("WF"),
        };
        let matching = Resolved {
            path: "tickets/WF-1.md".to_owned(),
            collection: Some(0),
            via: Via::Key,
            project: None,
            slug: None,
        };

        assert!(target_allowed(
            Some(&Target::Schemas(vec!["wayfinder".to_owned()])),
            &matching,
            Some(&info)
        ));
        assert!(!target_allowed(
            Some(&Target::Schemas(vec!["other".to_owned()])),
            &matching,
            Some(&info)
        ));
    }

    #[test]
    fn a_bare_name_in_target_never_allows_a_ref_that_crossed_an_import() {
        let info = SchemaInfo {
            name: "learning",
            code: None,
        };
        let crossed = Resolved {
            path: "precedents/x.md".to_owned(),
            collection: Some(0),
            via: Via::Path,
            project: Some("memory".to_owned()),
            slug: None,
        };

        assert!(
            !target_allowed(
                Some(&Target::Schemas(vec!["learning".to_owned()])),
                &crossed,
                Some(&info)
            ),
            "a bare name means a schema of this project, never one reached through an import"
        );
    }

    #[test]
    fn a_qualified_name_in_target_allows_exactly_the_schema_it_names_in_that_import() {
        let info = SchemaInfo {
            name: "learning",
            code: None,
        };
        let crossed = Resolved {
            path: "precedents/x.md".to_owned(),
            collection: Some(0),
            via: Via::Path,
            project: Some("memory".to_owned()),
            slug: None,
        };

        assert!(target_allowed(
            Some(&Target::Schemas(vec!["memory::learning".to_owned()])),
            &crossed,
            Some(&info)
        ));
        assert!(!target_allowed(
            Some(&Target::Schemas(vec!["memory::precedent".to_owned()])),
            &crossed,
            Some(&info)
        ));
        assert!(
            !target_allowed(
                Some(&Target::Schemas(vec!["other::learning".to_owned()])),
                &crossed,
                Some(&info)
            ),
            "the qualified name has to name the alias the ref actually crossed, not just any \
             import"
        );
    }
}
