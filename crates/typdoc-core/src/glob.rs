/// Whether `name` fits `pattern`, where `*` stands for any run of characters, none included.
pub fn matches(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let mut parts: Vec<&str> = parts.collect();
    let Some(last) = parts.pop() else {
        return rest.is_empty();
    };
    for part in parts {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn a_pattern_without_a_star_is_the_whole_name() {
        assert!(matches("note.md", "note.md"));
        assert!(!matches("note.md", "Note.md"));
        assert!(!matches("note.md", "note.md.bak"));
        assert!(!matches("note.md", "a-note.md"));
    }

    #[test]
    fn a_star_stands_for_any_run_of_characters_none_included() {
        assert!(matches("*.md", "note.md"));
        assert!(matches("*.md", ".md"));
        assert!(!matches("*.md", "note.txt"));
        assert!(matches("a*", "a"));
        assert!(matches("a*b*c", "a--b--c"));
        assert!(matches("a*b*c", "abc"));
        assert!(!matches("a*b*c", "acb"));
    }

    #[test]
    fn the_parts_around_a_star_do_not_share_characters() {
        assert!(!matches("a*a", "a"));
        assert!(matches("a*a", "aa"));
        assert!(!matches("ab*ba", "aba"));
        assert!(matches("*", ""));
    }
}
