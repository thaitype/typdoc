//! Names (SPC-18): one grammar reads every name typdoc is given, and its inverse writes every
//! name typdoc prints. A document's identity is its project (this one, or an import's alias), its
//! namespace and its path from the project folder; a name is that identity written from a place.
//!
//! [`resolve`] reads a name as a document, and [`format`] writes a document as a name that
//! [`resolve`] reads back as it, from the same place.

use std::collections::BTreeMap;
use std::path::Path;

use crate::argument::read_key;
use crate::config::{Namespace, RefBase};
use crate::index::Index;
use crate::project::ImportState;
use crate::refs::{Reason, Resolved, Via, WrittenSlug, resolve_path};
use crate::schema::normalize;

/// A document a name is written in: its path, its namespace, and where its collection reads a path
/// with no prefix from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InDocument<'a> {
    pub path: &'a str,
    pub namespace: usize,
    pub ref_base: RefBase,
}

impl InDocument<'_> {
    /// The folder `./` and `../` are read from: the document's own, whatever `refBase` says.
    fn here(&self) -> String {
        folder_of(self.path)
    }

    fn base(&self, namespaces: &[Namespace]) -> String {
        match self.ref_base {
            RefBase::File => folder_of(self.path),
            RefBase::Namespace => namespaces[self.namespace].folder.clone(),
        }
    }
}

/// Where a name is written.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Place<'a> {
    Ref(InDocument<'a>),
    /// A path, never a key.
    BodyLink(InDocument<'a>),
    /// A key, never a path.
    Mention {
        namespace: usize,
    },
    /// A key with no prefix is looked up in each namespace of `scope`, and a path with no prefix
    /// is from the project folder. A path on disk (`./`, an absolute path) needs the current
    /// directory and is read by the caller, before this.
    Argument {
        scope: &'a [usize],
    },
}

/// The project a name is read in: its namespaces, its index, its folder and its imports.
pub(crate) struct Scene<'a> {
    pub namespaces: &'a [Namespace],
    pub index: &'a Index,
    pub root: &'a Path,
    pub imports: &'a BTreeMap<String, ImportState>,
}

/// A name told apart from its text and its place, before anything is looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reading {
    /// A key, looked up in each of `namespaces`.
    Key {
        namespaces: Vec<usize>,
        key: String,
        slug: Option<String>,
    },
    /// A path from the project folder, joined and normalized.
    Path(String),
    /// `alias::rest`, `rest` still to be read in the import.
    Import {
        alias: String,
        rest: String,
    },
    BadPrefix,
    /// An absolute path in a ref or a body link, which is never resolved.
    Absolute,
    /// A body link that is a URL, not a ref.
    NotARef,
}

/// Why a name reads as no document: a ref's `unresolved` reasons, and what else the grammar tells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unresolved {
    Reason(Reason),
    Absolute,
    NotARef,
    /// A key with no prefix, as an argument, found in more than one namespace of the scope.
    Ambiguous(Vec<usize>),
}

/// Where a key with no prefix is looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Keys {
    /// Nowhere: a body link is always a path.
    None,
    In(Vec<usize>),
    /// A project with several namespaces reached through an import: a key names one.
    NeedsPrefix,
}

/// What a place allows, which is all [`read_with`] needs to know of it.
struct Rules {
    /// The folder a path with no prefix is read from.
    base: String,
    /// The folder `./` and `../` are read from in a document: the document's own, whatever
    /// `refBase` says (SPC-18).
    here: String,
    keys: Keys,
    body_link: bool,
    mention: bool,
    /// A ref or a body link: `./` is from the document, and an absolute path is never resolved.
    in_document: bool,
    /// The rest after an import's alias, where a second `::` names no project.
    in_import: bool,
}

impl Rules {
    fn of(place: Place, namespaces: &[Namespace]) -> Rules {
        let plain = Rules {
            base: String::new(),
            here: String::new(),
            keys: Keys::None,
            body_link: false,
            mention: false,
            in_document: false,
            in_import: false,
        };
        match place {
            Place::Ref(doc) => Rules {
                base: doc.base(namespaces),
                here: doc.here(),
                keys: Keys::In(vec![doc.namespace]),
                in_document: true,
                ..plain
            },
            Place::BodyLink(doc) => Rules {
                base: doc.base(namespaces),
                here: doc.here(),
                body_link: true,
                in_document: true,
                ..plain
            },
            Place::Mention { namespace } => Rules {
                keys: Keys::In(vec![namespace]),
                mention: true,
                ..plain
            },
            Place::Argument { scope } => Rules {
                keys: Keys::In(scope.to_vec()),
                ..plain
            },
        }
    }

    /// The rest after `alias::`, read in the imported project, from its folder.
    fn in_import(&self, namespaces: &[Namespace]) -> Rules {
        Rules {
            base: String::new(),
            here: String::new(),
            keys: match (&self.keys, namespaces.len()) {
                (Keys::None, _) => Keys::None,
                (_, 1) => Keys::In(vec![0]),
                _ => Keys::NeedsPrefix,
            },
            body_link: self.body_link,
            mention: self.mention,
            in_document: false,
            in_import: true,
        }
    }
}

/// URL schemes no namespace or alias may be named, so a body link that starts with one is a URL.
const URL_SCHEMES: [&str; 4] = ["http", "https", "mailto", "file"];

/// Reads `text` written at `place`, without looking anything up.
pub(crate) fn read(text: &str, place: Place, namespaces: &[Namespace]) -> Reading {
    read_with(text, &Rules::of(place, namespaces), namespaces)
}

