//! `date` -- print the current date and time.
//!
//! Deliberately UTC-only for now (no `$TZ`/localtime support, no
//! IANA timezone database) and read-only (no `-s`/`--set` to change
//! the system clock) -- both are real gaps, not silent ones; see
//! README.md's Known limitations. What's here covers the by-far more
//! common case: printing the current time in a script-friendly
//! format.
//!
//! Calendar math is Howard Hinnant's `civil_from_days` (see
//! <http://howardhinnant.github.io/date_algorithms.html>), a
//! well-known, publicly documented way to convert a day count to a
//! proleptic Gregorian year/month/day -- reimplemented here in plain
//! `std` arithmetic to keep this crate's zero-dependency stance
//! rather than pulling in a date/time crate for one utility. `tests`
//! below round-trip it against its inverse (`days_from_civil`, kept
//! test-only since nothing at runtime needs that direction) across a
//! wide date range, plus check it against independently-verified
//! fixed points.

use crate::common::errors::{AppError, AppResult};

const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub const USAGE: &str = "date [+FORMAT] -- print the current date and time (UTC)";

struct Civil {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    weekday: usize, // 0 = Sunday
}

fn div_floor(a: i64, b: i64) -> i64 {
    let q = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// Days since 1970-01-01 -> proleptic Gregorian (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = div_floor(z, 146_097);
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m: u32 = if mp < 10 {
        (mp + 3) as u32
    } else {
        (mp - 9) as u32
    }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Inverse of `civil_from_days`. Only used by `tests` to round-trip
/// the conversion above -- nothing at runtime needs this direction,
/// so it's `cfg(test)`-only rather than dead code in a real build.
#[cfg(test)]
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = div_floor(y, 400);
    let yoe = (y - era * 400) as u64; // [0, 399]
    let mp: u64 = if m > 2 {
        (m - 3) as u64
    } else {
        (m + 9) as u64
    }; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d as u64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe as i64 - 719_468
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn day_of_year(year: i64, month: u32, day: u32) -> u32 {
    let mut doy = day;
    for m in 1..month {
        doy += days_in_month(year, m);
    }
    doy
}

fn from_unix_time(secs: i64) -> Civil {
    let days = secs.div_euclid(86_400);
    let time_of_day = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    // Epoch day (1970-01-01, day 0) was a Thursday (index 4).
    let weekday = (days + 4).rem_euclid(7) as usize;
    Civil {
        year,
        month,
        day,
        hour: (time_of_day / 3600) as u32,
        minute: ((time_of_day % 3600) / 60) as u32,
        second: (time_of_day % 60) as u32,
        weekday,
    }
}

fn default_format(c: &Civil) -> String {
    format!(
        "{} {} {:2} {:02}:{:02}:{:02} UTC {}",
        &DAY_NAMES[c.weekday][..3],
        &MONTH_NAMES[(c.month - 1) as usize][..3],
        c.day,
        c.hour,
        c.minute,
        c.second,
        c.year,
    )
}

fn render(fmt: &str, c: &Civil, unix_secs: i64) -> String {
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('Y') => out.push_str(&c.year.to_string()),
            Some('m') => out.push_str(&format!("{:02}", c.month)),
            Some('d') => out.push_str(&format!("{:02}", c.day)),
            Some('H') => out.push_str(&format!("{:02}", c.hour)),
            Some('M') => out.push_str(&format!("{:02}", c.minute)),
            Some('S') => out.push_str(&format!("{:02}", c.second)),
            Some('j') => out.push_str(&format!("{:03}", day_of_year(c.year, c.month, c.day))),
            Some('A') => out.push_str(DAY_NAMES[c.weekday]),
            Some('a') => out.push_str(&DAY_NAMES[c.weekday][..3]),
            Some('B') => out.push_str(MONTH_NAMES[(c.month - 1) as usize]),
            Some('b') => out.push_str(&MONTH_NAMES[(c.month - 1) as usize][..3]),
            Some('s') => out.push_str(&unix_secs.to_string()),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

pub fn run(args: Vec<String>) -> AppResult<()> {
    let (opts, _forced) = crate::common::args::split_dashdash(args);
    let mut format_spec: Option<String> = None;
    for arg in opts {
        match arg.strip_prefix('+') {
            Some(spec) => format_spec = Some(spec.to_string()),
            None => return Err(AppError::usage(format!("unrecognized option '{arg}'"))),
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AppError::new("system clock is set before 1970-01-01"))?;
    let unix_secs = now.as_secs() as i64;
    let civil = from_unix_time(unix_secs);

    match format_spec {
        Some(spec) => println!("{}", render(&spec, &civil, unix_secs)),
        None => println!("{}", default_format(&civil)),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970_01_01_thursday() {
        let c = from_unix_time(0);
        assert_eq!((c.year, c.month, c.day), (1970, 1, 1));
        assert_eq!(DAY_NAMES[c.weekday], "Thursday");
        assert_eq!((c.hour, c.minute, c.second), (0, 0, 0));
    }

    #[test]
    fn known_fixed_points() {
        // Independently verified (Python's datetime.date, not just
        // this same algorithm run twice): 2000-01-01 is 10957 days
        // after the epoch, 2024-02-29 (a leap day) is 19782.
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
        assert_eq!(days_from_civil(2024, 2, 29), 19_782);
    }

    #[test]
    fn round_trips_across_a_wide_range() {
        for days in -100_000i64..100_000 {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "mismatch at day {days}");
        }
    }

    #[test]
    fn day_of_year_examples() {
        assert_eq!(day_of_year(2024, 1, 1), 1);
        assert_eq!(day_of_year(2024, 2, 29), 60); // 2024 is a leap year
        assert_eq!(day_of_year(2023, 2, 28), 59); // 2023 is not
        assert_eq!(day_of_year(2024, 12, 31), 366);
    }

    #[test]
    fn render_handles_common_specifiers() {
        let c = Civil {
            year: 2026,
            month: 9,
            day: 20,
            hour: 14,
            minute: 5,
            second: 9,
            weekday: 0, // Sunday
        };
        assert_eq!(render("%Y-%m-%d", &c, 0), "2026-09-20");
        assert_eq!(render("%H:%M:%S", &c, 0), "14:05:09");
        assert_eq!(render("%A, %B %d", &c, 0), "Sunday, September 20");
        assert_eq!(render("%s", &c, 1_758_000_000), "1758000000");
        assert_eq!(render("100%%", &c, 0), "100%");
    }
}
