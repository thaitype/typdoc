//! The parts of `mv` that touch no file and no index: recomputing a path ref's written form
//! after its target moves, and splicing a body link's destination in place while keeping its
//! own written form (design.md, `typdoc mv`: "each ref keeping its written form... A body link
//! keeps its own form too"). Kept apart from `project.rs`'s orchestration so each rule here has
//! its own narrow test, with no project, no index and no file system to stand up first.

use std::io;
use std::path::{Path, PathBuf};

use crate::config::{Namespace, RefBase};
use crate::document::Document;
use crate::fs::{Fs, prepare_replacement};
use crate::namespace_lock::NamespaceLock;
use crate::project::RefsReference;
use crate::validate::Finding;

/// Why a ref `mv` found still points at the old name is one of the three the design gives: held
/// by a project this one imports, which is read-only (decision 2); a plain-text mention, which
/// `mv` never rewrites; or a body link in a document whose own `body.links` rule is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnrewrittenReason {
    ImportedProject,
    Mention,
    LinksRuleOff,
}

/// One ref `mv` did not rewrite: the reference itself, in the shape `refs --reverse` already
/// gives one (the document that holds it, `field`, `written` and a position), plus why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrewrittenRef {
    pub reference: RefsReference,
    pub reason: UnrewrittenReason,
}

/// What a successful `mv` reports: the document under its new name, the refs it could not
/// rewrite and why, and what the destination's schema rejects when the move landed the document
/// somewhere its fields do not satisfy (decision 16: carried out and reported, not refused).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvReport {
    pub document: Document,
    pub unrewritten: Vec<UnrewrittenRef>,
    pub findings: Vec<Finding>,
}

/// One file whose content changes in full: the bytes to write at `path` (an absolute path on
/// disk), prepared as a temp file beside it and not renamed into place until every other content
/// change [`commit`] is given is also ready.
pub struct ContentChange {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

/// Runs every one of `changes` and then moves `from` to `to`, in the two-phase order decision 1
/// fixes: a temp file for each content change is prepared first (in the order given), and only
/// once every one of them exists does any rename happen — each content change's rename, in the
/// order given, and the document's own move last of all, since "the document itself is always
/// moved last" is the property a re-run's recovery rests on.
///
/// A failure at any point stops here and leaves every rename not yet reached exactly as it was;
/// that is decision 1's residual window, not closed by this function. `_lock` is not inspected:
/// it is here so this cannot be called without one (decision 6), the same as
/// [`crate::fs::write_atomically`]; which of a caller's possibly several held locks this is, and
/// whether it is the right one for every path touched, is the caller's property, not this
/// function's.
pub fn commit(
    fs: &dyn Fs,
    _lock: &NamespaceLock<'_>,
    changes: &[ContentChange],
    from: &Path,
    to: &Path,
) -> io::Result<()> {
    let mut prepared = Vec::with_capacity(changes.len());
    for change in changes {
        let temp = prepare_replacement(fs, _lock, &change.path, &change.bytes)?;
        prepared.push((change.path.clone(), temp));
    }
    for (path, temp) in &prepared {
        fs.rename(temp, path)?;
    }
    fs.rename(from, to)
}

/// A path relative to the project folder, made relative to `base` instead (a folder, itself
/// relative to the project folder, empty for the project root): the folders `base` and `target`
/// do not share, walked back with `..`, then the rest of `target`. This is what a written,
/// unprefixed ref (`refBase: file` or `refBase: namespace`) is recomputed against once its
/// target's path changes; the result is not necessarily the path a human would have chosen, only
/// one that reads back to `target` when joined against `base` the way `refs::resolve_one` already
/// joins one.
pub(crate) fn relative_to(base: &str, target: &str) -> String {
    let base_parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    let target_parts: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
    let mut shared = 0;
    while shared < base_parts.len()
        && shared + 1 < target_parts.len()
        && base_parts[shared] == target_parts[shared]
    {
        shared += 1;
    }
    let ups = base_parts.len() - shared;
    let mut out: Vec<&str> = Vec::with_capacity(ups + (target_parts.len() - shared));
    out.extend(std::iter::repeat_n("..", ups));
    out.extend(&target_parts[shared..]);
    if out.is_empty() {
        target_parts.last().copied().unwrap_or("").to_owned()
    } else {
        out.join("/")
    }
}

/// The namespace `path` (relative to the project folder) falls under: the one namespace when
/// there is only `default` (whose folder is empty, so it covers every path), or the namespace
/// whose folder is the path's first component, when one matches. `None` when the project has
/// several named namespaces and `path`'s first component names none of them — a file outside
/// every namespace folder (story 1's decision, reachable by a ref, kept by a write the same way).
pub(crate) fn namespace_of(namespaces: &[Namespace], path: &str) -> Option<usize> {
    if namespaces.len() == 1 && namespaces[0].folder.is_empty() {
        return Some(0);
    }
    let first = path.split('/').next().unwrap_or("");
    namespaces.iter().position(|ns| ns.folder == first)
}

/// Whether `written` is a sibling-namespace-prefixed form (`name:rest`) rather than a bare
/// relative path: the same reading `refs::classify` gives it, without needing that module's
/// `Ctx` — a leading `./` or `../` escapes a colon that is part of the path, and `::` is the
/// import form, never reached here since a ref that resolved to a document of this project was
/// never one (an import prefix routes to a different project entirely).
fn sibling_prefix<'a>(written: &'a str, namespaces: &[Namespace]) -> Option<(&'a str, &'a str)> {
    if written.starts_with("./") || written.starts_with("../") || written.contains("::") {
        return None;
    }
    let (prefix, rest) = written.split_once(':')?;
    namespaces
        .iter()
        .any(|ns| ns.name == prefix)
        .then_some((prefix, rest))
}

