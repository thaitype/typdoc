//! The body-side ref forms `body.links`, `body.anchors` and `body.mentions` read: standard
//! Markdown links (inline, image, reference-style), reference definitions, duplicate labels, and
//! text that looks like a link but the parser did not read as one. Pure parsing over the text of
//! one document: no filesystem, no index, no config. Resolving a destination against the project
//! (the same prefix rules frontmatter refs use, per the design's Body links paragraph) is
//! `refs::classify_body`; this module only says what a document's body contains and where.
//!
//! The definition scan (`definitions`, `duplicate_definitions`) reads every reference definition
//! `pulldown_cmark::Parser::reference_definitions()` finds, not only the first per label: the
//! parser itself keeps only the first, so each later one is found by blanking out (with spaces,
//! so line numbers do not move) the definitions already found and reparsing, until none are left.
//! This reuses the parser's own CommonMark rules for what a definition is (never inside code,
//! never inside a fenced or indented block) instead of a hand-written scanner for the same
//! grammar ticket 6's research already showed is not small to reproduce correctly.

use std::collections::BTreeMap;
use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use crate::frontmatter;
use crate::lines::{LineMap, Position};

fn markdown_options() -> Options {
    Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES
}

/// One checked link occurrence: an inline link, an image, or a reference-style use with a
/// definition (`[t][ref]`, `[ref][]`, `[ref]`, each checked once per occurrence — ticket 10's
/// answer: "counted in `$body`"). `written` is the destination exactly as authored (for a
/// reference-style use, the destination of the definition it resolved to, which is what actually
/// decides where the link goes); `target` and `anchor` are that destination's path and fragment,
/// percent-decoded, split at the first literal `#`. `target` is `None` when the destination has
/// no path at all (`[t](#local)`), which names the document itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyLink {
    pub written: String,
    pub target: Option<String>,
    pub anchor: Option<String>,
    pub line: usize,
    pub col: usize,
    /// Whether this occurrence is a reference-style use (`[t][ref]`, `[ref][]`, `[ref]`): its
    /// destination is checked once already, at its definition, and "the uses are not reported
    /// separately" (design's Reference definitions paragraph), so a caller building `body.links`
    /// and `body.anchors` findings checks only the occurrences where this is `false` and leaves a
    /// reference-style one to the matching `Definition` — it still belongs in `$body` (ticket 12),
    /// which is why it stays in `links` rather than being dropped here.
    pub is_reference: bool,
}

/// A reference definition (`[ref]: path`), checked once at its own position regardless of use,
/// with the number of reference-style links and images that resolved to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub written: String,
    pub target: Option<String>,
    pub anchor: Option<String>,
    pub line: usize,
    pub col: usize,
    pub uses: usize,
}

/// A label defined more than once: the later definition (CommonMark ignores it; the first stays
/// active), reported at its own position, naming the line of the definition that is still active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateDefinition {
    pub line: usize,
    pub col: usize,
    pub first_line: usize,
}

/// Text outside code that looks like a link, an image, or a reference definition, but the parser
/// did not read as one — usually an unescaped space in the destination. `inner` is the would-be
/// destination, with a trailing title already removed and percent-decoded, for the caller to
/// classify the same way a real destination is (a URL scheme, a namespace prefix, or a path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suspect {
    pub inner: String,
    pub line: usize,
    pub col: usize,
    pub definition: bool,
}

/// Everything `body.links`, `body.anchors` and the duplicate-label part of `body.links` need from
/// one document's body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BodyLinks {
    pub links: Vec<BodyLink>,
    pub definitions: Vec<Definition>,
    pub duplicate_definitions: Vec<DuplicateDefinition>,
    pub suspects: Vec<Suspect>,
}

/// A key-shaped mention found in plain body text (`body.mentions`): `written` is the token as it
/// stands, a bare key or one written with a sibling or import prefix (`story-2:WF-5`,
/// `memory::LRN-5`); the shape alone does not say which, since that needs the project's
/// namespaces, so this module only reports the token and its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    pub written: String,
    pub line: usize,
    pub col: usize,
}

