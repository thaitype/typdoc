//! Ref forms in frontmatter, resolved through the index of names as they are on disk, and the
//! rules that check them: `refs.resolve`, `refs.target`, `refs.codedByPath` and `refs.moved` (one
//! ref at a time, from `resolve_one`) and `refs.acyclic` (a whole-project graph, built from the
//! same `resolve_one`, and turned into findings by `cyclic_findings`). `names.shadowed` lives in
//! `project.rs`, since it is a fact about the config and never reads a document.
//!
//! A ref is not an argument: a bare form always means the document's own namespace, whatever the
//! working directory, and a prefix that names neither a sibling namespace nor an import is
//! `bad-prefix`, never a relative path (SPC-14).
//!
//! `classify` reads no file and no index, so the form rules are tested here without a file
//! system; the part of `resolve_one` that reads the disk is tested through `validate` in the
//! binary's tests.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::argument::looks_like_key;
use crate::config::{Namespace, RefBase};
use crate::imports::Absence;
use crate::index::Index;
use crate::project::ImportState;
use crate::schema::{Target, normalize};

/// Why a ref did not resolve: the `unresolved` ids of SPC-12. `ImportAbsent` carries why the
/// import is absent, so `imports.absent`'s message can name the variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reason {
    NotFound,
    BadPrefix,
    ImportAbsent(Absence),
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
/// schema checks `project` first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub path: String,
    pub collection: Option<usize>,
    pub via: Via,
    pub project: Option<String>,
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
    pub codes: &'a BTreeSet<String>,
    pub index: &'a Index,
    pub root: &'a Path,
    pub imports: &'a BTreeMap<String, ImportState>,
}

enum Form {
    Key {
        namespace: usize,
        key: String,
    },
    Path {
        base: String,
        rest: String,
    },
    /// Not yet looked up: `classify` reads no index, and `Ctx::imports` decides between
    /// `bad-prefix`, `import-absent` and a resolution.
    Import {
        alias: String,
        rest: String,
    },
    BadPrefix,
}

/// An `Import` path is already joined against the imported project's folder, so it is resolved
/// against that project's index and root. A URL scheme is `Skip`, never a finding.
pub(crate) enum BodyDestination {
    Path(String),
    Import { alias: String, path: String },
    BadPrefix,
    ImportAbsent(Absence),
    Skip,
}

/// Resolves one frontmatter ref by the forms in SPC-14.
pub(crate) fn resolve_one(written: &str, ctx: &Ctx) -> Outcome {
    match classify(
        written,
        ctx.doc_namespace,
        ctx.doc_path,
        ctx.ref_base,
        ctx.namespaces,
        ctx.codes,
    ) {
        Form::Key { namespace, key } => resolve_key(namespace, &key, ctx.index),
        Form::Path { base, rest } => resolve_path(&join(&base, &rest), ctx.index, ctx.root),
        Form::Import { alias, rest } => resolve_into_import(&alias, &rest, ctx.imports),
        Form::BadPrefix => Err(Reason::BadPrefix),
    }
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
    match classify(
        written,
        ctx.doc_namespace,
        ctx.doc_path,
        ctx.ref_base,
        ctx.namespaces,
        ctx.codes,
    ) {
        Form::Key { namespace, key } => {
            if namespace == candidate.namespace && candidate.key == Some(key.as_str()) {
                return Ok(Resolved {
                    path: candidate.path.to_owned(),
                    collection: None,
                    via: Via::Key,
                    project: None,
                });
            }
            resolve_key(namespace, &key, ctx.index)
        }
        Form::Path { base, rest } => {
            let joined = join(&base, &rest);
            if joined == candidate.path {
                return Ok(Resolved {
                    path: candidate.path.to_owned(),
                    collection: None,
                    via: Via::Path,
                    project: None,
                });
            }
            resolve_path(&joined, ctx.index, ctx.root)
        }
        Form::Import { alias, rest } => resolve_into_import(&alias, &rest, ctx.imports),
        Form::BadPrefix => Err(Reason::BadPrefix),
    }
}

fn resolve_into_import(
    alias: &str,
    rest: &str,
    imports: &BTreeMap<String, ImportState>,
) -> Outcome {
    match imports.get(alias) {
        None => Err(Reason::BadPrefix),
        Some(ImportState::Absent(absence)) => Err(Reason::ImportAbsent(absence.clone())),
        Some(ImportState::Loaded(imported)) => resolve_into_project(
            rest,
            imported.namespaces(),
            imported.index_ref(),
            imported.root_ref(),
            &imported.codes(),
        )
        .map(|resolved| Resolved {
            project: Some(alias.to_owned()),
            ..resolved
        }),
    }
}