/// The steps of SPC-18 in their order, the first that applies deciding. A path on disk as an
/// argument is the caller's, read before this.
fn read_with(text: &str, rules: &Rules, namespaces: &[Namespace]) -> Reading {
    if rules.body_link
        && let Some((scheme, rest)) = text.split_once(':')
        && (URL_SCHEMES.contains(&scheme) || rest.starts_with("//"))
    {
        return Reading::NotARef;
    }
    // Before any prefix, so a file whose name holds a colon is written `./a:b.md`.
    if rules.in_document && from_document(text) {
        return Reading::Path(join(&rules.here, text));
    }
    // Before any prefix, so `C:\a.md` on Windows is an absolute path, not the prefix `C`.
    if rules.in_document && (text.starts_with('/') || Path::new(text).is_absolute()) {
        return Reading::Absolute;
    }
    if let Some((alias, rest)) = text.split_once("::") {
        if rules.in_import {
            return Reading::BadPrefix;
        }
        return Reading::Import {
            alias: alias.to_owned(),
            rest: rest.to_owned(),
        };
    }
    if let Some((prefix, rest)) = text.split_once(':') {
        return match namespace_named(namespaces, prefix) {
            Some(namespace) => match key_in(rest, &Keys::In(vec![namespace]), rules) {
                Some(key) => key,
                None if rules.mention => Reading::BadPrefix,
                None => Reading::Path(join(&namespaces[namespace].folder, rest)),
            },
            // A body link to a URL with no `//`, such as `tel:123`, is not a namespace attempt;
            // one that ends in `.md` is.
            None if rules.body_link && !rest.ends_with(".md") => Reading::NotARef,
            None => Reading::BadPrefix,
        };
    }
    if let Some(key) = key_in(text, &rules.keys, rules) {
        return key;
    }
    if rules.mention {
        return Reading::BadPrefix;
    }
    Reading::Path(join(&rules.base, text))
}

/// `text` as a key where `keys` says one is read, whatever its code (SPC-18): a document is always
/// a `.md` file, so a name with the shape of a key is never a path.
fn key_in(text: &str, keys: &Keys, rules: &Rules) -> Option<Reading> {
    if rules.body_link {
        return None;
    }
    let (key, slug) = read_key(text)?;
    Some(match keys {
        Keys::None => return None,
        Keys::In(namespaces) => Reading::Key {
            namespaces: namespaces.clone(),
            key: key.to_owned(),
            slug: slug.map(str::to_owned),
        },
        Keys::NeedsPrefix => Reading::BadPrefix,
    })
}

/// Where a name points before anything there is looked up: a key or a path, in this project or in
/// an import.
pub(crate) struct Located<'s> {
    pub import: Option<Imported<'s>>,
    /// A `Reading::Key` or a `Reading::Path`; any other reading is an `Unresolved` instead.
    pub reading: Reading,
}

/// An import a name reads into: its alias in this project, and the project loaded there.
pub(crate) struct Imported<'s> {
    pub alias: String,
    pub project: &'s crate::project::Project,
}

impl Located<'_> {
    /// Looks the reading up in the project it points into: the import, or the one `scene` is.
    pub(crate) fn look_up(self, scene: &Scene) -> Result<Resolved, Unresolved> {
        match self.import {
            Some(Imported { alias, project }) => {
                let resolved = look_up(self.reading, project.index_ref(), project.root_ref())?;
                Ok(Resolved {
                    project: Some(alias),
                    ..resolved
                })
            }
            None => look_up(self.reading, scene.index, scene.root),
        }
    }
}

/// Reads `text` written at `place`, and follows an import prefix into the import, without looking
/// the key or the path up.
pub(crate) fn locate<'s>(
    text: &str,
    place: Place,
    scene: &Scene<'s>,
) -> Result<Located<'s>, Unresolved> {
    let rules = Rules::of(place, scene.namespaces);
    let (import, reading) = match read_with(text, &rules, scene.namespaces) {
        Reading::Import { alias, rest } => {
            let imported = match scene.imports.get(&alias) {
                None => return Err(Unresolved::Reason(Reason::BadPrefix)),
                Some(ImportState::Absent(absence)) => {
                    return Err(Unresolved::Reason(Reason::ImportAbsent(absence.clone())));
                }
                Some(ImportState::Loaded(imported)) => imported.as_ref(),
            };
            let namespaces = imported.namespaces();
            let reading = read_with(&rest, &rules.in_import(namespaces), namespaces);
            (
                Some(Imported {
                    alias,
                    project: imported,
                }),
                reading,
            )
        }
        reading => (None, reading),
    };
    match reading {
        Reading::Key { .. } | Reading::Path(_) => Ok(Located { import, reading }),
        Reading::Import { .. } | Reading::BadPrefix => Err(Unresolved::Reason(Reason::BadPrefix)),
        Reading::Absolute => Err(Unresolved::Absolute),
        Reading::NotARef => Err(Unresolved::NotARef),
    }
}

/// Reads and looks up `text` written at `place` in `scene`.
pub(crate) fn resolve(text: &str, place: Place, scene: &Scene) -> Result<Resolved, Unresolved> {
    locate(text, place, scene)?.look_up(scene)
}