/// The body of `file`, parsed once: every checked link, every reference definition (active and
/// duplicate), and every "looks like a link but is not" candidate. `Err` only when the
/// frontmatter block is never closed (`frontmatter::split`'s one failure), which a caller that
/// has already read `frontmatter.parse` clean for this document never reaches.
pub fn scan(file: &str) -> Result<BodyLinks, String> {
    let body_at = frontmatter::split(file)?.body;
    let body = &file[body_at..];
    let map = LineMap::new(file);
    let at = |offset: usize| map.position(body_at + offset);

    let options = markdown_options();
    let mut resolved_ranges: Vec<Range<usize>> = Vec::new();
    let mut code_ranges: Vec<Range<usize>> = Vec::new();
    let mut links = Vec::new();
    let mut uses: BTreeMap<usize, usize> = BTreeMap::new();

    let parser = Parser::new_ext(body, options);
    let refdefs = parser.reference_definitions();
    for (event, range) in Parser::new_ext(body, options).into_offset_iter() {
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                id,
                ..
            })
            | Event::Start(Tag::Image {
                link_type,
                dest_url,
                id,
                ..
            }) => {
                resolved_ranges.push(range.clone());
                if !checked_link_type(link_type) {
                    continue;
                }
                let position = at(range.start);
                let (target, anchor) = split_destination(&dest_url);
                let is_reference = matches!(
                    link_type,
                    LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut
                );
                links.push(BodyLink {
                    written: dest_url.into_string(),
                    target,
                    anchor,
                    line: position.line,
                    col: position.col,
                    is_reference,
                });
                if is_reference && let Some(def) = refdefs.get(id.as_ref()) {
                    *uses.entry(def.span.start).or_default() += 1;
                }
            }
            Event::Start(Tag::CodeBlock(_)) => code_ranges.push(range),
            Event::Code(_) => code_ranges.push(range),
            _ => {}
        }
    }

    let (active, duplicates) = definitions(body, options);
    let mut definition_ranges: Vec<Range<usize>> =
        active.iter().map(|(span, _)| span.clone()).collect();
    definition_ranges.extend(duplicates.iter().map(|(span, _)| span.clone()));

    let definitions = active
        .into_iter()
        .map(|(span, dest)| {
            let position = at(span.start);
            let (target, anchor) = split_destination(&dest);
            Definition {
                written: dest,
                target,
                anchor,
                line: position.line,
                col: position.col,
                uses: uses.get(&span.start).copied().unwrap_or(0),
            }
        })
        .collect();
    let duplicate_definitions = duplicates
        .into_iter()
        .map(|(span, first_start)| DuplicateDefinition {
            line: at(span.start).line,
            col: at(span.start).col,
            first_line: at(first_start).line,
        })
        .collect();

    let excluded: Vec<Range<usize>> = resolved_ranges
        .into_iter()
        .chain(code_ranges)
        .chain(definition_ranges)
        .collect();
    let suspects = suspects(body, &excluded, &at);

    Ok(BodyLinks {
        links,
        definitions,
        duplicate_definitions,
        suspects,
    })
}

