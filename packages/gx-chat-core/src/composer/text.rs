//! UTF-16 offsets, because every composer offset in the chat is a JavaScript string index.
//!
//! CDXC:SessionChat 2026-09-22 WHY:
//! `sessionChatComposerReferences`, the caret, the selection and every suggestion trigger count
//! UTF-16 code units, which is what a JS string index is. The renderer and the document both speak
//! that unit, so the core scans over code points and converts at the boundary rather than handing
//! out byte offsets that would move every pill on the first emoji in a draft.

/// A draft indexed the way JavaScript indexes it.
///
/// Holds the code points once and a prefix table of UTF-16 lengths, so a scan works in code points
/// (which is what the TypeScript regexes effectively match) and reports offsets in code units.
#[derive(Clone, Debug)]
pub struct Utf16Text {
    chars: Vec<char>,
    /// `prefix[i]` is the UTF-16 length of the first `i` code points.
    prefix: Vec<usize>,
}

impl Utf16Text {
    /// Indexes `text`.
    pub fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let mut prefix = Vec::with_capacity(chars.len() + 1);
        let mut total = 0;
        prefix.push(0);
        for character in &chars {
            total += character.len_utf16();
            prefix.push(total);
        }
        Self { chars, prefix }
    }

    /// The code points, for a scan.
    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    /// How many code points the text has.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// The code point at `index`, or `None` past the end. Negative indexes cannot exist here; the
    /// TypeScript reads `text[-1]` as `undefined`, which callers spell as `None`.
    pub fn at(&self, index: usize) -> Option<char> {
        self.chars.get(index).copied()
    }

    /// The UTF-16 offset of code point `index`.
    pub fn offset(&self, index: usize) -> usize {
        self.prefix[index.min(self.chars.len())]
    }

    /// The code point index a UTF-16 offset falls on, rounded down to a whole code point.
    pub fn index_of_offset(&self, offset: usize) -> usize {
        match self.prefix.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
    }

    /// The whole text.
    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// `text.slice(start, end)` in code points.
    pub fn slice(&self, start: usize, end: usize) -> String {
        let start = start.min(self.chars.len());
        let end = end.clamp(start, self.chars.len());
        self.chars[start..end].iter().collect()
    }

    /// The total UTF-16 length, which is `text.length` in JavaScript.
    pub fn utf16_len(&self) -> usize {
        *self.prefix.last().unwrap_or(&0)
    }
}

/// The UTF-16 length of `text`, which is `text.length` in JavaScript.
pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}
