//! Heading anchors as GitHub makes them (SPC-14).

use std::collections::HashMap;

use unicode_general_category::{GeneralCategory as Category, get_general_category};

/// Gives each heading of one document its anchor, in the order the headings come.
#[derive(Default)]
pub struct Slugger {
    taken: HashMap<String, usize>,
}

impl Slugger {
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

fn slug_of(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|&c| !is_deleted(c))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

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