/// Every key-shaped token in plain body text, outside links, images and reference definitions;
/// inside inline code per `inline_code`, inside fenced or indented code per `fenced_code`.
pub fn mentions(file: &str, inline_code: bool, fenced_code: bool) -> Result<Vec<Mention>, String> {
    let body_at = frontmatter::split(file)?.body;
    let body = &file[body_at..];
    let map = LineMap::new(file);
    let at = |offset: usize| map.position(body_at + offset);

    let mut found = Vec::new();
    let mut skip_depth = 0usize; // inside a link's or an image's own text
    let mut code_block_depth = 0usize;
    for (event, range) in Parser::new_ext(body, markdown_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => skip_depth += 1,
            Event::End(TagEnd::Link | TagEnd::Image) => skip_depth = skip_depth.saturating_sub(1),
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Text(_) if skip_depth == 0 && (code_block_depth == 0 || fenced_code) => {
                scan_mentions(&body[range.clone()], range.start, &at, &mut found);
            }
            Event::Code(_) if skip_depth == 0 && inline_code => {
                // The event's own range covers the surrounding backticks; the content itself is
                // found inside it rather than assumed to start right after a fixed-width fence.
                let raw = &body[range.clone()];
                let inner_start = raw.find(|c: char| c != '`').unwrap_or(0);
                let inner_end = raw.rfind(|c: char| c != '`').map_or(raw.len(), |i| i + 1);
                if inner_start < inner_end {
                    scan_mentions(
                        &raw[inner_start..inner_end],
                        range.start + inner_start,
                        &at,
                        &mut found,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(found)
}

/// Only the forms the design's Body links paragraph checks: inline, image and reference-style
/// (`Reference`, `Collapsed`, `Shortcut`); autolinks, email autolinks and wikilinks are URL-scheme
/// or off-by-default forms and are never checked (design: "Autolinks... are URL-scheme links and
/// are skipped"). The `*Unknown` variants (a reference with no definition) never reach here: with
/// no broken-link callback given to the parser, an undefined reference is plain text, never a
/// `Link` event (ticket 10's research), which is itself how "an undefined `[t][ref]` is not
/// reported" holds.
fn checked_link_type(link_type: LinkType) -> bool {
    matches!(
        link_type,
        LinkType::Inline | LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut
    )
}

/// `dest`, percent-decoded, split at the first literal `#` (before decoding, so a literal `%23`
/// in a path is never mistaken for the fragment separator). `None` for an empty path: `[t](#a)`
/// has no path at all, and names the document itself.
fn split_destination(dest: &str) -> (Option<String>, Option<String>) {
    let (path, anchor) = match dest.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (dest, None),
    };
    let target = (!path.is_empty()).then(|| percent_decode(path));
    (target, anchor.map(percent_decode))
}

/// A `%` not followed by two hex digits is kept as written (the same rule the design gives for a
/// heading anchor's fragment; applied here to every percent-decoded destination for one rule
/// rather than a different one for paths and another for fragments).
pub(crate) fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2]))
        {
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// A definition's own span and its destination as written.
type ActiveDefinitions = Vec<(Range<usize>, String)>;
/// A duplicate definition's own span and the byte offset the active definition of the same label
/// starts at.
type DuplicateDefinitions = Vec<(Range<usize>, usize)>;

/// Every reference definition `body` holds, active (the first of its label, what `RefDefs` keeps)
/// and duplicate (every later one): found by reparsing with what is already found blanked out
/// (spaces only, line endings kept), so a definition inside a code block never surfaces (the
/// parser itself already refuses it there) without a second, hand-written definition grammar.
fn definitions(body: &str, options: Options) -> (ActiveDefinitions, DuplicateDefinitions) {
    let first_parser = Parser::new_ext(body, options);
    let mut active: Vec<(Range<usize>, String)> = first_parser
        .reference_definitions()
        .iter()
        .map(|(_, def)| (def.span.clone(), def.dest.to_string()))
        .collect();
    active.sort_by_key(|(span, _)| span.start);

    let mut working = body.to_owned();
    for (span, _) in &active {
        blank(&mut working, span.clone());
    }
    let mut duplicates = Vec::new();
    loop {
        let parser = Parser::new_ext(&working, options);
        let refdefs_here: Vec<(String, Range<usize>)> = parser
            .reference_definitions()
            .iter()
            .map(|(label, def)| (label.to_owned(), def.span.clone()))
            .collect();
        if refdefs_here.is_empty() {
            break;
        }
        let lookup = Parser::new_ext(body, options);
        let original = lookup.reference_definitions();
        for (label, span) in refdefs_here {
            let first_start = original
                .get(&label)
                .map_or(span.start, |def| def.span.start);
            duplicates.push((span.clone(), first_start));
            blank(&mut working, span);
        }
    }
    duplicates.sort_by_key(|(span, _)| span.start);
    (active, duplicates)
}

/// Replaces `span` in `text` with spaces, line endings kept, so a reparse cannot find the same
/// definition again but every other line keeps its own line number. `span` always lies on
/// character boundaries (it comes from `pulldown_cmark`'s own byte offsets into this text), and
/// every byte in it is replaced, so the result is always valid UTF-8.
fn blank(text: &mut String, span: Range<usize>) {
    let mut bytes = std::mem::take(text).into_bytes();
    for b in &mut bytes[span] {
        if *b != b'\n' && *b != b'\r' {
            *b = b' ';
        }
    }
    *text = String::from_utf8(bytes).expect("only ascii space was written over full characters");
}

/// Text outside `excluded` that looks like `[t](inner)`, `![t](inner)` or `[label]: inner` but
/// the parser did not read as a link, an image or a definition, and whose `inner`, once a
/// trailing title is removed, ends in a file extension (design's Text that looks like a link but
/// is not).
fn suspects(
    body: &str,
    excluded: &[Range<usize>],
    at: &impl Fn(usize) -> Position,
) -> Vec<Suspect> {
    let mut found = Vec::new();
    let is_excluded = |pos: usize| excluded.iter().any(|range| range.contains(&pos));

    // Bracket-paren candidates: `[` or `![`, a `]` on the same line, immediately followed by `(`,
    // and a `)` on the same line.
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let start = if i > 0 && bytes[i - 1] == b'!' {
                i - 1
            } else {
                i
            };
            if !is_excluded(start)
                && let Some(close_bracket) = find_on_line(body, i + 1, b']')
                && body.as_bytes().get(close_bracket + 1) == Some(&b'(')
                && let Some(close_paren) = find_on_line(body, close_bracket + 2, b')')
            {
                let inner_raw = &body[close_bracket + 2..close_paren];
                if let Some(inner) = candidate_inner(inner_raw) {
                    let position = at(start);
                    found.push(Suspect {
                        inner,
                        line: position.line,
                        col: position.col,
                        definition: false,
                    });
                }
                i = close_paren + 1;
                continue;
            }
        }
        i += 1;
    }

    // Definition-lookalikes: a line, once leading spaces are trimmed, starting `[label]:`.
    let mut line_start = 0;
    while line_start < body.len() {
        let line_end = crate::lines::next_line(body, line_start);
        let line = body[line_start..line_end].trim_end_matches(['\n', '\r']);
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let start = line_start + indent;
        if !is_excluded(start)
            && let Some(rest) = trimmed.strip_prefix('[')
            && let Some(close) = rest.find(']')
            && rest.as_bytes().get(close + 1) == Some(&b':')
        {
            let inner_raw = &rest[close + 2..];
            if let Some(inner) = candidate_inner(inner_raw) {
                let position = at(start);
                found.push(Suspect {
                    inner,
                    line: position.line,
                    col: position.col,
                    definition: true,
                });
            }
        }
        line_start = line_end;
    }

    found.sort_by_key(|suspect| (suspect.line, suspect.col));
    found
}

