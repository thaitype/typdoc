//! Match templates: what `match` in a collection file and an entry of `namespaces` are read as.

/// A collection's `slug`: which form of a coded file name it expects (SPC-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlugMode {
    #[default]
    Optional,
    Required,
    None,
}

/// How a coded file name stands against its collection's `slug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameState {
    Expected,
    /// No slug under `required`, or a slug under `none`.
    UnexpectedForm,
    /// The text after the key's `-` is empty or holds a character a slug excludes.
    InvalidSlug,
}

/// What a coded template reads from the name of one of its documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName {
    pub key: String,
    pub slug: Option<String>,
    pub state: NameState,
}

/// Whether `slug` may follow a key in a file name: not empty, and no whitespace, `/`, `#` or `:`
/// (SPC-17).
pub fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '/' | '#' | ':'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Bound {
    code: String,
    /// `false` only for a template whose own text after `{key}` starts with a digit or `-`,
    /// bound under `none`: such a template never looks for a slug (SPC-17).
    reads_slug: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Literal(String),
    /// Any run of characters within the segment, none included.
    Star,
    /// Matches only once bound to the code of a schema.
    Key(Option<Bound>),
}

/// What `{key}` captured from a name that fits a segment.
#[derive(Debug, Default)]
struct Capture {
    key: Option<String>,
    slug: Option<String>,
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

    /// Whether `name` fits, as the name of a file: a leading `.` is nothing special here,
    /// because a file is named by the template that reaches it rather than found by walking
    /// into it. A folder is asked about with `matches_folder` instead.
    pub fn matches(&self, name: &str) -> bool {
        !name.is_empty() && fits(&self.parts, name)
    }

    /// Whether `name` fits, as the name of a folder to enter. A folder whose name begins with
    /// `.` is entered only by a segment that is plain text: naming a folder is saying it is
    /// wanted, while a segment holding a wildcard is saying "whatever is here".
    pub fn matches_folder(&self, name: &str) -> bool {
        if name.starts_with('.') && self.literal().is_none() {
            return false;
        }
        self.matches(name)
    }

    /// The substring `{key}` captures from `name`, if this segment has that placeholder and
    /// `name` fits the segment as a whole.
    fn capture_key(&self, name: &str) -> Option<Capture> {
        if name.is_empty() {
            return None;
        }
        fits_capture(&self.parts, name).filter(|capture| capture.key.is_some())
    }

    /// Whether this segment's `{key}` reads a slug after it; `false` too when it has none.
    fn reads_slug(&self) -> bool {
        self.parts
            .iter()
            .any(|part| matches!(part, Part::Key(Some(bound)) if bound.reads_slug))
    }

    /// The whole segment as plain text, when it holds no `*` and no `{key}`.
    fn literal(&self) -> Option<&str> {
        match self.parts.as_slice() {
            [Part::Literal(text)] => Some(text.as_str()),
            [] => Some(""),
            _ => None,
        }
    }

