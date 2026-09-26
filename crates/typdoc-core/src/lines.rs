//! Lines and columns, counted as SPC-1 gives.

use std::num::NonZeroUsize;

/// A place in a text, 1-based. Ordered by `line` then `col`, so findings can be sorted by
/// position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

/// Where each line of a text begins.
pub struct LineMap<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineMap<'a> {
    pub fn new(text: &'a str) -> LineMap<'a> {
        let bytes = text.as_bytes();
        let mut starts = Vec::new();
        let mut rest = bytes;
        while let Some(len) = line_len(rest) {
            starts.push(bytes.len() - rest.len());
            rest = &rest[len.get()..];
        }
        LineMap { text, starts }
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// The line and column of the byte at `offset`, which lies on a character boundary. An
    /// offset on a line ending belongs to the line that the ending closes.
    pub fn position(&self, offset: usize) -> Position {
        let line = self.starts.partition_point(|&start| start <= offset).max(1);
        let start = self.starts.get(line - 1).copied().unwrap_or(0);
        Position {
            line,
            col: self.text[start..offset].chars().count() + 1,
        }
    }
}

/// The length of the first line of `bytes`, its ending included, or `None` when `bytes` is
/// empty and begins no line. A line holds at least its ending, so the length is never zero and
/// a walk that steps by it always reaches the end of the text.
fn line_len(bytes: &[u8]) -> Option<NonZeroUsize> {
    let len = match bytes.iter().position(|&b| b == b'\n' || b == b'\r') {
        None => bytes.len(),
        Some(at) if bytes[at] == b'\r' && bytes.get(at + 1) == Some(&b'\n') => at + 2,
        Some(at) => at + 1,
    };
    NonZeroUsize::new(len)
}

/// The start of the line after the one that begins at `start`, or the end of the text.
pub(crate) fn next_line(text: &str, start: usize) -> usize {
    start + line_len(&text.as_bytes()[start..]).map_or(0, NonZeroUsize::get)
}
