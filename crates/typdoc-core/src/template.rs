//! Match templates: what `match` in a collection file and an entry of `namespaces` are read as.

/// One piece of a path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Literal(String),
    /// Any run of characters within the segment, none included.
    Star,
    /// Matches only once bound to the code of a schema.
    Key(Option<String>),
}

/// One segment of a path, made of literal text, `*` and `{key}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    parts: Vec<Part>,
}

impl Segment {
    /// Reads one segment. `**` is not one: a template holds it only as a whole segment.
    pub fn parse(text: &str) -> Result<Segment, String> {
        Segment::read(text, true)
    }

    /// Reads a name with `*` in it and no placeholder: `{` and `}` are characters of the name.
    pub fn parse_glob(text: &str) -> Result<Segment, String> {
        Segment::read(text, false)
    }

    fn read(text: &str, keys: bool) -> Result<Segment, String> {
        if text.is_empty() {
            return Err("a segment is empty".to_owned());
        }
        if text == "." || text == ".." {
            return Err(format!("`{text}` is not a name of a file or a folder"));
        }
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            if let Some(after) = rest.strip_prefix("**") {
                return Err(format!(
                    "`**` stands for whole folders, so it is a segment of its own, and `{after}` follows it in `{text}`"
                ));
            } else if let Some(after) = rest.strip_prefix('*') {
                push_literal(&mut parts, &mut literal);
                parts.push(Part::Star);
                rest = after;
            } else if let Some(after) = rest.strip_prefix("{key}").filter(|_| keys) {
                push_literal(&mut parts, &mut literal);
                parts.push(Part::Key(None));
                rest = after;
            } else if keys && (c == '{' || c == '}') {
                return Err(format!(
                    "`{c}` in `{text}` is not part of a placeholder: `{{key}}` is the only one"
                ));
            } else {
                literal.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
        push_literal(&mut parts, &mut literal);
        Ok(Segment { parts })
    }

    /// Whether `name` fits. A name that starts with `.` is matched only by a segment that
    /// starts with a `.` of its own, so no wildcard reaches a hidden file or folder.
    pub fn matches(&self, name: &str) -> bool {
        let dot = name.starts_with('.');
        let starts_with_dot =
            matches!(self.parts.first(), Some(Part::Literal(l)) if l.starts_with('.'));
        if dot && !starts_with_dot {
            return false;
        }
        !name.is_empty() && fits(&self.parts, name)
    }

    /// The substring `{key}` captures from `name`, if this segment has that placeholder and
    /// `name` fits the segment as a whole; the same leading-dot rule as `matches`.
    fn capture_key(&self, name: &str) -> Option<String> {
        let dot = name.starts_with('.');
        let starts_with_dot =
            matches!(self.parts.first(), Some(Part::Literal(l)) if l.starts_with('.'));
        if (dot && !starts_with_dot) || name.is_empty() {
            return None;
        }
        fits_capture(&self.parts, name).flatten()
    }

    /// The whole segment as plain text, when it holds no `*` and no `{key}`.
    fn literal(&self) -> Option<&str> {
        match self.parts.as_slice() {
            [Part::Literal(text)] => Some(text.as_str()),
            [] => Some(""),
            _ => None,
        }
    }
}

fn push_literal(parts: &mut Vec<Part>, literal: &mut String) {
    if !literal.is_empty() {
        parts.push(Part::Literal(std::mem::take(literal)));
    }
}

fn fits(parts: &[Part], name: &str) -> bool {
    fits_capture(parts, name).is_some()
}

/// Whether `name` fits `parts` and, if it does, the substring `{key}` captured along the way:
/// `None` when `parts` holds no `{key}`, `Some` when it does. The same shape as `fits`, so a
/// change to one rule cannot drift from the other.
fn fits_capture(parts: &[Part], name: &str) -> Option<Option<String>> {
    let Some((first, rest)) = parts.split_first() else {
        return name.is_empty().then_some(None);
    };
    match first {
        Part::Literal(text) => {
            let after = name.strip_prefix(text.as_str())?;
            fits_capture(rest, after)
        }
        Part::Star => name
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(name.len()))
            .find_map(|at| fits_capture(rest, &name[at..])),
        Part::Key(None) => None,
        Part::Key(Some(code)) => {
            let numbers = name
                .strip_prefix(code.as_str())
                .and_then(|after| after.strip_prefix('-'))?;
            let digits = numbers.bytes().take_while(u8::is_ascii_digit).count();
            (1..=digits).rev().find_map(|used| {
                fits_capture(rest, &numbers[used..])
                    .map(|inner| inner.or_else(|| Some(format!("{code}-{}", &numbers[..used]))))
            })
        }
    }
}

/// One step of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// `**`: any number of folders, none included.
    Folders,
    Name(Segment),
}

/// A `match`, counted from the namespace folder: the segments of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    steps: Vec<Step>,
}

