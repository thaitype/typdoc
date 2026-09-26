//! The parts of `mv` that need no project, index or file system: a path ref's written form after
//! its target moves, and a body link's destination spliced in place, each keeping the form it
//! was written in (SPC-2).

use std::io;
use std::path::{Path, PathBuf};

use crate::config::{Namespace, RefBase};
use crate::document::Document;
use crate::fs::{Fs, prepare_replacement};
use crate::namespace_lock::NamespaceLock;
use crate::project::RefsReference;
use crate::validate::Finding;

/// Why `mv` left a ref it found pointing at the old name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnrewrittenReason {
    ImportedProject,
    Mention,
    LinksRuleOff,
}

/// A ref `mv` did not rewrite, in the shape `refs --reverse` gives one, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrewrittenRef {
    pub reference: RefsReference,
    pub reason: UnrewrittenReason,
}

/// A ref `mv` rewrote: the project-relative path of the document that holds it, its field
/// (`$body` for a body link), and its written form before and after (SPC-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewrittenRef {
    pub document: String,
    pub field: String,
    pub before: String,
    pub after: String,
}

/// What a successful `mv` reports. `findings` is what the destination's schema rejects: a move
/// onto a schema the document does not satisfy is carried out and reported, not refused (SPC-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MvReport {
    pub document: Document,
    pub rewritten: Vec<RewrittenRef>,
    pub unrewritten: Vec<UnrewrittenRef>,
    pub findings: Vec<Finding>,
}

