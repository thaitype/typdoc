//! The parts of `mv` that need no project, index or file system: a body link's destination
//! spliced in place, keeping its `<…>` or percent-encoding, and the commit of every change. The
//! name a ref is rewritten to is written by `name::format` (SPC-18).

use std::io;
use std::path::{Path, PathBuf};

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