/// The written form a ref to the document now at `new_target` should take, keeping the category
/// (bare relative path, or sibling-namespace-prefixed) `written` already used, as design.md asks
/// ("keeping each ref's written form"). `holder_namespace`/`holder_path` are the document that
/// holds the ref; `ref_base` is its collection's own. When `written` was sibling-prefixed and
/// `new_target` no longer falls inside any namespace (it left every namespace folder), the
/// prefixed form falls back to a bare relative path, since there is no longer a namespace name to
/// prefix it with.
pub(crate) fn rewritten_path_ref(
    written: &str,
    ref_base: RefBase,
    holder_namespace: usize,
    holder_path: &str,
    namespaces: &[Namespace],
    new_target: &str,
) -> String {
    if let Some((_, _)) = sibling_prefix(written, namespaces)
        && let Some(target_ns) = namespace_of(namespaces, new_target)
    {
        let rest = relative_to(&namespaces[target_ns].folder, new_target);
        return format!("{}:{rest}", namespaces[target_ns].name);
    }
    let base = match ref_base {
        RefBase::File => folder_of(holder_path),
        RefBase::Namespace => namespaces[holder_namespace].folder.clone(),
    };
    relative_to(&base, new_target)
}

/// The folder a project-relative path sits in, or the project folder itself for a path with no
/// folder of its own. The same rule `refs::folder_of` already reads a ref's own base by (kept as
/// a second, small copy rather than made `pub(crate)` there: `refs.rs`'s copy is reached only
/// through `Ctx`, built from a document already being read, and duplicating four lines here costs
/// less than threading a `Ctx` through code that has no ref-resolution context of its own to
/// build one from).
fn folder_of(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => folder.to_owned(),
        None => String::new(),
    }
}

/// Whether `text` holds an unequal number of `(` and `)`: design.md's own condition for a body
/// link that needs `<…>` wrapping even though it holds no space and no `<`.
fn unbalanced_parens(text: &str) -> bool {
    text.chars().filter(|&c| c == '(').count() != text.chars().filter(|&c| c == ')').count()
}

/// The written form of a body link's destination once its target's path becomes `new_path`,
/// keeping the source text's own convention: a destination written `<…>` stays that way; one
/// written with a literal `%20` keeps its spaces spelled that way; and a destination written
/// plain, whose new path now holds a space, a `<` or unbalanced parentheses, is written `<…>`
/// (design.md, `typdoc mv`, the paragraph beginning "Moves a file"). Returns the text to put
/// between the parentheses, `<…>` included when it is used.
/// The written form of a body link's destination once its target's path becomes `new_path`,
/// keeping the source text's own convention: a destination written `<…>` stays that way; one
/// written with a literal `%20` keeps its spaces spelled that way; and a destination written
/// plain, whose new path now holds a space, a `<` or unbalanced parentheses, is written `<…>`
/// (design.md, `typdoc mv`, the paragraph beginning "Moves a file"). Returns the text to put
/// between the parentheses, `<…>` included when it is used.
fn rewritten_body_destination(
    was_bracketed: bool,
    was_percent_encoded: bool,
    new_path: &str,
) -> String {
    if was_bracketed {
        return format!("<{new_path}>");
    }
    if was_percent_encoded {
        return new_path.replace(' ', "%20");
    }
    if new_path.contains(' ') || new_path.contains('<') || unbalanced_parens(new_path) {
        return format!("<{new_path}>");
    }
    new_path.to_owned()
}

