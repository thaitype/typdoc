//! The headings of a document's body.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::frontmatter;
use crate::lines::LineMap;
use crate::slug::Slugger;

/// A heading of a body, in the shape of `toc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
    pub line: usize,
    pub end: usize,
}

/// The headings of a whole file, in the order of `line`, which counts from the top of the file,
/// frontmatter included. Headings in fenced code are not headings; those in block quotes and
/// list items are.
pub fn headings(file: &str) -> Result<Vec<Heading>, String> {
    let body_at = frontmatter::split(file)?.body;
    let map = LineMap::new(file);
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;

    let mut found: Vec<(u8, String, usize)> = Vec::new();
    let mut in_heading = false;
    let mut images = 0;
    for (event, range) in Parser::new_ext(&file[body_at..], options).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let line = map.position(body_at + range.start).line;
                found.push((level as u8, String::new(), line));
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
            }
            Event::Start(Tag::Image { .. }) => images += 1,
            Event::End(TagEnd::Image) => images -= 1,
            Event::Text(text) | Event::Code(text) if in_heading && images == 0 => {
                if let Some((_, plain, _)) = found.last_mut() {
                    plain.push_str(&text);
                }
            }
            _ => {}
        }
    }

    let mut slugger = Slugger::default();
    let last = map.line_count();
    Ok((0..found.len())
        .map(|i| {
            let (level, text, line) = &found[i];
            let end = found[i + 1..]
                .iter()
                .find(|(deeper_or_same, _, _)| deeper_or_same <= level)
                .map_or(last, |(_, _, next)| next - 1);
            Heading {
                level: *level,
                slug: slugger.slug(text),
                text: text.clone(),
                line: *line,
                end,
            }
        })
        .collect())
}
