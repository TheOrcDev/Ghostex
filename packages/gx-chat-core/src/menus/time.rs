//! The two date conversions the menus need, without a clock and without a dependency.
//!
//! The brain stamps a dispatch with `new Date(now).toISOString()` and compares evidence with
//! `Date.parse(iso)`. Both are pure functions of a number and a string, so they belong here rather
//! than in the host; the core still reads no clock, it only formats the one the host passed in.

/// `Date.parse(text)` as whole milliseconds.
///
/// One rule for the whole crate since 2026-09-22 (`crate::jstime`). The menus have no
/// `ChatContext` on these paths, so an offset-less date-time still reads as UTC; every producer
/// here writes `…Z` except a provider's own `resetsAt`, which is the one stamp that would move.
pub fn parse_iso_millis(text: &str) -> Option<i64> {
    crate::jstime::parse_iso_millis_utc(text).map(|millis| millis as i64)
}

/// `new Date(millis).toISOString()`.
pub fn iso_from_millis(millis: i64) -> String {
    let days = millis.div_euclid(86_400_000);
    let rest = millis.rem_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    let hour = rest / 3_600_000;
    let minute = rest / 60_000 % 60;
    let second = rest / 1_000 % 60;
    let fraction = rest % 1_000;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:03}Z")
}

/// The inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}
