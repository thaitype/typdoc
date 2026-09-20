//! The shell examples read from `docs/design.md`, so the shell harness and the document
//! share one list instead of a second one kept in test code that could drift from it.
//!
//! An example is a fenced code line, or an inline code span outside a fence, whose text
//! begins with `typdoc`, with `TYPDOC_` followed by a name and `=`, or with `--` followed
//! by a letter (an argument fragment). A trailing shell comment (an unquoted `#` to the end
//! of a fenced line) is removed, since a shell never sees it either.

use std::collections::BTreeSet;

/// Every example in `design`, once each, in the order it is first seen.
pub fn examples(design: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    let mut push = |text: String| {
        if is_example(&text) && seen.insert(text.clone()) {
            order.push(text);
        }
    };

    let mut in_fence = false;
    for line in design.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            push(strip_comment(line).trim().to_owned());
        } else {
            for span in inline_spans(line) {
                push(span);
            }
        }
    }
    order
}

/// Every code span inside the paragraph that begins `**Quoting in the shell.**`, in the
/// order it appears, whatever it begins with. `examples` above finds only the spans that
/// look like an invocation; this finds every span of the one paragraph that promises how
/// quoting works, so nothing in it is left unclassified.
pub fn quoting_paragraph_spans(design: &str) -> Vec<String> {
    let marker = "**Quoting in the shell.**";
    let start = design
        .find(marker)
        .unwrap_or_else(|| panic!("the design has no paragraph starting {marker}"));
    let rest = &design[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    let paragraph = &rest[..end];
    paragraph.lines().flat_map(inline_spans).collect()
}

/// A fenced line or a code span qualifies as an example by how it begins.
fn is_example(text: &str) -> bool {
    if text.starts_with("typdoc") {
        return true;
    }
    if let Some(rest) = text.strip_prefix("--") {
        return rest.starts_with(|c: char| c.is_ascii_alphabetic());
    }
    is_env_assignment(text)
}

/// `TYPDOC_` followed by one or more `[A-Z0-9_]` and then `=`.
fn is_env_assignment(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("TYPDOC_") else {
        return false;
    };
    let name_len = rest
        .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
        .unwrap_or(rest.len());
    name_len > 0 && rest.as_bytes().get(name_len) == Some(&b'=')
}

/// The text before the first `#` that is outside single or double quotes; a shell reads
/// nothing past it.
fn strip_comment(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    for (i, c) in line.char_indices() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return &line[..i],
            _ => {}
        }
    }
    line
}

/// The text of every backtick-delimited span of `line`, assuming spans open and close on
/// the same line, which holds throughout the design.
fn inline_spans(line: &str) -> Vec<String> {
    line.split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, part)| part.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fenced_line_that_begins_with_typdoc_is_an_example() {
        let design = "```bash\ntypdoc get WF-3 --json\nnot an example\n```\n";

        assert_eq!(examples(design), vec!["typdoc get WF-3 --json".to_owned()]);
    }

    #[test]
    fn an_inline_span_that_begins_with_an_argument_fragment_is_an_example() {
        let design = "Quote it: `--namespace '*'` always.\n";

        assert_eq!(examples(design), vec!["--namespace '*'".to_owned()]);
    }

    #[test]
    fn a_span_beginning_with_three_hyphens_is_not_an_argument_fragment() {
        let design = "A block opens with `---` on its own line.\n";

        assert_eq!(examples(design), Vec::<String>::new());
    }

    #[test]
    fn an_env_assignment_needs_a_name_and_an_equals_sign() {
        let design = "Set `TYPDOC_NAMESPACE='*'` or just mention `TYPDOC_NAMESPACE`.\n";

        assert_eq!(examples(design), vec!["TYPDOC_NAMESPACE='*'".to_owned()]);
    }

    #[test]
    fn a_trailing_comment_outside_quotes_is_removed_from_a_fenced_line() {
        let design = "```bash\ntypdoc get WF-3   # a comment\n```\n";

        assert_eq!(examples(design), vec!["typdoc get WF-3".to_owned()]);
    }

    #[test]
    fn a_hash_inside_quotes_is_kept_because_it_is_not_a_comment() {
        let design = "```bash\ntypdoc set WF-3 title='a # b'\n```\n";

        assert_eq!(
            examples(design),
            vec!["typdoc set WF-3 title='a # b'".to_owned()]
        );
    }

    #[test]
    fn duplicate_text_is_kept_once_in_first_seen_order() {
        let design = "`typdoc validate` first, then `--json`, then `typdoc validate` again.\n";

        assert_eq!(
            examples(design),
            vec!["typdoc validate".to_owned(), "--json".to_owned()]
        );
    }

    #[test]
    fn a_span_inside_a_fence_is_not_read_as_an_inline_span() {
        let design = "```bash\n`typdoc get WF-3`\n```\n";

        // The whole fenced line, backticks included, is not `typdoc`-prefixed once the
        // fence rule (not the inline rule) reads it, so it is not an example here; the
        // point is that the two extraction paths never double-count one line.
        assert_eq!(examples(design), Vec::<String>::new());
    }

    #[test]
    fn the_quoting_paragraph_yields_its_thirteen_spans_in_order() {
        let design = "\
Before.

**Quoting in the shell.** Give `--where`, `--if`, `--set` and `--namespace` values to typdoc exactly as written, in single quotes: `--where 'title=Cosmos\\, or SQL'`, `--namespace '*'`, `--namespace 'chief::*'`, `TYPDOC_NAMESPACE='*'`. v1 covers sh and bash, where single quotes pass every character, `\\`, `*`, `<`, `>` and `!` included, to typdoc untouched.

After.
";

        let spans = quoting_paragraph_spans(design);

        assert_eq!(
            spans,
            vec![
                "--where",
                "--if",
                "--set",
                "--namespace",
                "--where 'title=Cosmos\\, or SQL'",
                "--namespace '*'",
                "--namespace 'chief::*'",
                "TYPDOC_NAMESPACE='*'",
                "\\",
                "*",
                "<",
                ">",
                "!",
            ]
        );
    }

    #[test]
    fn a_design_with_no_quoting_paragraph_is_refused() {
        let result = std::panic::catch_unwind(|| quoting_paragraph_spans("no such paragraph here"));

        assert!(result.is_err());
    }

    #[test]
    fn the_real_design_yields_the_examples_and_spans_it_is_known_to_hold() {
        let design = crate::fixtures::design_text();

        let all = examples(&design);
        for text in [
            "typdoc get <key|path> [--json]",
            "typdoc list --collection wayfinder,decisions --where status=open --sort status --sort updated_at:desc",
            "typdoc new WF \"Cosmos or SQL?\" --set kind=grilling --set blocked_by=WF-1",
            "--where 'title=Cosmos\\, or SQL'",
            "TYPDOC_NAMESPACE='*'",
            "TYPDOC_DIR=.chief typdoc list --namespace '*' --where status=open",
        ] {
            assert!(all.iter().any(|e| e == text), "missing example: {text}");
        }
        assert_eq!(quoting_paragraph_spans(&design).len(), 13);
    }
}
