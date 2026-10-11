//! JavaScript-compatible string and date helpers.
//!
//! The sidebar rules being ported run in JavaScript, where `trim`, `\s`, `slice`, and `length`
//! work on UTF-16 code units and a JavaScript notion of whitespace. Parity with the TypeScript
//! projection depends on reproducing those exactly, not on Rust's own definitions.

/// JavaScript `\s` (and the set `String.prototype.trim` strips): the ECMAScript WhiteSpace and
/// LineTerminator code points. Differs from `char::is_whitespace`: it includes U+FEFF and excludes
/// U+0085.
pub(crate) fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JavaScript line terminators, the characters `.` does not match without the `s` flag.
pub(crate) fn is_js_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `String.prototype.trim`.
pub(crate) fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

/// `String.prototype.trimStart`.
pub(crate) fn js_trim_start(value: &str) -> &str {
    value.trim_start_matches(is_js_whitespace)
}

/// `value.trim().replace(/\s+/g, ' ')`.
pub(crate) fn collapse_js_whitespace(value: &str) -> String {
    let mut collapsed = String::with_capacity(value.len());
    let mut pending_space = false;
    for character in js_trim(value).chars() {
        if is_js_whitespace(character) {
            pending_space = true;
            continue;
        }
        if pending_space {
            collapsed.push(' ');
            pending_space = false;
        }
        collapsed.push(character);
    }
    collapsed
}

/// `value?.trim().replace(/\s+/g, ' ')`, `None` when that is empty.
pub(crate) fn normalized_non_empty(value: Option<&str>) -> Option<String> {
    let collapsed = collapse_js_whitespace(value?);
    (!collapsed.is_empty()).then_some(collapsed)
}

/// `value.length` of a JavaScript string.
pub(crate) fn utf16_len(value: &str) -> usize {
    value.chars().map(char::len_utf16).sum()
}

/// `value.slice(0, max_units)`. A character that would be cut in half (a surrogate pair at the
/// boundary) is left out; JavaScript would keep a lone surrogate, which Rust cannot hold.
pub(crate) fn utf16_prefix(value: &str, max_units: usize) -> &str {
    let mut units = 0;
    for (index, character) in value.char_indices() {
        units += character.len_utf16();
        if units > max_units {
            return &value[..index];
        }
    }
    value
}

/// `value.slice(start_units)`, with the same surrogate caveat as [`utf16_prefix`].
pub(crate) fn utf16_suffix(value: &str, start_units: usize) -> &str {
    let mut units = 0;
    for (index, character) in value.char_indices() {
        if units >= start_units {
            return &value[index..];
        }
        units += character.len_utf16();
    }
    ""
}

/// JavaScript `encodeURIComponent`, re-exported for the sidebar id builders.
pub(crate) use crate::keys::encode_uri_component;

/// `Date.parse` for the ISO-8601 forms gxserver and the app write: `YYYY-MM-DD`,
/// `YYYY-MM-DDTHH:MM`, `YYYY-MM-DDTHH:MM:SS`, optional fraction, and `Z` or `±HH:MM`. A day past
/// the end of its month rolls over the way `MakeDay` does (`2026-02-31` is 3 March) and a field
/// outside its range is rejected, both measured in QuickJS and V8 and the same in each.
///
/// Three deliberate differences from a JavaScript engine, none reachable from a daemon stamp,
/// which is always `YYYY-MM-DDTHH:MM:SS.sssZ`: a lowercase `t` or `z` is accepted here, matching
/// V8, where QuickJS returns `NaN`; only the format above is taken, where an engine also parses
/// its legacy and locale forms; and a date-time without an offset is read as UTC, where an engine
/// reads local time, which the core cannot know because it reads no clock and no time zone.
pub(crate) fn parse_iso_ms(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    let digits = |from: usize, count: usize| -> Option<i64> {
        let slice = bytes.get(from..from + count)?;
        let mut number = 0i64;
        for byte in slice {
            if !byte.is_ascii_digit() {
                return None;
            }
            number = number * 10 + i64::from(byte - b'0');
        }
        Some(number)
    };
    let year = digits(0, 4)?;
    if bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }
    let month = digits(5, 2)?;
    let day = digits(8, 2)?;
    // The field bounds an engine enforces; a day past the end of its month is carried into the
    // next one by the arithmetic below rather than rejected.
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut millis = days_from_civil(year, month, day) * 86_400_000;
    if bytes.len() == 10 {
        return Some(millis);
    }
    if !matches!(bytes.get(10), Some(b'T') | Some(b't')) {
        return None;
    }
    let hour = digits(11, 2)?;
    if bytes.get(13) != Some(&b':') {
        return None;
    }
    let minute = digits(14, 2)?;
    let mut index = 16;
    let mut second = 0;
    let mut fraction_ms = 0;
    if bytes.get(index) == Some(&b':') {
        second = digits(index + 1, 2)?;
        index += 3;
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            let start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            if index == start {
                return None;
            }
            // Milliseconds: the first three digits, padded; further digits are ignored.
            let mut scale = 100;
            for byte in &bytes[start..index.min(start + 3)] {
                fraction_ms += i64::from(byte - b'0') * scale;
                scale /= 10;
            }
        }
    }
    if hour > 24
        || minute > 59
        || second > 59
        || (hour == 24 && (minute, second, fraction_ms) != (0, 0, 0))
    {
        return None;
    }
    let offset_minutes = match bytes.get(index) {
        None => 0,
        Some(b'Z') | Some(b'z') if index + 1 == bytes.len() => 0,
        Some(sign @ (b'+' | b'-')) => {
            let offset_hour = digits(index + 1, 2)?;
            if bytes.get(index + 3) != Some(&b':') || index + 6 != bytes.len() {
                return None;
            }
            let offset_minute = digits(index + 4, 2)?;
            let total = offset_hour * 60 + offset_minute;
            if *sign == b'+' {
                total
            } else {
                -total
            }
        }
        _ => return None,
    };
    millis += ((hour * 60 + minute - offset_minutes) * 60 + second) * 1000 + fraction_ms;
    Some(millis)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The inverse of [`days_from_civil`], kept beside it so the two stay one pair.
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}