impl Template {
    pub fn parse(text: &str) -> Result<Template, String> {
        let steps = text
            .split('/')
            .map(|segment| match segment {
                "**" => Ok(Step::Folders),
                other => Segment::parse(other).map(Step::Name),
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("the match `{text}` cannot be read: {e}"))?;
        Ok(Template { steps })
    }

    /// Binds `{key}` to the code of the schema and refuses a template that does not fit it.
    pub fn bind(mut self, text: &str, code: Option<&str>) -> Result<Template, String> {
        let mut keys = 0;
        let mut wildcards = 0;
        for step in &mut self.steps {
            match step {
                Step::Folders => wildcards += 1,
                Step::Name(segment) => {
                    for part in &mut segment.parts {
                        match part {
                            Part::Key(bound) => {
                                keys += 1;
                                *bound = code.map(str::to_owned);
                            }
                            Part::Star => wildcards += 1,
                            Part::Literal(_) => {}
                        }
                    }
                }
            }
        }
        match code {
            Some(_) if keys != 1 => Err(format!(
                "the match `{text}` has {keys} placeholders `{{key}}`, and the schema has a code, so it needs exactly one"
            )),
            Some(_) if wildcards > 0 => Err(format!(
                "the match `{text}` has a wildcard, and a schema with a code takes `{{key}}` and no wildcard"
            )),
            None if keys > 0 => Err(format!(
                "the match `{text}` has `{{key}}`, and the schema has no code"
            )),
            _ => Ok(self),
        }
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Whether the last step holds `{key}`, the shape `filename.pattern` scans a folder for: a
    /// file directly beside a matched one that fits no collection there. A coded template can
    /// also hold `{key}` in an earlier step (`{key}/index.md`); that shape is not this rule's,
    /// since there is no one folder to list files in (doubt: not generalised here).
    pub fn key_in_last_step(&self) -> bool {
        matches!(
            self.steps.last(),
            Some(Step::Name(segment)) if segment.parts.iter().any(|part| matches!(part, Part::Key(_)))
        )
    }

    /// The literal folder every step before the last one names, when each is plain text with no
    /// `*` (always true of a bound coded template, since `bind` refuses a wildcard once a code
    /// is given); `None` for a template with one step, or with `**` before the last.
    pub fn literal_folder(&self) -> Option<Vec<&str>> {
        let (_, prefix) = self.steps.split_last()?;
        prefix
            .iter()
            .map(|step| match step {
                Step::Name(segment) => segment.literal(),
                Step::Folders => None,
            })
            .collect()
    }

    /// The last step's segment, for testing whether a file name fits it.
    pub fn last_segment(&self) -> Option<&Segment> {
        match self.steps.last()? {
            Step::Name(segment) => Some(segment),
            Step::Folders => None,
        }
    }

    /// The key `below` carries, counted from the namespace folder, if this template names one.
    /// A coded template has exactly one `{key}`, in exactly one step, so the component at that
    /// step is the only place it can come from; a template without a code never matches here.
    pub fn key(&self, below: &str) -> Option<String> {
        below
            .split('/')
            .zip(&self.steps)
            .find_map(|(name, step)| match step {
                Step::Name(segment) => segment.capture_key(name),
                Step::Folders => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(text: &str) -> Segment {
        Segment::parse(text).unwrap()
    }

    fn coded(text: &str) -> Template {
        Template::parse(text)
            .unwrap()
            .bind(text, Some("WF"))
            .unwrap()
    }

    fn key_segment(text: &str) -> Segment {
        match coded(text).steps().last().unwrap() {
            Step::Name(segment) => segment.clone(),
            Step::Folders => panic!("a segment was expected"),
        }
    }

    #[test]
    fn a_segment_without_a_star_is_the_whole_name_and_the_case_counts() {
        assert!(segment("note.md").matches("note.md"));
        assert!(!segment("note.md").matches("Note.md"));
        assert!(!segment("note.md").matches("note.md.bak"));
        assert!(!segment("note.md").matches("a-note.md"));
    }

    #[test]
    fn a_star_stands_for_any_run_of_characters_none_included() {
        assert!(segment("*.md").matches("note.md"));
        assert!(!segment("*.md").matches("note.txt"));
        assert!(segment("a*").matches("a"));
        assert!(segment("a*b*c").matches("a--b--c"));
        assert!(segment("a*b*c").matches("abc"));
        assert!(!segment("a*b*c").matches("acb"));
        assert!(segment("*").matches("\u{e9}"));
    }

    #[test]
    fn the_parts_around_a_star_do_not_share_characters() {
        assert!(!segment("a*a").matches("a"));
        assert!(segment("a*a").matches("aa"));
        assert!(!segment("ab*ba").matches("aba"));
    }

    #[test]
    fn a_name_that_starts_with_a_dot_is_matched_only_by_a_segment_that_starts_with_one() {
        assert!(!segment("*.md").matches(".md"));
        assert!(!segment("*.md").matches(".hidden.md"));
        assert!(!segment("*").matches(".git"));
        assert!(segment(".hidden.md").matches(".hidden.md"));
        assert!(segment(".*").matches(".git"));
        assert!(segment("*.md").matches("a.md"));
    }

    #[test]
    fn a_glob_takes_braces_as_characters_of_the_name() {
        let glob = Segment::parse_glob("{key}*").unwrap();

        assert!(glob.matches("{key}x"));
        assert!(!glob.matches("WF-3"));
        assert!(Segment::parse_glob("**").is_err());
        assert!(Segment::parse_glob("").is_err());
        assert!(Segment::parse_glob("..").is_err());
    }

    #[test]
    fn nothing_matches_an_empty_name() {
        assert!(!segment("*").matches(""));
    }

    #[test]
    fn a_key_is_the_code_a_dash_and_one_or_more_digits() {
        let key = key_segment("{key}.md");

        assert!(key.matches("WF-3.md"));
        assert!(key.matches("WF-30.md"));
        assert!(key.matches("WF-007.md"));
        assert!(!key.matches("WF-.md"));
        assert!(!key.matches("WF-3a.md"));
        assert!(!key.matches("wf-3.md"));
        assert!(!key.matches("XWF-3.md"));
        assert!(!key.matches("RFC-3.md"));
        assert!(!key.matches("WF-3"));
    }

    #[test]
    fn a_key_can_be_followed_by_text_that_starts_with_a_digit() {
        let key = key_segment("{key}1.md");

        assert!(key.matches("WF-31.md"));
        assert!(key.matches("WF-3111.md"));
        assert!(!key.matches("WF-1.md"));
    }

    #[test]
    fn a_key_that_is_not_bound_to_a_code_matches_nothing() {
        let Step::Name(segment) = Template::parse("{key}.md").unwrap().steps()[0].clone() else {
            panic!("a segment was expected");
        };

        assert!(!segment.matches("WF-3.md"));
    }

    #[test]
    fn a_segment_that_breaks_the_rules_of_a_segment_is_refused() {
        for text in [
            "", ".", "..", "a**b", "**a", "a**", "{", "}", "{other}", "{key", "a{b}",
        ] {
            assert!(Segment::parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_template_is_segments_and_whole_folder_wildcards() {
        let template = Template::parse("docs/**/*.md").unwrap();

        assert_eq!(
            template.steps(),
            [
                Step::Name(segment("docs")),
                Step::Folders,
                Step::Name(segment("*.md")),
            ]
        );
    }

    #[test]
    fn a_template_with_an_empty_or_relative_segment_is_refused() {
        for text in [
            "",
            "/a.md",
            "a/",
            "a//b.md",
            "./a.md",
            "../a.md",
            "a/../b.md",
            "a/**b/c",
        ] {
            assert!(Template::parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_coded_schema_takes_one_key_and_no_wildcard() {
        for text in ["tickets/{key}.md", "{key}.md", "{key}/index.md"] {
            assert!(
                Template::parse(text)
                    .unwrap()
                    .bind(text, Some("WF"))
                    .is_ok(),
                "{text}"
            );
        }
        for text in [
            "tickets/*.md",
            "tickets/**/{key}.md",
            "tickets/x.md",
            "{key}/{key}.md",
        ] {
            assert!(
                Template::parse(text)
                    .unwrap()
                    .bind(text, Some("WF"))
                    .is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn a_schema_without_a_code_takes_wildcards_and_no_key() {
        for text in ["*.md", "notes/**/*.md", "notes/a.md"] {
            assert!(
                Template::parse(text).unwrap().bind(text, None).is_ok(),
                "{text}"
            );
        }
        assert!(
            Template::parse("{key}.md")
                .unwrap()
                .bind("{key}.md", None)
                .is_err()
        );
    }

    #[test]
    fn a_coded_template_reads_back_the_key_a_matched_path_carries() {
        assert_eq!(
            coded("tickets/{key}.md").key("tickets/WF-3.md").as_deref(),
            Some("WF-3")
        );
        assert_eq!(coded("{key}.md").key("WF-30.md").as_deref(), Some("WF-30"));
        assert_eq!(
            coded("{key}/index.md").key("WF-3/index.md").as_deref(),
            Some("WF-3")
        );
        assert_eq!(coded("tickets/{key}.md").key("tickets/wf-3.md"), None);
        assert_eq!(coded("tickets/{key}.md").key("notes/a.md"), None);
    }

    #[test]
    fn the_literal_folder_is_every_step_before_the_last_as_plain_text() {
        assert_eq!(
            coded("tickets/{key}.md").literal_folder(),
            Some(vec!["tickets"])
        );
        assert_eq!(coded("{key}.md").literal_folder(), Some(vec![]));
        assert_eq!(coded("a/b/{key}.md").literal_folder(), Some(vec!["a", "b"]));
    }

    #[test]
    fn the_key_is_in_the_last_step_only_when_the_last_segment_holds_it() {
        assert!(coded("tickets/{key}.md").key_in_last_step());
        assert!(!coded("{key}/index.md").key_in_last_step());
    }

    #[test]
    fn a_template_with_no_code_names_no_key() {
        let uncoded = Template::parse("notes/*.md")
            .unwrap()
            .bind("notes/*.md", None)
            .unwrap();

        assert_eq!(uncoded.key("notes/a.md"), None);
    }
}