fn look_up(reading: Reading, index: &Index, root: &Path) -> Result<Resolved, Unresolved> {
    match reading {
        Reading::Key {
            namespaces,
            key,
            slug,
        } => resolve_key_in(&namespaces, &key, slug.as_deref(), index),
        Reading::Path(path) => resolve_path(&path, index, root).map_err(Unresolved::Reason),
        Reading::Import { .. } | Reading::BadPrefix => Err(Unresolved::Reason(Reason::BadPrefix)),
        Reading::Absolute => Err(Unresolved::Absolute),
        Reading::NotARef => Err(Unresolved::NotARef),
    }
}

/// A key looked up in each of `namespaces`: none found is not found, more than one is ambiguous.
fn resolve_key_in(
    namespaces: &[usize],
    key: &str,
    slug: Option<&str>,
    index: &Index,
) -> Result<Resolved, Unresolved> {
    let found: Vec<(usize, &str)> = namespaces
        .iter()
        .filter_map(|&namespace| index.key(namespace, key).map(|path| (namespace, path)))
        .collect();
    let path = match found.as_slice() {
        [] => return Err(Unresolved::Reason(Reason::NoKey(key.to_owned()))),
        [(_, path)] => *path,
        more => {
            return Err(Unresolved::Ambiguous(
                more.iter().map(|(n, _)| *n).collect(),
            ));
        }
    };
    let entry = index
        .get(path)
        .ok_or(Unresolved::Reason(Reason::NotFound))?;
    Ok(Resolved {
        path: path.to_owned(),
        collection: Some(entry.collection),
        via: Via::Key,
        project: None,
        slug: slug.map(|written| WrittenSlug {
            key: key.to_owned(),
            written: written.to_owned(),
            current: entry.slug.clone(),
        }),
    })
}

/// What [`format`] needs of a document: its project (`None` for the one being written in), its
/// namespace there, its path from that project's folder, and its key and slug.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Identity<'a> {
    pub project: Option<&'a str>,
    pub namespace: usize,
    pub path: &'a str,
    pub key: Option<&'a str>,
    pub slug: Option<&'a str>,
}

/// How [`format`] writes a document.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Form<'a> {
    /// Its path from the project folder, the file to open.
    Path,
    /// Its portable name, read as the document from every place in the project.
    Portable,
    /// Its path from the place's base, as a path with no prefix is read in a document.
    Relative,
    /// The shape `written` has: a key or a path, with or without a prefix, `./`, a slug.
    Like(&'a str),
}

/// Writes `identity` in `form` for `place`. `namespaces` are those of the project `identity` is in:
/// an import's own for a document of an import.
pub(crate) fn format(
    identity: Identity,
    form: Form,
    place: Place,
    namespaces: &[Namespace],
) -> String {
    let alias = identity
        .project
        .map(|alias| format!("{alias}::"))
        .unwrap_or_default();
    match form {
        Form::Path => identity.path.to_owned(),
        Form::Portable => match identity.key {
            Some(key) if namespaces.len() == 1 => format!("{alias}{key}"),
            Some(key) => format!("{alias}{}:{key}", prefix_of(namespaces, identity.namespace)),
            None => format!("{alias}{}", with_prefix(identity, namespaces)),
        },
        Form::Relative => relative(identity, place, namespaces, Start::Base),
        Form::Like(written) => like(identity, written, place, namespaces),
    }
}

/// The name `written` would be for `identity`, in `written`'s own shape.
fn like(identity: Identity, written: &str, place: Place, namespaces: &[Namespace]) -> String {
    if let Some((alias, rest)) = written.split_once("::") {
        let inner = Identity {
            project: None,
            ..identity
        };
        return format!(
            "{alias}::{}",
            like_in_project(inner, rest, place, namespaces, true)
        );
    }
    like_in_project(identity, written, place, namespaces, false)
}

fn like_in_project(
    identity: Identity,
    written: &str,
    place: Place,
    namespaces: &[Namespace],
    in_import: bool,
) -> String {
    let in_document = matches!(place, Place::Ref(_) | Place::BodyLink(_));
    if in_document && !in_import && from_document(written) {
        let dot = written.starts_with("./");
        return relative(identity, place, namespaces, Start::Here { dot });
    }
    let (prefixed, rest) = match written.split_once(':') {
        Some((_, rest)) => (true, rest),
        None => (false, written),
    };
    let as_key = read_key(rest).map(|(_, slug)| slug.is_some());
    match (identity.key, as_key) {
        (Some(key), Some(had_slug)) => {
            let key = match identity.slug {
                Some(slug) if had_slug => format!("{key}-{slug}"),
                _ => key.to_owned(),
            };
            let bare = if in_import {
                namespaces.len() == 1
            } else {
                writes_from(place, identity.namespace)
            };
            if prefixed || !bare {
                format!("{}:{key}", prefix_of(namespaces, identity.namespace))
            } else {
                key
            }
        }
        _ if prefixed => with_prefix(identity, namespaces),
        _ if in_import => identity.path.to_owned(),
        _ => match place {
            Place::Argument { .. } | Place::Mention { .. } => identity.path.to_owned(),
            Place::Ref(_) | Place::BodyLink(_) => {
                relative(identity, place, namespaces, Start::Base)
            }
        },
    }
}

/// Whether a key with no prefix written at `place` is looked up in `namespace` alone.
fn writes_from(place: Place, namespace: usize) -> bool {
    match place {
        Place::Ref(doc) => doc.namespace == namespace,
        Place::Mention { namespace: own } => own == namespace,
        Place::Argument { scope } => scope == [namespace],
        Place::BodyLink(_) => false,
    }
}