/// Splices the new path `new_path` into `line` in place of the old destination `old_written`,
/// found right after the `](` that begins at or after `link_col` (1-based,
/// [`crate::lines::Position`]'s own convention: the column of the link's opening `[` or `!`).
/// Whether the old destination was written `<…>` or held a literal `%20` is read from the line
/// itself, never guessed, and the new one keeps the same convention (`rewritten_body_destination`).
/// `None` when the line does not hold `](` at or after that column, or the text right after it
/// does not match `old_written` exactly (bracket-wrapped or not) — a defensive refusal rather
/// than a corrupted splice, for a line whose shape this function did not expect. The one
/// construction that does not fit: a link whose own text holds the literal three characters
/// `](`, which would be found first and is out of scope here (design.md leaves no case for it,
/// and no fixture in this story's corpus holds one).
pub(crate) fn splice_body_destination(
    line: &str,
    link_col: usize,
    old_written: &str,
    new_path: &str,
) -> Option<String> {
    let from = char_byte_offset(line, link_col.saturating_sub(1))?;
    let open = from + line[from..].find("](")?;
    let dest_start = open + 2;
    let bracketed = line[dest_start..].starts_with('<');
    let content_start = if bracketed {
        dest_start + 1
    } else {
        dest_start
    };
    let content_end = content_start + old_written.len();
    if content_end > line.len() || &line[content_start..content_end] != old_written {
        return None;
    }
    let close_end = if bracketed {
        content_end + 1
    } else {
        content_end
    };
    let percent_encoded = old_written.contains('%');
    let rendered = rewritten_body_destination(bracketed, percent_encoded, new_path);
    Some(format!(
        "{}{rendered}{}",
        &line[..dest_start],
        &line[close_end..]
    ))
}

/// The byte offset of the `chars`-th character of `line` (0-based), for a 1-based column already
/// turned into a 0-based count by the caller: [`crate::lines::Position`]'s `col` counts Unicode
/// scalar values, not bytes, so a line holding non-ASCII text before the link needs this rather
/// than indexing `line` by `chars` directly.
fn char_byte_offset(line: &str, chars: usize) -> Option<usize> {
    line.char_indices()
        .nth(chars)
        .map(|(at, _)| at)
        .or_else(|| (chars == line.chars().count()).then_some(line.len()))
}

