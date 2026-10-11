//! JavaScript string semantics the transcript rules are written in.
//!
//! The rules being ported measure, slice and trim strings the way JavaScript does, and several of
//! those lengths reach the document (a tool body clipped at 4,000, a preview truncated at 80). A
//! JavaScript string index is a UTF-16 code unit and a Rust one is a byte, so every budget that
//! crosses the seam is counted here in UTF-16 units instead.
//!
//! Where a budget would fall inside a surrogate pair JavaScript keeps the lone surrogate and Rust
//! cannot spell one, so the cut moves back to the whole code point. That is the only place the two
//! sides can differ, and only for a string cut exactly through an astral character.

/// Whitespace as JavaScript's `\s` and `String.prototype.trim` read it.
///
/// `char::is_whitespace` is the Unicode White_Space property, which is the same set apart from
/// U+0085 (not whitespace to JavaScript) and U+FEFF (whitespace to JavaScript, not to Unicode).
pub fn is_js_space(value: char) -> bool {
    (value.is_whitespace() && value != '\u{85}') || value == '\u{feff}'
}

/// `String.prototype.trim`.
pub fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_space)
}

/// `String.prototype.trimStart`.
pub fn js_trim_start(value: &str) -> &str {
    value.trim_start_matches(is_js_space)
}

/// `String.prototype.trimEnd`.
pub fn js_trim_end(value: &str) -> &str {
    value.trim_end_matches(is_js_space)
}

/// `String.prototype.length`: UTF-16 code units, not bytes and not characters.
pub fn utf16_len(value: &str) -> usize {
    value.chars().map(char::len_utf16).sum()
}

/// The longest prefix of at most `units` UTF-16 code units, cut on a code point boundary.
pub fn utf16_take(value: &str, units: usize) -> &str {
    let mut seen = 0;
    for (offset, character) in value.char_indices() {
        let next = seen + character.len_utf16();
        if next > units {
            return &value[..offset];
        }
        seen = next;
    }
    value
}

/// `text.replace(/\s+/g, ' ')`.
pub fn collapse_whitespace(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_run = false;
    for character in value.chars() {
        if is_js_space(character) {
            if !in_run {
                out.push(' ');
                in_run = true;
            }
            continue;
        }
        in_run = false;
        out.push(character);
    }
    out
}

/// `text.split('\n')`, which always yields at least one element.
pub fn split_lf(value: &str) -> Vec<&str> {
    value.split('\n').collect()
}

/// `text.split(/\r?\n/)`.
pub fn split_newlines(value: &str) -> Vec<&str> {
    value
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

/// `text.split(/\s+/)` for a string that has already been trimmed, which is how every caller uses
/// it: a leading run of whitespace would otherwise produce a leading empty token.
pub fn split_whitespace_tokens(value: &str) -> Vec<&str> {
    value
        .split(is_js_space)
        .filter(|part| !part.is_empty())
        .collect()
}

/// The last segment after either separator, as `path.split(/[\\/]/).at(-1)` reads it.
pub fn last_path_segment(value: &str) -> &str {
    match value.rfind(['/', '\\']) {
        Some(index) => &value[index + 1..],
        None => value,
    }
}

/// `decodeURI`, which leaves the reserved set (`;/?:@&=+$,#`) escaped and fails on a malformed
/// escape. A failure means the caller keeps the raw text, so this returns `None` rather than a
/// lossy reading.
pub fn decode_uri(value: &str) -> Option<String> {
    const RESERVED: &[u8] = b";/?:@&=+$,#";
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        let digits = bytes.get(index + 1..index + 3)?;
        let decoded = u8::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        if decoded < 0x80 && RESERVED.contains(&decoded) {
            out.extend_from_slice(&bytes[index..index + 3]);
        } else {
            out.push(decoded);
        }
        index += 3;
    }
    String::from_utf8(out).ok()
}

/// ASCII-only lowercase, which is what every classification rule here wants: the tag names, tool
/// names and extensions being matched are ASCII, and a full Unicode fold would map a Turkish
/// dotless i onto a letter the rule never meant to accept.
pub fn ascii_lower(value: &str) -> String {
    value.to_ascii_lowercase()
}
