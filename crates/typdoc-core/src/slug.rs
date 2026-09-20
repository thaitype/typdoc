//! Heading anchors as GitHub makes them.

use std::collections::HashMap;

use unicode_general_category::{GeneralCategory as Category, get_general_category};

/// Gives each heading of one document its anchor, in the order the headings come.
#[derive(Default)]
pub struct Slugger {
    taken: HashMap<String, usize>,
}

impl Slugger {
    /// The anchor for a heading whose plain text is `text`: the slug of the text, with
    /// `-1`, `-2` and so on added when an earlier heading has it, skipping every result
    /// already taken.
    pub fn slug(&mut self, text: &str) -> String {
        let base = slug_of(text);
        let mut result = base.clone();
        while self.taken.contains_key(&result) {
            let count = self.taken.entry(base.clone()).or_insert(0);
            *count += 1;
            result = format!("{base}-{count}");
        }
        self.taken.insert(result.clone(), 0);
        result
    }
}

/// Lowercased, with every character that is not a letter, a digit, a mark, `-`, `_` or a
/// space deleted, and each space turned into `-`.
fn slug_of(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|&c| !is_deleted(c))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// Deleted: other numbers, punctuation other than the connector `_`, symbols, controls,
/// private-use, format and unassigned characters, and separators other than a space; unless
/// the character is alphabetic or is `-`.
fn is_deleted(c: char) -> bool {
    if c == ' ' || c == '-' || c.is_alphabetic() {
        return false;
    }
    use Category::*;
    matches!(
        get_general_category(c),
        OtherNumber
            | ClosePunctuation
            | FinalPunctuation
            | InitialPunctuation
            | OpenPunctuation
            | OtherPunctuation
            | DashPunctuation
            | MathSymbol
            | CurrencySymbol
            | ModifierSymbol
            | OtherSymbol
            | Control
            | PrivateUse
            | Format
            | Unassigned
            | SpaceSeparator
            | LineSeparator
            | ParagraphSeparator
    )
}