/// A file whose content [`commit`] replaces in full; `path` is absolute.
pub struct ContentChange {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

/// Replaces the content of every one of `changes`, then moves `from` to `to` (SPC-2): a temp file
/// for every change is prepared before any rename, and the document moves last, since a re-run's
/// recovery rests on it still being at `from`.
///
/// A failure stops at once and leaves every rename not yet reached undone; that window is not
/// closed. `_lock` is not inspected: it makes a call without a lock fail to compile (SPC-10).
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

/// `target`, relative to the project folder, made relative to the folder `base`, as an
/// unprefixed ref is written. The result is not necessarily the one a person would choose, only
/// one that `refs::resolve_one` resolves back to `target`.
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

/// `None` for a file outside every namespace folder, which a ref can still reach (SPC-7).
pub(crate) fn namespace_of(namespaces: &[Namespace], path: &str) -> Option<usize> {
    if namespaces.len() == 1 && namespaces[0].folder.is_empty() {
        return Some(0);
    }
    let first = path.split('/').next().unwrap_or("");
    namespaces.iter().position(|ns| ns.folder == first)
}

/// The reading `refs::classify` gives, without its `Ctx`: a leading `./` or `../` makes a colon
/// part of the path, and `::` never reaches here, since an import prefix leads to another
/// project.
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

/// The written form a ref to the document now at `new_target` takes, keeping the kind of form
/// `written` used: key, sibling-prefixed or relative path (SPC-2). A prefixed form whose target
/// has left every namespace falls back to a relative path: there is no namespace left to name.
///
/// `key_rewrite` comes only from `mv --renumber`, the one caller that meets a key form: a plain
/// `mv` never moves a coded document, and a key never ends in `.md`, so a path form cannot equal
/// `old_key` by chance. `written` already resolves to the moved document, so a form equal to
/// `old_key` is that document's own key. A bare key is always given the target's prefix: it
/// means the holder's own namespace, and `--renumber` always moves to another one (SPC-2).
pub(crate) fn rewritten_path_ref(
    written: &str,
    ref_base: RefBase,
    holder_namespace: usize,
    holder_path: &str,
    namespaces: &[Namespace],
    new_target: &str,
    key_rewrite: Option<(&str, &str)>,
) -> String {
    if let Some((old_key, new_key)) = key_rewrite
        && let Some(target_ns) = namespace_of(namespaces, new_target)
    {
        let is_key_form = written == old_key
            || written.split_once(':').is_some_and(|(prefix, rest)| {
                rest == old_key && namespaces.iter().any(|ns| ns.name == prefix)
            });
        if is_key_form {
            return format!("{}:{new_key}", namespaces[target_ns].name);
        }
    }
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

/// A copy of `refs::folder_of`, which is reached only through a `Ctx` this code cannot build.
fn folder_of(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((folder, _)) => folder.to_owned(),
        None => String::new(),
    }
}

fn unbalanced_parens(text: &str) -> bool {
    text.chars().filter(|&c| c == '(').count() != text.chars().filter(|&c| c == ')').count()
}

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

/// Replaces `old_written` with `new_path` in the destination after the first `](` at or after
/// `link_col` (1-based: the column of the link's `[` or `!`), keeping its `<…>` or `%20` form.
/// `None`, rather than a damaged line, when the text there is not `old_written`.
///
/// A link whose own text holds `](` is not handled: that `](` is found first.
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

/// [`crate::lines::Position`]'s `col` counts characters, not bytes.
fn char_byte_offset(line: &str, chars: usize) -> Option<usize> {
    line.char_indices()
        .nth(chars)
        .map(|(at, _)| at)
        .or_else(|| (chars == line.chars().count()).then_some(line.len()))
}

/// The range excludes the line ending, so replacing it keeps how the line ends.
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
            None,
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
            None,
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
            None,
        );
        assert_eq!(new, "outside/new.md");
    }

    #[test]
    fn a_bare_key_ref_is_promoted_to_the_targets_sibling_prefix_after_renumbering() {
        let namespaces = ns(&[("story-1", "story-1"), ("story-3", "story-3")]);
        let new = rewritten_path_ref(
            "WF-5",
            RefBase::File,
            0,
            "story-1/notes/holder.md",
            &namespaces,
            "story-3/tickets/WF-1.md",
            Some(("WF-5", "WF-1")),
        );
        assert_eq!(new, "story-3:WF-1");
    }

    #[test]
    fn a_sibling_prefixed_key_ref_keeps_the_key_form_and_updates_both_the_prefix_and_the_key() {
        let namespaces = ns(&[
            ("default", ""),
            ("story-1", "story-1"),
            ("story-3", "story-3"),
        ]);
        let new = rewritten_path_ref(
            "story-1:WF-5",
            RefBase::File,
            0,
            "holder.md",
            &namespaces,
            "story-3/tickets/WF-1.md",
            Some(("WF-5", "WF-1")),
        );
        assert_eq!(
            new, "story-3:WF-1",
            "a key form, not a path form: story-3:tickets/WF-1.md would be a defect here"
        );
    }

    #[test]
    fn a_sibling_prefixed_key_ref_into_the_holders_own_namespace_keeps_the_prefix_rather_than_downgrading_to_bare()
     {
        // Kept prefixed, as each ref keeps its written form (SPC-2); a `story-1:` prefix
        // resolves inside `story-1` too.
        let namespaces = ns(&[("story-1", "story-1"), ("story-9", "story-9")]);
        let new = rewritten_path_ref(
            "story-9:WF-5",
            RefBase::File,
            0,
            "story-1/notes/holder.md",
            &namespaces,
            "story-1/tickets/WF-1.md",
            Some(("WF-5", "WF-1")),
        );
        assert_eq!(new, "story-1:WF-1");
    }

    #[test]
    fn a_path_shaped_written_ref_is_untouched_by_key_rewrite_even_when_a_move_is_also_a_renumber() {
        let namespaces = ns(&[("story-1", "story-1"), ("story-3", "story-3")]);
        let new = rewritten_path_ref(
            "tickets/WF-5.md",
            RefBase::File,
            0,
            "story-1/notes/holder.md",
            &namespaces,
            "story-3/tickets/WF-1.md",
            Some(("WF-5", "WF-1")),
        );
        assert_eq!(new, "../../story-3/tickets/WF-1.md");
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
        // `old_written` is the text as written, `%20` included, as `BodyLink::written` holds it
        // (SPC-12).
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