/// `raw`, trimmed, with a trailing title removed and percent-decoded, if it ends in a file
/// extension (`.` plus one to eight ASCII letters or digits, at least one a letter) with an
/// optional `#anchor` after it; `None` otherwise (the usual case: `raw` is ordinary prose that
/// happens to contain a `[...](...)` shape, such as `[0][1]`).
fn candidate_inner(raw: &str) -> Option<String> {
    let stripped = strip_title(raw.trim());
    if stripped.is_empty() {
        return None;
    }
    let before_anchor = stripped.split('#').next().unwrap_or(stripped);
    has_extension(before_anchor).then(|| percent_decode(stripped))
}

/// A trailing title (`"…"`, `'…'` or `(…)`, preceded by whitespace or at the very start) removed
/// from `text`, per the design's own three title forms.
fn strip_title(text: &str) -> &str {
    let trimmed = text.trim_end();
    for (open, close) in [('"', '"'), ('\'', '\''), ('(', ')')] {
        if let Some(body) = trimmed.strip_suffix(close)
            && let Some(at) = body.rfind(open)
            && (body[..at].is_empty() || body[..at].ends_with(char::is_whitespace))
        {
            return body[..at].trim_end();
        }
    }
    trimmed
}

/// An extension: `.` plus one to eight ASCII letters or digits, at least one of them a letter
/// (ticket 10's decision default: `version 1.2` is not reported, `ask Mr.Smith` still is).
fn has_extension(text: &str) -> bool {
    let Some((_, ext)) = text.rsplit_once('.') else {
        return false;
    };
    let len = ext.chars().count();
    (1..=8).contains(&len)
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && ext.chars().any(|c| c.is_ascii_alphabetic())
}

