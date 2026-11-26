//! Calendar dates without external crates.
//!
//! Memory uses plain `YYYY-MM-DD` dates. Timestamps such as
//! `2026-11-25T14:03:00Z` are accepted and cut to their date part.
//! Day arithmetic uses the days-from-civil algorithm (proleptic Gregorian).

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub y: i32,
    pub m: u32,
    pub d: u32,
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

impl Date {
    /// Parses `YYYY-MM-DD`, optionally followed by `T...` or a space and a time.
    pub fn parse(s: &str) -> Option<Date> {
        let s = s.trim();
        let head = s.get(..10)?;
        let b = head.as_bytes();
        if b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        if !head
            .bytes()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        {
            return None;
        }
        if s.len() > 10 {
            let next = s.as_bytes()[10];
            if next != b'T' && next != b' ' {
                return None;
            }
        }
        let y: i32 = head[0..4].parse().ok()?;
        let m: u32 = head[5..7].parse().ok()?;
        let d: u32 = head[8..10].parse().ok()?;
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return None;
        }
        Some(Date { y, m, d })
    }

    /// Today in UTC. `VMEM_TODAY=YYYY-MM-DD` overrides it (tests, replays).
    pub fn today() -> Date {
        if let Ok(v) = std::env::var("VMEM_TODAY") {
            if let Some(d) = Date::parse(&v) {
                return d;
            }
        }
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Date::from_days(secs.div_euclid(86_400))
    }

    pub fn from_days(z: i64) -> Date {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        Date { y: y as i32, m: m as u32, d: d as u32 }
    }

    pub fn to_days(self) -> i64 {
        let m = self.m as i64;
        let d = self.d as i64;
        let y = if m <= 2 { self.y as i64 - 1 } else { self.y as i64 };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn add_days(self, n: i64) -> Date {
        Date::from_days(self.to_days() + n)
    }

    /// Days from `self` until `later` (negative if `later` is earlier).
    pub fn days_until(self, later: Date) -> i64 {
        later.to_days() - self.to_days()
    }

    /// "November 25, 2026"
    pub fn long(self) -> String {
        format!("{} {}, {}", MONTHS[(self.m - 1) as usize], self.d, self.y)
    }

    /// "November 25", for speech.
    pub fn spoken(self) -> String {
        format!("{} {}", MONTHS[(self.m - 1) as usize], self.d)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dates_and_timestamps() {
        assert_eq!(Date::parse("2026-11-25"), Some(Date { y: 2026, m: 11, d: 25 }));
        assert_eq!(Date::parse("2026-11-25T14:03:00Z"), Some(Date { y: 2026, m: 11, d: 25 }));
        assert_eq!(Date::parse("2026-02-29"), None);
        assert_eq!(Date::parse("2024-02-29"), Some(Date { y: 2024, m: 2, d: 29 }));
        assert_eq!(Date::parse("2026-13-01"), None);
        assert_eq!(Date::parse("2026-11-2"), None);
        assert_eq!(Date::parse("2026-11-25x"), None);
        assert_eq!(Date::parse("año-11-25"), None);
    }

    #[test]
    fn day_arithmetic_round_trips() {
        assert_eq!(Date::parse("1970-01-01").unwrap().to_days(), 0);
        let d = Date::parse("2026-11-25").unwrap();
        assert_eq!(Date::from_days(d.to_days()), d);
        assert_eq!(d.add_days(7).to_string(), "2026-12-02");
        assert_eq!(d.add_days(-25).to_string(), "2026-10-31");
        assert_eq!(Date::parse("2027-01-02").unwrap().days_until(d), -38);
        for z in (-800_000..800_000).step_by(997) {
            assert_eq!(Date::from_days(z).to_days(), z);
        }
    }

    #[test]
    fn formats_for_people() {
        let d = Date::parse("2026-11-25").unwrap();
        assert_eq!(d.long(), "November 25, 2026");
        assert_eq!(d.spoken(), "November 25");
    }
}