/// `rest` after an import prefix (SPC-14). A bare key into a project with several namespaces is
/// `bad-prefix` even when no key collides. A path is read from the imported project's folder,
/// never by `refBase`, which belongs to the referring document's collection. A further `::` is
/// `bad-prefix`: imports of imports are ignored.
fn resolve_into_project(
    rest: &str,
    namespaces: &[Namespace],
    index: &Index,
    root: &Path,
    codes: &BTreeSet<String>,
) -> Outcome {
    if rest.contains("::") {
        return Err(Reason::BadPrefix);
    }
    if let Some((prefix, sub)) = rest.split_once(':') {
        let namespace = namespace_named(namespaces, prefix).ok_or(Reason::BadPrefix)?;
        return if looks_like_key(sub) {
            resolve_key(namespace, sub, index)
        } else {
            resolve_path(&join(&namespaces[namespace].folder, sub), index, root)
        };
    }
    if looks_like_key(rest) && codes.contains(code_of(rest)) {
        if namespaces.len() != 1 {
            return Err(Reason::BadPrefix);
        }
        return resolve_key(0, rest, index);
    }
    resolve_path(rest, index, root)
}

/// `target` is percent-decoded with any `#anchor` split off. The prefix rules are a frontmatter
/// ref's, except that a body link is always a path, never a key, and a single colon whose prefix
/// names no sibling is a URL scheme, skipped rather than `bad-prefix`, since body text holds
/// real URLs.
pub(crate) fn classify_body(target: &str, ctx: &Ctx) -> BodyDestination {
    if target.starts_with("./") || target.starts_with("../") {
        let base = base_of(
            ctx.ref_base,
            ctx.doc_path,
            ctx.doc_namespace,
            ctx.namespaces,
        );
        return BodyDestination::Path(join(&base, target));
    }
    if let Some((alias, rest)) = target.split_once("::") {
        return match ctx.imports.get(alias) {
            None => BodyDestination::BadPrefix,
            Some(ImportState::Absent(absence)) => BodyDestination::ImportAbsent(absence.clone()),
            Some(ImportState::Loaded(imported)) => {
                // `alias::namespace:path` is accepted as a frontmatter ref accepts it, though the
                // path alone would already be unambiguous.
                let (base, rest) = match rest.split_once(':') {
                    Some((namespace, sub)) => {
                        match namespace_named(imported.namespaces(), namespace) {
                            Some(index) => (imported.namespaces()[index].folder.clone(), sub),
                            None => return BodyDestination::BadPrefix,
                        }
                    }
                    None => (String::new(), rest),
                };
                BodyDestination::Import {
                    alias: alias.to_owned(),
                    path: join(&base, rest),
                }
            }
        };
    }
    if let Some((prefix, rest)) = target.split_once(':') {
        return match namespace_named(ctx.namespaces, prefix) {
            Some(namespace) => BodyDestination::Path(join(&ctx.namespaces[namespace].folder, rest)),
            None => BodyDestination::Skip,
        };
    }
    let base = base_of(
        ctx.ref_base,
        ctx.doc_path,
        ctx.doc_namespace,
        ctx.namespaces,
    );
    BodyDestination::Path(join(&base, target))
}

/// A leading `./` or `../` is read before any prefix, so a file name holding a colon is never
/// read as a namespace (SPC-14).
fn classify(
    written: &str,
    doc_namespace: usize,
    doc_path: &str,
    ref_base: RefBase,
    namespaces: &[Namespace],
    codes: &BTreeSet<String>,
) -> Form {
    if written.starts_with("./") || written.starts_with("../") {
        return Form::Path {
            base: base_of(ref_base, doc_path, doc_namespace, namespaces),
            rest: written.to_owned(),
        };
    }
    if let Some((alias, rest)) = written.split_once("::") {
        return Form::Import {
            alias: alias.to_owned(),
            rest: rest.to_owned(),
        };
    }
    if let Some((prefix, rest)) = written.split_once(':') {
        return match namespace_named(namespaces, prefix) {
            Some(namespace) if looks_like_key(rest) => Form::Key {
                namespace,
                key: rest.to_owned(),
            },
            Some(namespace) => Form::Path {
                base: namespaces[namespace].folder.clone(),
                rest: rest.to_owned(),
            },
            None => Form::BadPrefix,
        };
    }
    if looks_like_key(written) && codes.contains(code_of(written)) {
        return Form::Key {
            namespace: doc_namespace,
            key: written.to_owned(),
        };
    }
    Form::Path {
        base: base_of(ref_base, doc_path, doc_namespace, namespaces),
        rest: written.to_owned(),
    }
}

fn base_of(
    ref_base: RefBase,
    doc_path: &str,
    doc_namespace: usize,
    namespaces: &[Namespace],
) -> String {
    match ref_base {
        RefBase::File => folder_of(doc_path),
        RefBase::Namespace => namespaces[doc_namespace].folder.clone(),
    }
}