/// The byte range of `line_number` (1-based, [`crate::lines::Position`]'s own convention) within
/// `text`, its line ending excluded, so a caller can replace exactly that range without touching
/// how the line ends. `None` when `text` has fewer lines than `line_number`.
pub(crate) fn line_span(text: &str, line_number: usize) -> Option<(usize, usize)> {
    let mut start = 0;
    for _ in 1..line_number {
        if start >= text.len() {
            return None;
        }
        start = crate::lines::next_line(text, start);
    }
    if start >= text.len() {
        return None;
    }
    let end_with_ending = crate::lines::next_line(text, start);
    let end = start
        + text[start..end_with_ending]
            .trim_end_matches(['\r', '\n'])
            .len();
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ns(entries: &[(&str, &str)]) -> Vec<Namespace> {
        entries
            .iter()
            .map(|(name, folder)| Namespace {
                name: (*name).to_owned(),
                folder: (*folder).to_owned(),
            })
            .collect()
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
    }

    #[test]
    fn relative_to_the_project_root_has_no_leading_dots() {
        assert_eq!(relative_to("notes", "top.md"), "../top.md");
    }

    #[test]
    fn namespace_of_a_single_default_namespace_covers_every_path() {
        let namespaces = ns(&[("default", "")]);
        assert_eq!(namespace_of(&namespaces, "a/b.md"), Some(0));
        assert_eq!(namespace_of(&namespaces, "b.md"), Some(0));
    }

    #[test]
    fn namespace_of_several_namespaces_matches_the_first_path_component() {
        let namespaces = ns(&[("story-1", "story-1"), ("story-2", "story-2")]);
        assert_eq!(namespace_of(&namespaces, "story-2/notes/x.md"), Some(1));
        assert_eq!(
            namespace_of(&namespaces, "elsewhere/x.md"),
            None,
            "outside every namespace folder"
        );
    }

    #[test]
    fn a_bare_written_ref_is_recomputed_as_a_bare_relative_path() {
        let namespaces = ns(&[("default", "")]);
        let new = rewritten_path_ref(
            "old.md",
            RefBase::File,
            0,
            "tickets/holder.md",
            &namespaces,
            "tickets/new.md",
        );
        assert_eq!(new, "new.md");
    }

    #[test]
    fn a_sibling_prefixed_written_ref_keeps_the_prefix_form_and_updates_it() {
        let namespaces = ns(&[("default", ""), ("story-2", "story-2")]);
        let new = rewritten_path_ref(
            "story-2:old.md",
            RefBase::File,
            0,
            "holder.md",
            &namespaces,
            "story-2/moved/new.md",
        );
        assert_eq!(new, "story-2:moved/new.md");
    }

    #[test]
    fn a_sibling_prefixed_ref_whose_target_left_every_namespace_falls_back_to_a_bare_path() {
        let namespaces = ns(&[("default", ""), ("story-2", "story-2")]);
        let new = rewritten_path_ref(
            "story-2:old.md",
            RefBase::File,
            0,
            "holder.md",
            &namespaces,
            "outside/new.md",
        );
        assert_eq!(new, "outside/new.md");
    }

    #[test]
    fn rewritten_body_destination_keeps_bracket_wrapping() {
        assert_eq!(
            rewritten_body_destination(true, false, "new path.md"),
            "<new path.md>"
        );
    }

    #[test]
    fn rewritten_body_destination_keeps_percent_encoding() {
        assert_eq!(
            rewritten_body_destination(false, true, "new path.md"),
            "new%20path.md"
        );
    }

    #[test]
    fn rewritten_body_destination_wraps_a_plain_link_only_when_the_new_path_needs_it() {
        assert_eq!(rewritten_body_destination(false, false, "new.md"), "new.md");
        assert_eq!(
            rewritten_body_destination(false, false, "new path.md"),
            "<new path.md>",
            "a space in the new path, and the old link used neither convention"
        );
        assert_eq!(
            rewritten_body_destination(false, false, "new(1.md"),
            "<new(1.md>",
            "unbalanced parentheses"
        );
    }

    #[test]
    fn splice_body_destination_replaces_a_bare_inline_link() {
        let line = "See [the note](old.md) for details.";
        let spliced = splice_body_destination(line, 5, "old.md", "new.md").unwrap();
        assert_eq!(spliced, "See [the note](new.md) for details.");
    }

    #[test]
    fn splice_body_destination_keeps_bracket_wrapping_read_from_the_line_itself() {
        let line = "[img](<old file.md>)";
        let spliced = splice_body_destination(line, 1, "old file.md", "new file.md").unwrap();
        assert_eq!(spliced, "[img](<new file.md>)");
    }

    #[test]
    fn splice_body_destination_keeps_percent_encoding_read_from_the_line_itself() {
        let line = "[img](old%20file.md)";
        // `old_written` is the raw text as authored, `%20` included — the same form
        // `BodyLink::written` carries (design.md, References: percent-encoding is never
        // decoded on the way to `written`, only `target` is).
        let spliced = splice_body_destination(line, 1, "old%20file.md", "new file.md").unwrap();
        assert_eq!(spliced, "[img](new%20file.md)");
    }

    #[test]
    fn splice_body_destination_wraps_a_plain_destination_when_the_new_path_needs_it() {
        let line = "[img](old.md)";
        let spliced = splice_body_destination(line, 1, "old.md", "new file.md").unwrap();
        assert_eq!(spliced, "[img](<new file.md>)");
    }

    #[test]
    fn splice_body_destination_refuses_a_line_that_does_not_match_what_it_expected() {
        let line = "[t](something-else.md)";
        assert_eq!(splice_body_destination(line, 1, "old.md", "new.md"), None);
    }

    #[test]
    fn line_span_finds_each_line_excluding_its_ending() {
        let text = "one\r\ntwo\nthree";
        assert_eq!(line_span(text, 1), Some((0, 3)), "{:?}", &text[0..3]);
        assert_eq!(line_span(text, 2), Some((5, 8)));
        assert_eq!(
            line_span(text, 3),
            Some((9, 14)),
            "the last line has no ending at all"
        );
        assert_eq!(line_span(text, 4), None, "there is no fourth line");
    }
}
