//! Recurrence rules for scheduled notes ("매주 수요일", "격주 목요일",
//! "매월 둘째 주 수요일" …).

use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// How a schedule repeats after its first date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    None,
    Daily,
    /// Every `every` weeks on the weekdays in `days` (bit 0 = 월 … bit 6 = 일).
    Weekly {
        every: u8,
        days: u8,
    },
    /// The same day of every month (clamped to short months).
    MonthlyDay,
    /// The `nth` (1..=4, or -1 for the last) `weekday` of every month.
    MonthlyNth {
        nth: i8,
        weekday: Weekday,
    },
}

pub const WEEKDAYS: [&str; 7] = ["월", "화", "수", "목", "금", "토", "일"];
const ORDINALS: [&str; 4] = ["첫째", "둘째", "셋째", "넷째"];

pub fn bit(w: Weekday) -> u8 {
    1 << w.num_days_from_monday()
}
/// Which week of its month `d` falls in for its weekday: 1..=5.
pub fn nth_of_month(d: NaiveDate) -> i8 {
    ((d.day() - 1) / 7 + 1) as i8
}
pub fn is_last_of_month(d: NaiveDate) -> bool {
    (d + Duration::days(7)).month() != d.month()
}
fn last_day(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.map_or(28, |n| (n - Duration::days(1)).day())
}

impl Repeat {
    pub fn is_none(&self) -> bool {
        *self == Repeat::None
    }
    /// Whether a schedule first starting on `start` repeats on `d`.
    pub fn matches(&self, start: NaiveDate, d: NaiveDate) -> bool {
        if d < start {
            return false;
        }
        match *self {
            Repeat::None => d == start,
            Repeat::Daily => true,
            Repeat::Weekly { every, days } => {
                let week =
                    |x: NaiveDate| x - Duration::days(x.weekday().num_days_from_monday() as i64);
                let weeks = (week(d) - week(start)).num_days() / 7;
                days & bit(d.weekday()) != 0 && weeks % every.max(1) as i64 == 0
            }
            Repeat::MonthlyDay => {
                let last = last_day(d.year(), d.month());
                d.day() == start.day().min(last)
            }
            Repeat::MonthlyNth { nth, weekday } => {
                d.weekday() == weekday
                    && if nth < 0 {
                        is_last_of_month(d)
                    } else {
                        nth_of_month(d) == nth
                    }
            }
        }
    }
    /// Like [`Repeat::label`], naming the day for monthly-by-date rules.
    pub fn describe(&self, start: NaiveDate) -> String {
        match self {
            Repeat::MonthlyDay => format!("매월 {}일", start.day()),
            _ => self.label(),
        }
    }
    /// A short Korean description, e.g. "격주 목요일".
    pub fn label(&self) -> String {
        match *self {
            Repeat::None => "반복 안 함".into(),
            Repeat::Daily => "매일".into(),
            Repeat::Weekly { every, days } => {
                let names: Vec<_> = (0..7)
                    .filter(|i| days & (1 << i) != 0)
                    .map(|i| WEEKDAYS[i])
                    .collect();
                let prefix = match every {
                    0 | 1 => "매주".to_string(),
                    2 => "격주".to_string(),
                    n => format!("{n}주마다"),
                };
                if names.len() == 1 {
                    format!("{prefix} {}요일", names[0])
                } else {
                    format!("{prefix} {}", names.join("·"))
                }
            }
            Repeat::MonthlyDay => "매월 같은 날".into(),
            Repeat::MonthlyNth { nth, weekday } => format!(
                "매월 {} 주 {}요일",
                if nth < 0 {
                    "마지막"
                } else {
                    ORDINALS[(nth.clamp(1, 4) - 1) as usize]
                },
                WEEKDAYS[weekday.num_days_from_monday() as usize]
            ),
        }
    }
}
pub fn ordinal(nth: i8) -> &'static str {
    if nth < 0 {
        "마지막"
    } else {
        ORDINALS[(nth.clamp(1, 4) - 1) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    #[test]
    fn weekly_and_biweekly_follow_their_weekday() {
        // 2026-11-11 is a Wednesday.
        let start = d(2026, 11, 11);
        let wed = Repeat::Weekly {
            every: 1,
            days: bit(Weekday::Wed),
        };
        assert!(wed.matches(start, d(2026, 11, 18)));
        assert!(!wed.matches(start, d(2026, 11, 19)));
        assert!(
            !wed.matches(start, d(2026, 11, 4)),
            "never before the start"
        );
        let thu = Repeat::Weekly {
            every: 2,
            days: bit(Weekday::Thu),
        };
        assert!(thu.matches(start, d(2026, 11, 12)));
        assert!(!thu.matches(start, d(2026, 11, 19)));
        assert!(thu.matches(start, d(2026, 11, 26)));
        assert_eq!(thu.label(), "격주 목요일");
        assert_eq!(wed.label(), "매주 수요일");
    }
    #[test]
    fn monthly_nth_weekday_and_last_weekday() {
        let start = d(2026, 11, 1);
        let second_wed = Repeat::MonthlyNth {
            nth: 2,
            weekday: Weekday::Wed,
        };
        assert!(second_wed.matches(start, d(2026, 11, 11)));
        assert!(!second_wed.matches(start, d(2026, 11, 18)));
        assert!(second_wed.matches(start, d(2026, 12, 9)));
        assert_eq!(second_wed.label(), "매월 둘째 주 수요일");
        let last_fri = Repeat::MonthlyNth {
            nth: -1,
            weekday: Weekday::Fri,
        };
        assert!(last_fri.matches(start, d(2026, 11, 27)));
        assert!(!last_fri.matches(start, d(2026, 11, 20)));
    }
    #[test]
    fn monthly_day_clamps_to_short_months() {
        let start = d(2026, 1, 31);
        assert!(Repeat::MonthlyDay.matches(start, d(2026, 2, 28)));
        assert!(Repeat::MonthlyDay.matches(start, d(2026, 3, 31)));
        assert!(!Repeat::MonthlyDay.matches(start, d(2026, 3, 30)));
    }
}
