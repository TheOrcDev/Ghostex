//! `Date.parse` for the ISO-8601 stamps every surface of the chat carries, in one place.
//!
//! CDXC:SessionChat 2026-09-22 WHY:
//! This rule existed in SIX copies (`extras/time.rs`, `menus/time.rs`, `menus/context/time.rs`,
//! `composer/queue.rs`, `questions/notice_state.rs`, `session/startup_sends.rs`) and they
//! disagreed on what `Date.parse` answers: one accepted hour 99, one accepted trailing junk after
//! `Z`, one dropped the offset entirely and lost up to 999 ms of a one- or two-digit fraction, and
//! two indexed by byte range so a stamp whose first characters are not ASCII panicked the host
//! thread. Measured against V8, the copy below is the only one that was right, so it is the one
//! that survived; the other five delegate to it.
//!
//! The one case that needs the host is an offset-less DATE-TIME (`2026-09-22T10:00:00`), which
//! `Date.parse` resolves against LOCAL time. The core reads no timezone, so the offset comes from
//! [`crate::ChatContext::utc_offset_minutes`]; a caller with no context uses
//! [`parse_iso_millis_utc`], which is what all five delegating copies did before the fold.

/// Epoch milliseconds for an ISO-8601 stamp, or `None` where `Date.parse` returns `NaN`.
///
/// Accepts `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM`, `YYYY-MM-DDTHH:MM:SS`, an optional fractional second,
/// and an optional `Z` or `±HH:MM` offset. A date-only stamp is UTC and a date-time without an
/// offset is local, which is what the ECMAScript date-time string format says.
///
/// CDXC:SessionChat 2026-09-22 WHY:
/// Every field is taken with `str::get`, not with a byte range. A stamp is server data and can be
/// any string at all, and the core runs on the host's own thread: `&value[0..4]` on a `detectedAt`
/// whose first characters are not ASCII panics in the middle of a slice and takes the chat window
/// with it. `Date.parse` answers NaN for all of them, which is what `None` is here.
pub fn parse_iso_millis(value: &str, utc_offset_minutes: i32) -> Option<f64> {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let year: i64 = digits(value.get(0..4)?)?;
    if bytes[4] != b'-' {
        return None;
    }
    let month: i64 = digits(value.get(5..7)?)?;
    if bytes[7] != b'-' {
        return None;
    }
    let day: i64 = digits(value.get(8..10)?)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let rest = value.get(10..)?;
    if rest.is_empty() {
        // A date-only stamp is UTC, which is what the ECMAScript date-time string format says.
        return epoch_millis(CivilStamp {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            millis: 0.0,
            offset_minutes: 0,
        });
    }
    if !rest.starts_with('T') && !rest.starts_with('t') && !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.get(1..)?;
    if rest.len() < 5 {
        return None;
    }
    let hour: i64 = digits(rest.get(0..2)?)?;
    if rest.as_bytes()[2] != b':' {
        return None;
    }
    let minute: i64 = digits(rest.get(3..5)?)?;
    let mut tail = rest.get(5..)?;
    let mut second: i64 = 0;
    let mut fraction = 0.0_f64;
    if tail.starts_with(':') {
        if tail.len() < 3 {
            return None;
        }
        second = digits(tail.get(1..3)?)?;
        tail = tail.get(3..)?;
        if tail.starts_with('.') {
            let digit_count = tail[1..]
                .bytes()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            if digit_count == 0 {
                return None;
            }
            // `Date.parse` keeps millisecond precision and truncates the rest.
            let millis_text: String = tail[1..]
                .chars()
                .take(digit_count.min(3))
                .chain("000".chars())
                .take(3)
                .collect();
            fraction = digits::<i64>(&millis_text)? as f64;
            tail = tail.get(1 + digit_count..)?;
        }
    }
    if hour > 24 || minute > 59 || second > 59 {
        return None;
    }
    let offset = match tail {
        "" => utc_offset_minutes,
        "Z" | "z" => 0,
        _ => parse_offset(tail)?,
    };
    epoch_millis(CivilStamp {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millis: fraction,
        offset_minutes: offset,
    })
}

/// `±HH:MM`, `±HHMM` or `±HH`, the offsets `Date.parse` accepts on an ISO stamp.
fn parse_offset(text: &str) -> Option<i32> {
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let body: String = text[1..].chars().filter(|c| *c != ':').collect();
    if body.len() != 2 && body.len() != 4 {
        return None;
    }
    let hours: i32 = digits(body.get(0..2)?)?;
    let minutes: i32 = if body.len() == 4 {
        digits(body.get(2..4)?)?
    } else {
        0
    };
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 60 + minutes))
}

/// One parsed stamp, before it becomes a number.
struct CivilStamp {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    millis: f64,
    offset_minutes: i32,
}

/// Milliseconds since the epoch, the same arithmetic `Date.UTC` does.
fn epoch_millis(stamp: CivilStamp) -> Option<f64> {
    let days = days_from_civil(stamp.year, stamp.month, stamp.day);
    let seconds = days * 86_400 + stamp.hour * 3_600 + stamp.minute * 60 + stamp.second;
    Some((seconds as f64) * 1_000.0 + stamp.millis - f64::from(stamp.offset_minutes) * 60_000.0)
}

/// Howard Hinnant's `days_from_civil`: days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// An all-digit field parsed as a number, or `None` when anything else is in it.
fn digits<T: std::str::FromStr>(text: &str) -> Option<T> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// [`parse_iso_millis`] for a caller with no [`crate::ChatContext`] to hand.
///
/// An offset-less date-time reads as UTC, which is what every copy of this rule did before the
/// fold. Every producer on those paths writes `…Z`, so the difference cannot reach a document
/// today; a caller that can see an agent-written stamp should thread the real offset instead.
pub fn parse_iso_millis_utc(value: &str) -> Option<f64> {
    parse_iso_millis(value, 0)
}