    /// The inverse of `capture_key`, for `typdoc new` to name the file of a key it issued.
    /// `None` for `*` or an unbound `{key}`, which a bound coded template never holds.
    fn render(&self, key: &str, slug: Option<&str>) -> Option<String> {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(text) => out.push_str(text),
                Part::Star | Part::Key(None) => return None,
                Part::Key(Some(_)) => {
                    out.push_str(key);
                    if let Some(slug) = slug {
                        out.push('-');
                        out.push_str(slug);
                    }
                }
            }
        }
        Some(out)
    }

    /// The text right after `{key}`, when this segment holds `{key}` and text follows it.
    fn text_after_key(&self) -> Option<&str> {
        let at = self
            .parts
            .iter()
            .position(|part| matches!(part, Part::Key(_)))?;
        match self.parts.get(at + 1)? {
            Part::Literal(text) => Some(text),
            Part::Star | Part::Key(_) => None,
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

/// `None` when `name` does not fit; otherwise what `{key}` captured, if `parts` has one.
/// `fits` is built on this, so the two cannot drift apart.
fn fits_capture(parts: &[Part], name: &str) -> Option<Capture> {
    let Some((first, rest)) = parts.split_first() else {
        return name.is_empty().then(Capture::default);
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
        Part::Key(Some(bound)) => {
            let code = &bound.code;
            let numbers = name
                .strip_prefix(code.as_str())
                .and_then(|after| after.strip_prefix('-'))?;
            let digits = numbers.bytes().take_while(u8::is_ascii_digit).count();
            if !bound.reads_slug {
                return (1..=digits).rev().find_map(|used| {
                    fits_capture(rest, &numbers[used..]).map(|inner| Capture {
                        key: inner
                            .key
                            .or_else(|| Some(format!("{code}-{}", &numbers[..used]))),
                        slug: inner.slug,
                    })
                });
            }
            // The number is every digit (SPC-17), so `WF-10` is never `WF-1` and a slug.
            if digits == 0 {
                return None;
            }
            let key = format!("{code}-{}", &numbers[..digits]);
            let after = &numbers[digits..];
            if fits_capture(rest, after).is_some() {
                return Some(Capture {
                    key: Some(key),
                    slug: None,
                });
            }
            let slugged = after.strip_prefix('-')?;
            slugged
                .char_indices()
                .map(|(at, _)| at)
                .chain(std::iter::once(slugged.len()))
                .rev()
                .find_map(|end| {
                    fits_capture(rest, &slugged[end..]).map(|_| Capture {
                        key: Some(key.clone()),
                        slug: Some(slugged[..end].to_owned()),
                    })
                })
        }
    }
}

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
    slug: SlugMode,
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
        Ok(Template {
            steps,
            slug: SlugMode::default(),
        })
    }

    /// The collection's `slug` this template was bound under.
    pub fn slug_mode(&self) -> SlugMode {
        self.slug
    }

    /// Binds `{key}` to the code of the schema and refuses a template that does not fit it, or
    /// that cannot tell a slug from its own text under `slug` (SPC-17).
    pub fn bind(
        mut self,
        text: &str,
        code: Option<&str>,
        slug: SlugMode,
    ) -> Result<Template, String> {
        let after_key = self.steps.iter().find_map(|step| match step {
            Step::Name(segment) => segment
                .text_after_key()
                .and_then(|text| text.chars().next()),
            Step::Folders => None,
        });
        let ambiguous = after_key.filter(|c| c.is_ascii_digit() || *c == '-');
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
                                *bound = code.map(|code| Bound {
                                    code: code.to_owned(),
                                    reads_slug: ambiguous.is_none(),
                                });
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
            Some(_) => match ambiguous {
                Some(c) if slug != SlugMode::None => Err(format!(
                    "the match `{text}` has `{c}` right after `{{key}}`, so a slug after the key \
                     cannot be told apart from the template's own text: set `slug` to `none` in \
                     this collection file to read these names without a slug"
                )),
                _ => {
                    self.slug = slug;
                    Ok(self)
                }
            },
            None => Ok(self),
        }
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Whether the last step holds `{key}`, the shape `filename.pattern` scans a folder for. A
    /// `{key}` in an earlier step (`{key}/index.md`) is not this rule's: there is no one folder
    /// to list.
    pub fn key_in_last_step(&self) -> bool {
        matches!(
            self.steps.last(),
            Some(Step::Name(segment)) if segment.parts.iter().any(|part| matches!(part, Part::Key(_)))
        )
    }

    /// The folder the steps before the last name, when each is plain text, as in every bound
    /// coded template; `None` for a template of one step.
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

    /// The name of every folder this template writes out as plain text, from every step but the
    /// last, which names a file. A step that is `**`, or a segment with a wildcard in it, names
    /// no folder and is passed over, while the plain-text steps after it still count: `.agents`
    /// in `**/.agents/*.md` is a folder the project has written out, wherever it sits.
    ///
    /// It is what decides whether a walk that is not itself a template, such as the audit's own,
    /// enters a folder whose name begins with `.`. That walk has no template position to be at,
    /// so it asks by name: a name the project wrote out is a folder the project said it wants,
    /// which is the whole reason a plain-text segment reaches a folder a wildcard does not.
    pub fn literal_folder_names(&self) -> Vec<&str> {
        let Some((_, folders)) = self.steps.split_last() else {
            return Vec::new();
        };
        folders
            .iter()
            .filter_map(|step| match step {
                Step::Name(segment) => segment.literal(),
                Step::Folders => None,
            })
            .collect()
    }

    pub fn last_segment(&self) -> Option<&Segment> {
        match self.steps.last()? {
            Step::Name(segment) => Some(segment),
            Step::Folders => None,
        }
    }

    /// The key `below` carries, counted from the namespace folder, with its slug and how its
    /// name stands against the collection's `slug`, if this template names one. A coded template
    /// has exactly one `{key}`, in exactly one step, so the component at that step is the only
    /// place it can come from; a template without a code never matches here.
    pub fn read(&self, below: &str) -> Option<FileName> {
        let (capture, reads_slug) =
            below
                .split('/')
                .zip(&self.steps)
                .find_map(|(name, step)| match step {
                    Step::Name(segment) => segment
                        .capture_key(name)
                        .map(|capture| (capture, segment.reads_slug())),
                    Step::Folders => None,
                })?;
        let key = capture.key?;
        let slug = capture.slug;
        let state = match (&slug, self.slug) {
            _ if !reads_slug => NameState::Expected,
            (Some(_), SlugMode::None) | (None, SlugMode::Required) => NameState::UnexpectedForm,
            (Some(slug), _) if !valid_slug(slug) => NameState::InvalidSlug,
            _ => NameState::Expected,
        };
        Some(FileName { key, slug, state })
    }

    /// Whether `below`, counted from the namespace folder, fits the whole template. Read as
    /// [`crate::index::Index::build`]'s walk reads it, but against a path in hand: `mv`'s
    /// destination may not exist yet, so there is nothing to walk.
    pub(crate) fn matches_path(&self, below: &str) -> bool {
        let components: Vec<&str> = if below.is_empty() {
            Vec::new()
        } else {
            below.split('/').collect()
        };
        fits_steps(&self.steps, &components)
    }

    /// The inverse of [`Template::read`], for `typdoc new` to name the file of a key it issued.
    /// `None` for `**`, `*` or an unbound `{key}`, none of which a bound coded template holds.
    pub fn render(&self, key: &str, slug: Option<&str>) -> Option<String> {
        let mut parts = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            match step {
                Step::Folders => return None,
                Step::Name(segment) => parts.push(segment.render(key, slug)?),
            }
        }
        Some(parts.join("/"))
    }
}