fn namespace_named(namespaces: &[Namespace], name: &str) -> Option<usize> {
    namespaces
        .iter()
        .position(|namespace| namespace.name == name)
}

#[expect(
    clippy::expect_used,
    reason = "each of the three calls first checks that the text has the key shape, which needs a \
              dash: `resolve_into_project` and `classify` in this file test `looks_like_key(..)` on the \
              same string in the `if` that holds the call, and `Project::mention_missing` passes the \
              part after the last `:` of a `Mention.written`, which `links::mention_shape` accepts \
              only when `looks_like_key_shape` holds for that same part"
)]
pub(crate) fn code_of(key: &str) -> &str {
    key.split_once('-')
        .map(|(code, _)| code)
        .expect("looks_like_key already found a dash")
}

fn resolve_key(namespace: usize, key: &str, index: &Index) -> Outcome {
    let path = index.key(namespace, key).ok_or(Reason::NotFound)?;
    #[expect(
        clippy::expect_used,
        reason = "`path` comes from `index.key(..)` on the same `index`; `Index::build` binds a key \
                  to a path in the same step that inserts the path's entry, removes both together \
                  for a path more than one collection matches, and `Index` has no other mutator, so \
                  a path in a key group has an entry"
    )]
    let entry = index
        .get(path)
        .expect("a key in the key index always has a matching entry");
    Ok(Resolved {
        path: path.to_owned(),
        collection: Some(entry.collection),
        via: Via::Key,
        project: None,
    })
}

/// `""` (the project folder) for a path with no folder.
fn folder_of(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => folder.to_owned(),
        None => String::new(),
    }
}

fn join(base: &str, rest: &str) -> String {
    if base.is_empty() {
        normalize(rest)
    } else {
        normalize(&format!("{base}/{rest}"))
    }
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
        });
    }
    if case_exact_file(root, path) {
        return Ok(Resolved {
            path: path.to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
        });
    }
    Err(Reason::NotFound)
}