/// `namespace:path-from-its-folder`, `default:` in a project with one namespace (SPC-18).
fn with_prefix(identity: Identity, namespaces: &[Namespace]) -> String {
    let folder = &namespaces[identity.namespace].folder;
    format!(
        "{}:{}",
        prefix_of(namespaces, identity.namespace),
        relative_to(folder, identity.path)
    )
}

fn prefix_of(namespaces: &[Namespace], namespace: usize) -> &str {
    if namespaces.len() == 1 {
        "default"
    } else {
        &namespaces[namespace].name
    }
}

/// Which folder [`relative`] writes a path from.
#[derive(Debug, Clone, Copy)]
enum Start {
    /// The place's base, as a path with no prefix is read.
    Base,
    /// The document's folder, as `./` and `../` are read; `dot` keeps a leading `./`.
    Here { dot: bool },
}

/// `identity`'s path from `start`. A path from the base that would climb out of it starts with
/// `../`, which is read from the document's folder: where that is not the base, a ref names the
/// document with its prefix instead (`PRN-11`), and a body link, which takes none (`PRN-6`), with
/// the path from the document. A first segment
/// holding a colon, which would read as a prefix, is written with `./` when that is the document's
/// folder, and with the namespace's prefix otherwise.
fn relative(identity: Identity, place: Place, namespaces: &[Namespace], start: Start) -> String {
    let (base, here) = match place {
        Place::Ref(doc) | Place::BodyLink(doc) => (doc.base(namespaces), doc.here()),
        Place::Mention { .. } | Place::Argument { .. } => (String::new(), String::new()),
    };
    let (from, dot) = match start {
        Start::Here { dot } => (&here, dot),
        Start::Base if relative_to(&base, identity.path).starts_with("../") => {
            if base != here
                && matches!(place, Place::Ref(_))
                && namespace_of_path(namespaces, identity.path) == Some(identity.namespace)
            {
                return with_prefix(identity, namespaces);
            }
            (&here, false)
        }
        Start::Base => (&base, false),
    };
    let relative = relative_to(from, identity.path);
    let colon = relative
        .split('/')
        .next()
        .is_some_and(|first| first.contains(':'));
    if colon && *from != here {
        return with_prefix(identity, namespaces);
    }
    if dot || colon {
        format!("./{relative}")
    } else {
        relative
    }
}

/// A namespace named exactly `name`, or `default` in a project with one namespace; never a glob or
/// a list (SPC-18).
pub(crate) fn namespace_named(namespaces: &[Namespace], name: &str) -> Option<usize> {
    namespaces
        .iter()
        .position(|namespace| namespace.name == name)
        .or_else(|| (name == "default" && namespaces.len() == 1).then_some(0))
}

/// The message for `name`, written where a namespace of this project is expected.
pub(crate) fn not_a_namespace(name: &str, namespaces: &[Namespace]) -> String {
    let mut known: Vec<&str> = namespaces.iter().map(|space| space.name.as_str()).collect();
    known.sort_unstable();
    format!(
        "`{name}` is not a namespace of this project, which has: {}",
        known.join(", ")
    )
}

/// The message for a prefix that names no namespace of this project.
pub(crate) fn not_a_prefix(name: &str, namespaces: &[Namespace]) -> String {
    format!(
        "{}; a prefix names one namespace exactly, and `--namespace` selects several",
        not_a_namespace(name, namespaces)
    )
}

/// The namespace a path from the project folder is in: the one whose folder holds it, the deepest
/// when folders nest, else one whose folder is the project folder; `None` outside every namespace
/// folder, which a ref can still reach (SPC-7).
pub(crate) fn namespace_of_path(namespaces: &[Namespace], path: &str) -> Option<usize> {
    namespaces
        .iter()
        .enumerate()
        .filter(|(_, space)| {
            !space.folder.is_empty()
                && (path == space.folder || path.starts_with(&format!("{}/", space.folder)))
        })
        .max_by_key(|(_, space)| space.folder.len())
        .or_else(|| {
            namespaces
                .iter()
                .enumerate()
                .find(|(_, space)| space.folder.is_empty())
        })
        .map(|(index, _)| index)
}

/// Whether a name in a document is read from the document's folder: `./` or `../` (SPC-18).
pub(crate) fn from_document(text: &str) -> bool {
    text.starts_with("./") || text.starts_with("../")
}

/// `""` (the project folder) for a path with no folder.
pub(crate) fn folder_of(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => folder.to_owned(),
        None => String::new(),
    }
}

pub(crate) fn join(base: &str, rest: &str) -> String {
    if base.is_empty() {
        normalize(rest)
    } else {
        normalize(&format!("{base}/{rest}"))
    }
}