/// Recursive, so that `**` can backtrack over how many components it takes.
fn fits_steps(steps: &[Step], components: &[&str]) -> bool {
    let Some((step, rest_steps)) = steps.split_first() else {
        return components.is_empty();
    };
    match step {
        Step::Folders => {
            (0..=components.len()).any(|taken| fits_steps(rest_steps, &components[taken..]))
        }
        Step::Name(segment) => match components.split_first() {
            None => false,
            Some((first, rest_components)) => {
                let fits = if rest_steps.is_empty() {
                    segment.matches(first)
                } else {
                    segment.matches_folder(first)
                };
                fits && fits_steps(rest_steps, rest_components)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(text: &str) -> Segment {
        Segment::parse(text).unwrap()
    }

    fn coded(text: &str) -> Template {
        bound(text, SlugMode::Optional)
    }

    fn bound(text: &str, slug: SlugMode) -> Template {
        Template::parse(text)
            .unwrap()
            .bind(text, Some("WF"), slug)
            .unwrap()
    }

    /// Bound under `none`, the one mode a template of every shape binds in, so that the tests of
    /// a bare segment can name any template.
    fn key_segment(text: &str) -> Segment {
        match bound(text, SlugMode::None).steps().last().unwrap() {
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
    fn a_file_name_that_starts_with_a_dot_is_matched_like_any_other() {
        assert!(segment("*.md").matches(".md"));
        assert!(segment("*.md").matches(".hidden.md"));
        assert!(segment("*").matches(".git"));
        assert!(segment(".hidden.md").matches(".hidden.md"));
        assert!(segment("*.md").matches("a.md"));
        assert!(!segment("*.md").matches(".hidden.txt"));
    }

    #[test]
    fn a_folder_whose_name_starts_with_a_dot_is_entered_only_by_a_segment_of_plain_text() {
        assert!(!segment("*").matches_folder(".git"));
        assert!(!segment(".*").matches_folder(".git"));
        assert!(!segment(".g*t").matches_folder(".git"));
        assert!(segment(".git").matches_folder(".git"));
        assert!(segment("*").matches_folder("git"));
        assert!(!segment("*").matches_folder(""));
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
                    .bind(text, Some("WF"), SlugMode::Optional)
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
                    .bind(text, Some("WF"), SlugMode::Optional)
                    .is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn a_schema_without_a_code_takes_wildcards_and_no_key() {
        for text in ["*.md", "notes/**/*.md", "notes/a.md"] {
            assert!(
                Template::parse(text)
                    .unwrap()
                    .bind(text, None, SlugMode::Optional)
                    .is_ok(),
                "{text}"
            );
        }
        assert!(
            Template::parse("{key}.md")
                .unwrap()
                .bind("{key}.md", None, SlugMode::Optional)
                .is_err()
        );
    }

    #[test]
    fn a_coded_template_reads_back_the_key_a_matched_path_carries() {
        assert_eq!(
            coded("tickets/{key}.md")
                .read("tickets/WF-3.md")
                .map(|name| name.key)
                .as_deref(),
            Some("WF-3")
        );
        assert_eq!(
            coded("{key}.md")
                .read("WF-30.md")
                .map(|name| name.key)
                .as_deref(),
            Some("WF-30")
        );
        assert_eq!(
            coded("{key}/index.md")
                .read("WF-3/index.md")
                .map(|name| name.key)
                .as_deref(),
            Some("WF-3")
        );
        assert_eq!(
            coded("tickets/{key}.md")
                .read("tickets/wf-3.md")
                .map(|name| name.key),
            None
        );
        assert_eq!(
            coded("tickets/{key}.md")
                .read("notes/a.md")
                .map(|name| name.key),
            None
        );
    }

    #[test]
    fn render_names_the_file_a_key_belongs_at_and_is_the_inverse_of_key() {
        for (text, key) in [
            ("tickets/{key}.md", "WF-3"),
            ("{key}.md", "WF-30"),
            ("{key}/index.md", "WF-3"),
        ] {
            let template = coded(text);
            let rendered = template.render(key, None).unwrap();

            assert_eq!(
                template.read(&rendered).map(|name| name.key).as_deref(),
                Some(key),
                "{text}"
            );
        }

        assert_eq!(
            coded("tickets/{key}.md").render("WF-3", None),
            Some("tickets/WF-3.md".to_owned())
        );
    }

    #[test]
    fn render_is_none_for_a_wildcard_or_a_folders_step() {
        let uncoded = Template::parse("notes/*.md")
            .unwrap()
            .bind("notes/*.md", None, SlugMode::Optional)
            .unwrap();
        assert_eq!(uncoded.render("anything", None), None);

        let with_folders = Template::parse("**/{key}.md").unwrap();
        assert_eq!(with_folders.render("WF-3", None), None);
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
    fn the_plain_text_folder_names_are_every_step_but_the_last_that_holds_no_wildcard() {
        for (text, names) in [
            (".agents/notes/*.md", vec![".agents", "notes"]),
            (".agents/**/*.md", vec![".agents"]),
            ("**/.agents/*.md", vec![".agents"]),
            ("*/.agents/a.md", vec![".agents"]),
            ("a*/b/c.md", vec!["b"]),
            (".agents.md", vec![]),
            ("**/*.md", vec![]),
        ] {
            assert_eq!(
                Template::parse(text).unwrap().literal_folder_names(),
                names,
                "{text}"
            );
        }
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
            .bind("notes/*.md", None, SlugMode::Optional)
            .unwrap();

        assert_eq!(uncoded.read("notes/a.md").map(|name| name.key), None);
    }

    fn uncoded(text: &str) -> Template {
        Template::parse(text)
            .unwrap()
            .bind(text, None, SlugMode::Optional)
            .unwrap()
    }

    #[test]
    fn matches_path_fits_a_plain_glob_and_refuses_a_different_folder_or_extension() {
        let t = uncoded("notes/*.md");

        assert!(t.matches_path("notes/a.md"));
        assert!(!t.matches_path("other/a.md"), "wrong folder");
        assert!(!t.matches_path("notes/a.txt"), "wrong extension");
        assert!(!t.matches_path("a.md"), "missing the literal folder");
        assert!(!t.matches_path("notes/sub/a.md"), "one step too many");
    }

    #[test]
    fn matches_path_lets_double_star_consume_any_number_of_folders_including_none() {
        let t = uncoded("**/*.md");

        assert!(t.matches_path("a.md"), "zero folders");
        assert!(t.matches_path("x/a.md"), "one folder");
        assert!(t.matches_path("x/y/a.md"), "several folders");
    }

    #[test]
    fn matches_path_of_a_coded_template_fits_only_its_own_key() {
        let t = coded("tickets/{key}.md");

        assert!(t.matches_path("tickets/WF-3.md"));
        assert!(!t.matches_path("tickets/WF-3.txt"));
        assert!(!t.matches_path("tickets/other/WF-3.md"));
    }

    fn read(template: &Template, below: &str) -> Option<(String, Option<String>, NameState)> {
        template
            .read(below)
            .map(|name| (name.key, name.slug, name.state))
    }

    fn slugged(
        key: &str,
        slug: &str,
        state: NameState,
    ) -> Option<(String, Option<String>, NameState)> {
        Some((key.to_owned(), Some(slug.to_owned()), state))
    }

    fn plain(key: &str, state: NameState) -> Option<(String, Option<String>, NameState)> {
        Some((key.to_owned(), None, state))
    }

    #[test]
    fn the_number_is_every_digit_after_the_code_and_a_slug_begins_at_the_dash_after_the_last() {
        let t = coded("tickets/{key}.md");

        assert_eq!(
            read(&t, "tickets/WF-10.md"),
            plain("WF-10", NameState::Expected)
        );
        assert_eq!(
            read(&t, "tickets/WF-12-x.md"),
            slugged("WF-12", "x", NameState::Expected)
        );
        assert_eq!(
            read(&t, "tickets/WF-1-2x.md"),
            slugged("WF-1", "2x", NameState::Expected)
        );
    }

    #[test]
    fn the_slug_is_what_is_left_once_the_templates_own_text_is_taken_off_the_end() {
        let t = coded("tickets/{key}.md");

        assert_eq!(
            read(&t, "tickets/WF-1-v1.2.md"),
            slugged("WF-1", "v1.2", NameState::Expected)
        );
        assert_eq!(
            read(&t, "tickets/WF-1-a.md.md"),
            slugged("WF-1", "a.md", NameState::Expected)
        );
    }

    #[test]
    fn a_slug_in_any_language_is_valid() {
        assert_eq!(
            read(&coded("{key}.md"), "WF-3-\u{e23}\u{e48}\u{e32}\u{e07}.md"),
            slugged("WF-3", "\u{e23}\u{e48}\u{e32}\u{e07}", NameState::Expected)
        );
    }

    #[test]
    fn a_slug_that_is_empty_or_holds_an_excluded_character_is_a_member_with_an_invalid_slug() {
        let t = coded("{key}.md");

        for (name, slug) in [
            ("WF-1-.md", ""),
            ("WF-1-a b.md", "a b"),
            ("WF-1-a\tb.md", "a\tb"),
            ("WF-1-a#b.md", "a#b"),
            ("WF-1-a:b.md", "a:b"),
        ] {
            assert_eq!(
                read(&t, name),
                slugged("WF-1", slug, NameState::InvalidSlug),
                "{name:?}"
            );
            assert!(t.matches_path(name), "{name:?}");
        }
    }

    #[test]
    fn text_after_the_digits_that_is_not_a_dash_makes_no_member() {
        let t = coded("{key}.md");

        for name in ["WF-1x.md", "WF-1.txt", "WF-1-x.txt", "WF-.md", "WF--x.md"] {
            assert_eq!(read(&t, name), None, "{name:?}");
            assert!(!t.matches_path(name), "{name:?}");
        }
    }

    #[test]
    fn a_key_in_a_folder_takes_its_slug_on_the_folder() {
        let t = coded("{key}/README.md");

        assert_eq!(
            read(&t, "WF-1-x/README.md"),
            slugged("WF-1", "x", NameState::Expected)
        );
        assert_eq!(
            read(&t, "WF-1/README.md"),
            plain("WF-1", NameState::Expected)
        );
        assert!(t.matches_path("WF-1-x/README.md"));
        let Step::Name(folder) = &t.steps()[0] else {
            panic!("a segment was expected");
        };
        assert!(folder.matches_folder("WF-1-x"));
    }

    #[test]
    fn the_collections_slug_decides_which_form_is_expected() {
        let optional = bound("{key}.md", SlugMode::Optional);
        let required = bound("{key}.md", SlugMode::Required);
        let none = bound("{key}.md", SlugMode::None);

        assert_eq!(
            read(&optional, "WF-1.md"),
            plain("WF-1", NameState::Expected)
        );
        assert_eq!(
            read(&optional, "WF-1-x.md"),
            slugged("WF-1", "x", NameState::Expected)
        );
        assert_eq!(
            read(&required, "WF-1.md"),
            plain("WF-1", NameState::UnexpectedForm)
        );
        assert_eq!(
            read(&required, "WF-1-x.md"),
            slugged("WF-1", "x", NameState::Expected)
        );
        assert_eq!(
            read(&required, "WF-1-a b.md"),
            slugged("WF-1", "a b", NameState::InvalidSlug)
        );
        assert_eq!(read(&none, "WF-1.md"), plain("WF-1", NameState::Expected));
        assert_eq!(
            read(&none, "WF-1-x.md"),
            slugged("WF-1", "x", NameState::UnexpectedForm)
        );
        assert_eq!(
            read(&none, "WF-1-a b.md"),
            slugged("WF-1", "a b", NameState::UnexpectedForm)
        );
    }

    #[test]
    fn render_is_the_inverse_of_read_with_a_slug_and_without_one() {
        for (text, key, slug) in [
            ("tickets/{key}.md", "WF-3", Some("lock-order")),
            ("tickets/{key}.md", "WF-3", None),
            ("{key}.md", "WF-30", Some("2x")),
            ("{key}/index.md", "WF-3", Some("v1.2")),
            ("{key}/index.md", "WF-3", None),
        ] {
            let template = coded(text);
            let rendered = template.render(key, slug).unwrap();
            let name = template.read(&rendered).unwrap();

            assert_eq!(name.key, key, "{text}");
            assert_eq!(name.slug.as_deref(), slug, "{text}");
        }
        assert_eq!(
            coded("tickets/{key}.md").render("WF-8", Some("lock-order")),
            Some("tickets/WF-8-lock-order.md".to_owned())
        );
    }

    #[test]
    fn a_template_with_a_digit_or_a_dash_right_after_the_key_binds_only_under_none() {
        for text in ["{key}1.md", "{key}-notes.md", "t/{key}9/a.md"] {
            for slug in [SlugMode::Optional, SlugMode::Required] {
                let refused = Template::parse(text)
                    .unwrap()
                    .bind(text, Some("WF"), slug)
                    .unwrap_err();
                assert!(refused.contains("`slug` to `none`"), "{text}: {refused}");
            }
            assert!(
                Template::parse(text)
                    .unwrap()
                    .bind(text, Some("WF"), SlugMode::None)
                    .is_ok(),
                "{text}"
            );
        }
    }

    #[test]
    fn a_template_with_a_digit_after_the_key_under_none_reads_no_slug() {
        let t = bound("{key}1.md", SlugMode::None);

        assert_eq!(read(&t, "WF-31.md"), plain("WF-3", NameState::Expected));
        assert_eq!(read(&t, "WF-3111.md"), plain("WF-311", NameState::Expected));
        assert_eq!(read(&t, "WF-3-x1.md"), None);
        let dash = bound("{key}-notes.md", SlugMode::None);
        assert_eq!(
            read(&dash, "WF-3-notes.md"),
            plain("WF-3", NameState::Expected)
        );
        assert_eq!(read(&dash, "WF-3-x-notes.md"), None);
    }
}