/// The position of the first `target` byte at or after `from`, not crossing a line ending.
fn find_on_line(text: &str, from: usize, target: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    (from..bytes.len())
        .find(|&i| {
            if bytes[i] == target {
                true
            } else {
                bytes[i] == b'\n' || bytes[i] == b'\r'
            }
        })
        .filter(|&i| bytes[i] == target)
}

/// A run of characters a mention token may be made of: ASCII letters, digits, `-` and `:`, so a
/// maximal run captures a whole prefixed or bare token and nothing beside it (the same word
/// boundary the design's `WF-3a` and `xWF-3` counter-examples ask for: either extends the run and
/// so fails the key shape).
fn is_mention_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == ':'
}

/// Every mention-shaped token (a bare key, or one with a sibling or import prefix) in `text`,
/// `base` being its offset in the whole body.
fn scan_mentions(
    text: &str,
    base: usize,
    at: &impl Fn(usize) -> Position,
    found: &mut Vec<Mention>,
) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_mention_char(text[i..].chars().next().unwrap_or('\0')) {
            let start = i;
            while i < bytes.len() {
                let Some(c) = text[i..].chars().next() else {
                    break;
                };
                if !is_mention_char(c) {
                    break;
                }
                i += c.len_utf8();
            }
            let token = &text[start..i];
            if let Some(written) = mention_shape(token) {
                let position = at(base + start);
                found.push(Mention {
                    written: written.to_owned(),
                    line: position.line,
                    col: position.col,
                });
            }
        } else {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
}

/// Whether `token` has the shape of a mention: a bare key, or a key after a `name:` or `name::`
/// prefix (the key shape itself, `argument::looks_like_key`, is not reachable from here without
/// making this module depend on `argument`'s private item; the same regex is repeated by hand,
/// as `refs.rs`'s own tests already keep two independent copies of small shape rules apart from
/// their callers).
fn mention_shape(token: &str) -> Option<&str> {
    let key_part = match token.rfind(':') {
        Some(at) => &token[at + 1..],
        None => token,
    };
    looks_like_key_shape(key_part).then_some(token)
}

/// `^[A-Z][A-Z0-9]*-\d+$`, the same shape `argument::looks_like_key` checks.
fn looks_like_key_shape(text: &str) -> bool {
    let Some((code, digits)) = text.split_once('-') else {
        return false;
    };
    let mut chars = code.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_reads_hex_pairs_and_keeps_a_malformed_percent() {
        assert_eq!(percent_decode("my%20file.md"), "my file.md");
        assert_eq!(percent_decode("100%done"), "100%done");
        assert_eq!(percent_decode("50%"), "50%");
    }

    #[test]
    fn has_extension_needs_at_least_one_letter() {
        assert!(has_extension("x.md"));
        assert!(has_extension("x.PNG"));
        assert!(!has_extension("version 1.2"));
        assert!(has_extension("ask Mr.Smith"));
        assert!(!has_extension("no-dot"));
        assert!(!has_extension("x.123456789"));
    }
}