/// `path` written from the folder `base`, both from the project folder: `../` for each of `base`'s
/// folders that `path` is not in.
pub(crate) fn relative_to(base: &str, path: &str) -> String {
    let base: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    let target: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let common = base.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<&str> = vec![".."; base.len() - common];
    parts.extend(&target[common..]);
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::io;
    use std::path::PathBuf;

    use super::*;
    use crate::env::{Env, ProcessStatus};
    use crate::project::Project;

    fn namespace(name: &str, folder: &str) -> Namespace {
        Namespace {
            name: name.to_owned(),
            folder: folder.to_owned(),
        }
    }

    #[test]
    fn folder_of_a_top_level_path_is_the_project_folder() {
        assert_eq!(folder_of("a.md"), "");
        assert_eq!(folder_of("tickets/a.md"), "tickets");
    }

    #[test]
    fn relative_to_the_same_folder_is_the_bare_file_name() {
        assert_eq!(relative_to("tickets", "tickets/new.md"), "new.md");
        assert_eq!(relative_to("", "new.md"), "new.md");
    }

    #[test]
    fn relative_to_a_sibling_folder_walks_up_and_back_down() {
        assert_eq!(
            relative_to("tickets/sub", "tickets/other.md"),
            "../other.md"
        );
        assert_eq!(
            relative_to("a/b/c", "a/x/y.md"),
            "../../x/y.md",
            "two folders up, from the deepest shared ancestor `a`"
        );
        assert_eq!(relative_to("notes", "top.md"), "../top.md");
    }

    #[test]
    fn join_normalizes_dots_the_same_way_a_schema_reference_does() {
        assert_eq!(join("tickets", "./a.md"), "tickets/a.md");
        assert_eq!(join("tickets", "../a.md"), "a.md");
        assert_eq!(join("", "a.md"), "a.md");
    }

    fn two() -> Vec<Namespace> {
        vec![
            namespace("story-1", "story-1"),
            namespace("story-2", "story-2"),
        ]
    }

    fn one() -> Vec<Namespace> {
        vec![namespace("default", "")]
    }

    const HOLDER: &str = "story-1/notes/a.md";

    fn holder(ref_base: RefBase) -> InDocument<'static> {
        InDocument {
            path: HOLDER,
            namespace: 0,
            ref_base,
        }
    }

    fn key(namespaces: Vec<usize>, key: &str) -> Reading {
        Reading::Key {
            namespaces,
            key: key.to_owned(),
            slug: None,
        }
    }

    fn path(text: &str) -> Reading {
        Reading::Path(text.to_owned())
    }

    #[test]
    fn a_ref_reads_each_form_by_the_steps_of_spc_18() {
        let ns = two();
        let place = Place::Ref(holder(RefBase::File));
        for (text, expected) in [
            ("WF-1", key(vec![0], "WF-1")),
            ("XX-1", key(vec![0], "XX-1")),
            ("story-2:WF-1", key(vec![1], "WF-1")),
            ("story-2:notes/x.md", path("story-2/notes/x.md")),
            (
                "story-2:story-2/notes/x.md",
                path("story-2/story-2/notes/x.md"),
            ),
            ("notes/x.md", path("story-1/notes/notes/x.md")),
            ("./x.md", path("story-1/notes/x.md")),
            ("../../story-2/notes/x.md", path("story-2/notes/x.md")),
            ("./a:b.md", path("story-1/notes/a:b.md")),
            ("/elsewhere/x.md", Reading::Absolute),
            ("/story-2:x.md", Reading::Absolute),
            (
                "C:/elsewhere/x.md",
                if cfg!(windows) {
                    Reading::Absolute
                } else {
                    Reading::BadPrefix
                },
            ),
            ("stroy-2:notes/x.md", Reading::BadPrefix),
            ("story-*:WF-1", Reading::BadPrefix),
            ("story-1,story-2:WF-1", Reading::BadPrefix),
            (
                "memory::LRN-1",
                Reading::Import {
                    alias: "memory".to_owned(),
                    rest: "LRN-1".to_owned(),
                },
            ),
        ] {
            assert_eq!(read(text, place, &ns), expected, "{text}");
        }
        assert_eq!(
            read("WF-1-some-slug", place, &ns),
            Reading::Key {
                namespaces: vec![0],
                key: "WF-1".to_owned(),
                slug: Some("some-slug".to_owned())
            }
        );
    }

    /// The rest after a key has to be a slug (SPC-17), or the name is a path.
    /// Even beside a namespace named `C`: a drive letter is not a prefix.
    #[test]
    #[cfg(windows)]
    fn a_windows_absolute_path_in_a_ref_or_a_body_link_is_absolute() {
        let namespaces = vec![namespace("C", "C"), namespace("story-2", "story-2")];
        for place in [
            Place::Ref(holder(RefBase::File)),
            Place::BodyLink(holder(RefBase::File)),
        ] {
            for text in [r"C:\elsewhere\x.md", "C:/elsewhere/x.md"] {
                assert_eq!(read(text, place, &namespaces), Reading::Absolute, "{text}");
            }
        }
    }

    #[test]
    fn a_key_followed_by_text_that_is_not_a_slug_is_a_path() {
        let place = Place::Ref(holder(RefBase::File));
        for (text, expected) in [
            ("WF-1-", "story-1/notes/WF-1-"),
            ("WF-1-a#b", "story-1/notes/WF-1-a#b"),
            ("WF-1-a b", "story-1/notes/WF-1-a b"),
            ("story-2:WF-5-a#b", "story-2/WF-5-a#b"),
        ] {
            assert_eq!(read(text, place, &two()), path(expected), "{text}");
        }
    }

    #[test]
    fn a_path_with_no_prefix_in_a_ref_is_read_from_the_namespace_folder_under_ref_base_namespace() {
        assert_eq!(
            read("notes/x.md", Place::Ref(holder(RefBase::Namespace)), &two()),
            path("story-1/notes/x.md")
        );
    }

    #[test]
    fn a_body_link_is_a_path_and_a_url_is_not_a_ref() {
        let ns = two();
        let place = Place::BodyLink(holder(RefBase::File));
        for (text, expected) in [
            ("WF-1", path("story-1/notes/WF-1")),
            ("story-2:WF-1", path("story-2/WF-1")),
            ("story-2:notes/x.md", path("story-2/notes/x.md")),
            ("stroy-2:notes/x.md", Reading::BadPrefix),
            ("https://example.com/a.md", Reading::NotARef),
            ("https://example.com/a::b.md", Reading::NotARef),
            ("mailto:someone@example.md", Reading::NotARef),
            ("tel:123", Reading::NotARef),
            ("stroy-2://host/a.md", Reading::NotARef),
            ("/elsewhere/x.md", Reading::Absolute),
        ] {
            assert_eq!(read(text, place, &ns), expected, "{text}");
        }
    }

    #[test]
    fn an_argument_reads_a_key_in_its_scope_and_a_path_from_the_project_folder() {
        let ns = two();
        let place = Place::Argument { scope: &[0, 1] };
        assert_eq!(read("WF-1", place, &ns), key(vec![0, 1], "WF-1"));
        assert_eq!(read("story-2:WF-1", place, &ns), key(vec![1], "WF-1"));
        assert_eq!(
            read("story-2:notes/x.md", place, &ns),
            path("story-2/notes/x.md")
        );
        assert_eq!(
            read("story-2/notes/x.md", place, &ns),
            path("story-2/notes/x.md")
        );
        assert_eq!(read("story-*:WF-1", place, &ns), Reading::BadPrefix);
    }

    #[test]
    fn default_names_the_one_namespace_of_a_project_with_one_whatever_its_name() {
        let place = Place::Argument { scope: &[0] };
        assert_eq!(
            read("default:notes/x.md", place, &one()),
            path("notes/x.md")
        );
        let named = [namespace("story-1", "story-1")];
        assert_eq!(
            read("default:notes/x.md", place, &named),
            path("story-1/notes/x.md")
        );
        assert_eq!(
            read("default:notes/x.md", place, &two()),
            Reading::BadPrefix
        );
    }

    #[test]
    fn a_mention_is_only_a_key() {
        let place = Place::Mention { namespace: 1 };
        assert_eq!(read("WF-1", place, &two()), key(vec![1], "WF-1"));
        assert_eq!(read("notes/x.md", place, &two()), Reading::BadPrefix);
        assert_eq!(
            read("story-2:notes/x.md", place, &two()),
            Reading::BadPrefix
        );
    }

    fn identity<'a>(namespace: usize, path: &'a str, key: Option<&'a str>) -> Identity<'a> {
        Identity {
            project: None,
            namespace,
            path,
            key,
            slug: None,
        }
    }

    #[test]
    fn the_portable_name_prefixes_the_namespace_when_there_are_several_and_the_alias_of_an_import()
    {
        let place = Place::Argument { scope: &[] };
        let coded = identity(1, "story-2/tickets/WF-1.md", Some("WF-1"));
        let uncoded = identity(1, "story-2/notes/x.md", None);
        assert_eq!(format(coded, Form::Portable, place, &two()), "story-2:WF-1");
        assert_eq!(
            format(uncoded, Form::Portable, place, &two()),
            "story-2:notes/x.md"
        );
        assert_eq!(
            format(
                identity(0, "tickets/WF-1.md", Some("WF-1")),
                Form::Portable,
                place,
                &one()
            ),
            "WF-1"
        );
        assert_eq!(
            format(
                identity(0, "notes/x.md", None),
                Form::Portable,
                place,
                &one()
            ),
            "default:notes/x.md"
        );
        let named = [namespace("story-1", "story-1")];
        assert_eq!(
            format(
                identity(0, "story-1/notes/x.md", None),
                Form::Portable,
                place,
                &named
            ),
            "default:notes/x.md"
        );
        let imported = Identity {
            project: Some("memory"),
            ..identity(0, "learn/LRN-1.md", Some("LRN-1"))
        };
        assert_eq!(
            format(imported, Form::Portable, place, &one()),
            "memory::LRN-1"
        );
        assert_eq!(
            format(uncoded, Form::Path, place, &two()),
            "story-2/notes/x.md"
        );
    }

    #[test]
    fn a_relative_name_is_written_from_the_places_base_and_guards_a_colon() {
        let uncoded = identity(1, "story-2/notes/x.md", None);
        assert_eq!(
            format(
                uncoded,
                Form::Relative,
                Place::Ref(holder(RefBase::File)),
                &two()
            ),
            "../../story-2/notes/x.md"
        );
        assert_eq!(
            format(
                uncoded,
                Form::Relative,
                Place::Ref(holder(RefBase::Namespace)),
                &two()
            ),
            // Out of the namespace folder, where no path with no prefix reaches: a ref takes the
            // prefix, and a body link the path from the document.
            "story-2:notes/x.md"
        );
        assert_eq!(
            format(
                uncoded,
                Form::Relative,
                Place::BodyLink(holder(RefBase::Namespace)),
                &two()
            ),
            "../../story-2/notes/x.md"
        );
        // `./` would be the document's folder, not the namespace's: the prefix says where.
        assert_eq!(
            format(
                identity(0, "story-1/a:b.md", None),
                Form::Relative,
                Place::BodyLink(holder(RefBase::Namespace)),
                &two()
            ),
            "story-1:a:b.md"
        );
        assert_eq!(
            format(
                identity(0, "story-1/notes/a:b.md", None),
                Form::Relative,
                Place::BodyLink(holder(RefBase::File)),
                &two()
            ),
            "./a:b.md"
        );
    }

    #[test]
    fn dot_slash_and_dot_dot_are_read_from_the_documents_folder_under_either_ref_base() {
        for ref_base in [RefBase::File, RefBase::Namespace] {
            for place in [
                Place::Ref(holder(ref_base)),
                Place::BodyLink(holder(ref_base)),
            ] {
                assert_eq!(read("./x.md", place, &two()), path("story-1/notes/x.md"));
                assert_eq!(read("../x.md", place, &two()), path("story-1/x.md"));
            }
        }
        let place = Place::Ref(holder(RefBase::Namespace));
        assert_eq!(read("x.md", place, &two()), path("story-1/x.md"));
    }

    #[test]
    fn like_keeps_the_shape_it_was_written_in() {
        let ns = two();
        let place = Place::Ref(holder(RefBase::File));
        let moved = Identity {
            slug: Some("new-slug"),
            ..identity(1, "story-2/tickets/WF-9-new-slug.md", Some("WF-9"))
        };
        for (written, expected) in [
            ("WF-1", "story-2:WF-9"),
            ("WF-1-old-slug", "story-2:WF-9-new-slug"),
            ("story-1:WF-1", "story-2:WF-9"),
            (
                "story-1:tickets/WF-1.md",
                "story-2:tickets/WF-9-new-slug.md",
            ),
            (
                "../tickets/WF-1.md",
                "../../story-2/tickets/WF-9-new-slug.md",
            ),
            ("./WF-1.md", "./../../story-2/tickets/WF-9-new-slug.md"),
        ] {
            assert_eq!(
                format(moved, Form::Like(written), place, &ns),
                expected,
                "{written}"
            );
        }
        let near = identity(0, "story-1/notes/b.md", None);
        assert_eq!(format(near, Form::Like("./a.md"), place, &ns), "./b.md");
        assert_eq!(format(near, Form::Like("a.md"), place, &ns), "b.md");
    }

    /// A slug change keeps the key and its namespace: a key written alone stays as it is, and a
    /// written slug becomes the new one, or none.
    #[test]
    fn like_under_a_slug_change_keeps_a_key_written_alone() {
        let ns = two();
        let place = Place::Ref(InDocument {
            path: "story-2/notes/a.md",
            namespace: 1,
            ref_base: RefBase::File,
        });
        let renamed = Identity {
            slug: Some("new"),
            ..identity(1, "story-2/tickets/WF-5-new.md", Some("WF-5"))
        };
        let unslugged = Identity {
            slug: None,
            ..identity(1, "story-2/tickets/WF-5.md", Some("WF-5"))
        };
        for (identity, written, expected) in [
            (renamed, "WF-5", "WF-5"),
            (renamed, "story-2:WF-5", "story-2:WF-5"),
            (renamed, "WF-5-old", "WF-5-new"),
            (renamed, "story-2:WF-5-old", "story-2:WF-5-new"),
            (unslugged, "story-2:WF-5-old", "story-2:WF-5"),
            (renamed, "../tickets/WF-5-old.md", "../tickets/WF-5-new.md"),
        ] {
            assert_eq!(
                format(identity, Form::Like(written), place, &ns),
                expected,
                "{written}"
            );
        }
    }

    /// A prefix is kept even where the key could now be written alone.
    #[test]
    fn like_keeps_a_prefix_into_the_holders_own_namespace() {
        let moved = identity(0, "story-1/tickets/WF-1.md", Some("WF-1"));
        let place = Place::Ref(holder(RefBase::File));
        assert_eq!(
            format(moved, Form::Like("story-2:WF-5"), place, &two()),
            "story-1:WF-1"
        );
    }

    struct NoEnv;

    impl Env for NoEnv {
        fn var(&self, _name: &str) -> Option<OsString> {
            None
        }

        fn current_dir(&self) -> io::Result<PathBuf> {
            Ok(PathBuf::from("/"))
        }

        fn hostname(&self) -> String {
            "fixed-host".to_owned()
        }

        fn process_status(&self, _pid: u32) -> ProcessStatus {
            ProcessStatus::Unknown
        }
    }

    /// The project `crates/typdoc/tests/name_forms.rs` characterises: namespaces `story-1` and
    /// `story-2`, notes and coded tickets, and an import `memory` with one namespace.
    fn name_forms_project() -> tempfile::TempDir {
        typdoc_testkit::scratch::tree(&[
            (
                ".typdoc/config.json",
                r#"{ "version": 1, "namespaces": ["story-*"], "imports": { "memory": "./memory" } }"#,
            ),
            (
                ".typdoc/collections/notes.json",
                r#"{ "match": "notes/**/*.md", "schema": "note.json" }"#,
            ),
            (
                ".typdoc/collections/tickets.json",
                r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
            ),
            (
                "note.json",
                r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
            ),
            (
                "ticket.json",
                r#"{ "name": "ticket", "code": "WF", "fields": { "title": { "type": "string" } } }"#,
            ),
            ("memory/.typdoc/config.json", r#"{ "version": 1 }"#),
            (
                "memory/.typdoc/collections/notes.json",
                r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
            ),
            (
                "memory/.typdoc/collections/learn.json",
                r#"{ "match": "learn/{key}.md", "schema": "learn.json" }"#,
            ),
            (
                "memory/note.json",
                r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
            ),
            (
                "memory/learn.json",
                r#"{ "name": "learn", "code": "LRN", "fields": { "title": { "type": "string" } } }"#,
            ),
            ("story-1/notes/a.md", "---\ntitle: A\n---\n"),
            ("story-1/notes/x.md", "---\ntitle: X\n---\n"),
            ("story-1/notes/sub/deep.md", "---\ntitle: deep\n---\n"),
            ("story-2/notes/x.md", "---\ntitle: X\n---\n"),
            ("story-1/tickets/WF-1.md", "---\ntitle: one\n---\n"),
            ("story-2/tickets/WF-1-a-slug.md", "---\ntitle: one\n---\n"),
            ("memory/notes/y.md", "---\ntitle: Y\n---\n"),
            ("memory/learn/LRN-1.md", "---\ntitle: L\n---\n"),
        ])
    }

    /// Projects of `fixtures/valid` that do not load on their own, on purpose: `nowhere-nested` is
    /// invalid, to show that imports of imports are never followed, and `memory` imports it, so it
    /// loads only as an import, where the round trip reaches it.
    const NOT_LOADED_ALONE: [&str; 2] = ["imports/memory", "imports/memory/nowhere-nested"];

    /// Every project of `fixtures/valid`, and the name-forms project, by its folder. A folder
    /// that holds a project and does not load is a failure, not a project skipped.
    fn projects(extra: &Path) -> (Vec<(PathBuf, Project)>, Vec<String>) {
        let mut roots = vec![extra.to_path_buf()];
        let mut stack = vec![typdoc_testkit::fixtures::path("valid")];
        while let Some(dir) = stack.pop() {
            if dir.join(".typdoc").join("config.json").is_file() {
                roots.push(dir.clone());
            }
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let hidden = path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with('.'));
                if path.is_dir() && !hidden {
                    stack.push(path);
                }
            }
        }
        roots.sort();
        let mut loaded = Vec::new();
        let mut failures = Vec::new();
        let valid = typdoc_testkit::fixtures::path("valid");
        for root in roots {
            let known = root.strip_prefix(&valid).is_ok_and(|relative| {
                NOT_LOADED_ALONE
                    .iter()
                    .any(|known| relative == Path::new(known))
            });
            match (Project::load(&root, &NoEnv), known) {
                (Ok(project), false) => loaded.push((root, project)),
                (Err(error), false) => {
                    failures.push(format!("{} does not load: {error}", root.display()));
                }
                (Ok(_), true) => failures.push(format!(
                    "{} loads, but is listed as a project that does not load alone",
                    root.display()
                )),
                (Err(_), true) => {}
            }
        }
        (loaded, failures)
    }

    #[test]
    fn every_name_typdoc_writes_reads_back_as_its_document_from_every_place() {
        let extra = name_forms_project();
        let (projects, mut failures) = projects(extra.path());
        let mut checked = 0;
        for (root, project) in &projects {
            let scene = project.scene();
            let namespaces = scene.namespaces;
            let every: Vec<usize> = (0..namespaces.len()).collect();
            let each: Vec<[usize; 1]> = (0..namespaces.len()).map(|n| [n]).collect();
            let documents: Vec<(&str, usize, Option<&str>, Option<&str>)> = scene
                .index
                .iter()
                .map(|(path, entry)| {
                    (
                        path,
                        entry.namespace,
                        entry.key.as_deref(),
                        entry.slug.as_deref(),
                    )
                })
                .collect();
            let mut targets: Vec<(Identity, &[Namespace])> = documents
                .iter()
                .map(|&(path, namespace, key, slug)| {
                    (
                        Identity {
                            project: None,
                            namespace,
                            path,
                            key,
                            slug,
                        },
                        namespaces,
                    )
                })
                .collect();
            for (alias, state) in scene.imports {
                if let ImportState::Loaded(imported) = state {
                    for (path, entry) in imported.index_ref().iter() {
                        targets.push((
                            Identity {
                                project: Some(alias.as_str()),
                                namespace: entry.namespace,
                                path,
                                key: entry.key.as_deref(),
                                slug: entry.slug.as_deref(),
                            },
                            imported.namespaces(),
                        ));
                    }
                }
            }
            let mut places: Vec<Place> = vec![Place::Argument { scope: &every }];
            places.extend(each.iter().map(|scope| Place::Argument { scope }));
            for &(path, namespace, _, _) in &documents {
                for ref_base in [RefBase::File, RefBase::Namespace] {
                    let doc = InDocument {
                        path,
                        namespace,
                        ref_base,
                    };
                    places.push(Place::Ref(doc));
                    places.push(Place::BodyLink(doc));
                }
            }
            for &(identity, own_namespaces) in &targets {
                for &place in &places {
                    let is_link = matches!(place, Place::BodyLink(_));
                    let mut written = Vec::new();
                    if !is_link || identity.key.is_none() {
                        written.push(format(identity, Form::Portable, place, own_namespaces));
                    }
                    if identity.project.is_none() {
                        let native = match place {
                            Place::Argument { .. } => Form::Path,
                            _ => Form::Relative,
                        };
                        written.push(format(identity, native, place, own_namespaces));
                    }
                    let likes: Vec<String> = written
                        .iter()
                        .map(|name| format(identity, Form::Like(name), place, own_namespaces))
                        .collect();
                    written.extend(likes);
                    for name in written {
                        checked += 1;
                        match resolve(&name, place, &scene) {
                            Ok(resolved)
                                if resolved.path == identity.path
                                    && resolved.project.as_deref() == identity.project => {}
                            other => failures.push(format!(
                                "{}: {} as `{name}` at {place:?}: {other:?}",
                                root.display(),
                                identity.path
                            )),
                        }
                    }
                }
            }
        }
        assert!(checked > 5000, "only {checked} names were checked");
        assert!(
            failures.is_empty(),
            "{} of {checked} failed:\n{}",
            failures.len(),
            failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