/// Every component is compared, not only the last: a file system that folds case does so for
/// each component of a lookup.
///
/// Normalizes first because `resolve_into_project` passes an import's path here without `join`,
/// so `.` and `..` must not be searched for as names.
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

    fn namespaces() -> Vec<Namespace> {
        vec![
            Namespace {
                name: "default".to_owned(),
                folder: String::new(),
            },
            Namespace {
                name: "story-2".to_owned(),
                folder: "story-2".to_owned(),
            },
        ]
    }

    fn classify_default(written: &str, ref_base: RefBase, codes: &BTreeSet<String>) -> Form {
        classify(
            written,
            0,
            "tickets/WF-1.md",
            ref_base,
            &namespaces(),
            codes,
        )
    }

    /// `classify_body` reads neither `codes` nor `index`, so empty ones stand in.
    fn classify_body_default(target: &str, ref_base: RefBase) -> BodyDestination {
        let namespaces = namespaces();
        let codes = BTreeSet::new();
        let index = Index::default();
        let imports = BTreeMap::new();
        let ctx = Ctx {
            doc_namespace: 0,
            doc_path: "tickets/WF-1.md",
            ref_base,
            namespaces: &namespaces,
            codes: &codes,
            index: &index,
            root: Path::new("."),
            imports: &imports,
        };
        classify_body(target, &ctx)
    }

    fn code_set(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|c| c.to_string()).collect()
    }

    #[test]
    fn a_bare_key_shaped_value_whose_code_exists_is_a_key_in_the_documents_own_namespace() {
        let form = classify_default("WF-1", RefBase::File, &code_set(&["WF"]));

        assert!(matches!(form, Form::Key { namespace: 0, key } if key == "WF-1"));
    }

    #[test]
    fn a_key_shaped_value_whose_code_is_not_in_the_project_is_read_as_a_relative_path() {
        let form = classify_default("WF-1", RefBase::File, &code_set(&[]));

        assert!(matches!(form, Form::Path { rest, .. } if rest == "WF-1"));
    }

    #[test]
    fn a_double_colon_is_an_import_form_never_a_relative_path() {
        // Whether `chief` is configured is decided later, against `Ctx::imports`.
        let form = classify_default("chief::WF-5", RefBase::File, &code_set(&[]));

        assert!(
            matches!(&form, Form::Import { alias, rest } if alias == "chief" && rest == "WF-5")
        );
    }

    #[test]
    fn a_double_colon_is_an_import_form_whatever_the_project_has() {
        let form = classify(
            "chief::WF-5",
            0,
            "a.md",
            RefBase::File,
            &namespaces(),
            &code_set(&[]),
        );

        assert!(
            matches!(&form, Form::Import { alias, rest } if alias == "chief" && rest == "WF-5")
        );
    }

    #[test]
    fn a_single_colon_naming_no_sibling_namespace_is_bad_prefix() {
        let form = classify_default("nosuch:WF-5", RefBase::File, &code_set(&[]));

        assert!(matches!(form, Form::BadPrefix));
    }

    #[test]
    fn a_single_colon_naming_a_sibling_with_a_key_shaped_rest_is_a_key_in_that_namespace() {
        let form = classify_default("story-2:WF-5", RefBase::File, &code_set(&[]));

        // No code is known on purpose: a sibling key needs only a real namespace, unlike a bare
        // key (SPC-14).
        assert!(matches!(form, Form::Key { namespace: 1, key } if key == "WF-5"));
    }

    #[test]
    fn a_single_colon_naming_a_sibling_with_a_path_rest_is_a_path_from_that_namespaces_folder() {
        let form = classify_default("story-2:notes/x.md", RefBase::Namespace, &code_set(&[]));

        // `RefBase::Namespace` on purpose: a sibling path reads from that namespace's folder,
        // never by `refBase`.
        assert!(
            matches!(form, Form::Path { base, rest } if base == "story-2" && rest == "notes/x.md")
        );
    }

    #[test]
    fn a_leading_dot_slash_escapes_a_colon_that_is_really_part_of_the_path() {
        let form = classify_default("./weird:name.md", RefBase::File, &code_set(&[]));

        assert!(matches!(form, Form::Path { rest, .. } if rest == "./weird:name.md"));
    }

    #[test]
    fn a_body_link_with_no_prefix_is_a_path_never_a_key_even_when_key_shaped() {
        let form = classify_body_default("WF-1", RefBase::File);

        assert!(matches!(form, BodyDestination::Path(path) if path == "tickets/WF-1"));
    }

    #[test]
    fn a_body_link_sibling_prefix_is_a_path_from_that_namespaces_folder_never_a_key() {
        let form = classify_body_default("story-2:WF-5", RefBase::File);

        assert!(matches!(form, BodyDestination::Path(path) if path == "story-2/WF-5"));
    }

    #[test]
    fn a_body_link_double_colon_naming_no_configured_import_is_bad_prefix() {
        let form = classify_body_default("memory::precedents/x.md", RefBase::File);

        assert!(matches!(form, BodyDestination::BadPrefix));
    }

    #[test]
    fn a_body_link_single_colon_naming_no_sibling_is_skipped_as_a_url_scheme() {
        for target in ["https://example.com/a.md", "mailto:a@b.com", "file:///a.md"] {
            let form = classify_body_default(target, RefBase::File);

            assert!(matches!(form, BodyDestination::Skip), "{target}");
        }
    }

    #[test]
    fn a_body_link_unprefixed_path_joins_against_the_documents_folder_or_its_namespace() {
        let file = classify_body_default("x.md", RefBase::File);
        let namespace = classify_body_default("x.md", RefBase::Namespace);

        assert!(matches!(file, BodyDestination::Path(path) if path == "tickets/x.md"));
        assert!(matches!(namespace, BodyDestination::Path(path) if path == "x.md"));
    }

    #[test]
    fn a_body_link_leading_dot_slash_escapes_a_colon_that_is_really_part_of_the_path() {
        let form = classify_body_default("./weird:name.md", RefBase::File);

        assert!(matches!(form, BodyDestination::Path(path) if path == "tickets/weird:name.md"));
    }

    #[test]
    fn the_unprefixed_form_is_joined_against_the_documents_folder_or_its_namespace_by_ref_base() {
        let file = classify_default("x.md", RefBase::File, &code_set(&[]));
        let namespace = classify_default("x.md", RefBase::Namespace, &code_set(&[]));

        assert!(matches!(file, Form::Path { base, .. } if base == "tickets"));
        assert!(matches!(namespace, Form::Path { base, .. } if base.is_empty()));
    }

    #[test]
    fn folder_of_a_top_level_path_is_the_project_folder() {
        assert_eq!(folder_of("a.md"), "");
        assert_eq!(folder_of("tickets/a.md"), "tickets");
    }

    #[test]
    fn join_normalizes_dots_the_same_way_a_schema_reference_does() {
        assert_eq!(join("tickets", "./a.md"), "tickets/a.md");
        assert_eq!(join("tickets", "../a.md"), "a.md");
        assert_eq!(join("", "a.md"), "a.md");
    }

    #[test]
    fn target_any_or_no_target_at_all_allows_any_resolved_ref() {
        let file = Resolved {
            path: "README.md".to_owned(),
            collection: None,
            via: Via::Path,
            project: None,
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
