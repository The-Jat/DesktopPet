//! Reminders and their repeat rules.
//!
//! All times are local wall-clock times (`NaiveDateTime`). That keeps "every
//! day at 09:00" meaning 09:00 on the clock, which is what a person expects
//! from a desk reminder.

use chrono::{Datelike, Duration, NaiveDateTime, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Repeat {
    Once,
    Daily,
    /// Monday to Friday.
    Weekdays,
    /// On the given days, 0 = Monday ... 6 = Sunday.
    Weekly {
        days: Vec<u8>,
    },
    EveryMinutes {
        minutes: u32,
    },
}

impl Repeat {
    /// The first occurrence strictly after `after`, keeping the time of day
    /// (or interval phase) of `due`. `None` when the rule never repeats.
    pub fn next_after(&self, due: NaiveDateTime, after: NaiveDateTime) -> Option<NaiveDateTime> {
        match self {
            Repeat::Once => None,
            Repeat::EveryMinutes { minutes } => {
                let step = Duration::minutes(i64::from((*minutes).max(1)));
                if after < due {
                    return Some(due);
                }
                let elapsed = (after - due).num_seconds();
                let k = elapsed / step.num_seconds() + 1;
                Some(due + step * k as i32)
            }
            Repeat::Daily | Repeat::Weekdays | Repeat::Weekly { .. } => {
                // Never schedule before the reminder's first occurrence.
                let start = after.date().max(due.date());
                (0..15)
                    .map(|i| (start + Duration::days(i)).and_time(due.time()))
                    .find(|c| *c > after && self.matches_day(c.weekday(), due.weekday()))
            }
        }
    }

    fn matches_day(&self, day: Weekday, due_day: Weekday) -> bool {
        match self {
            Repeat::Daily => true,
            Repeat::Weekdays => day.num_days_from_monday() < 5,
            Repeat::Weekly { days } if days.is_empty() => day == due_day,
            Repeat::Weekly { days } => days.contains(&(day.num_days_from_monday() as u8)),
            Repeat::Once | Repeat::EveryMinutes { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub details: String,
    pub due: NaiveDateTime,
    pub repeat: Repeat,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub snoozed_until: Option<NaiveDateTime>,
    #[serde(default)]
    pub completed_at: Option<NaiveDateTime>,
}

fn yes() -> bool {
    true
}

impl Reminder {
    pub fn new(id: u64, title: impl Into<String>, due: NaiveDateTime, repeat: Repeat) -> Self {
        Self {
            id,
            title: title.into(),
            details: String::new(),
            due,
            repeat,
            enabled: true,
            snoozed_until: None,
            completed_at: None,
        }
    }

    /// When the pet should next speak up about this reminder.
    pub fn fire_at(&self) -> NaiveDateTime {
        self.snoozed_until.unwrap_or(self.due)
    }

    pub fn is_due(&self, now: NaiveDateTime) -> bool {
        self.enabled && self.completed_at.is_none() && self.fire_at() <= now
    }

    /// Mark the current occurrence as done. Repeating reminders move on to
    /// their next occurrence after `now`; one-off reminders are completed.
    pub fn complete(&mut self, now: NaiveDateTime) {
        self.snoozed_until = None;
        match self.repeat.next_after(self.due, now) {
            Some(next) => self.due = next,
            None => self.completed_at = Some(now),
        }
    }

    pub fn snooze(&mut self, now: NaiveDateTime, minutes: u32) {
        self.snoozed_until = Some(now + Duration::minutes(i64::from(minutes.max(1))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, 0).unwrap()
    }

    // 2026-10-26 is a Monday.

    #[test]
    fn once_never_repeats() {
        assert_eq!(Repeat::Once.next_after(at(2026, 10, 26, 9, 0), at(2026, 10, 26, 9, 5)), None);
    }

    #[test]
    fn daily_keeps_time_of_day() {
        let due = at(2026, 10, 26, 9, 0);
        assert_eq!(Repeat::Daily.next_after(due, at(2026, 10, 26, 9, 0)), Some(at(2026, 10, 27, 9, 0)));
        assert_eq!(Repeat::Daily.next_after(due, at(2026, 10, 26, 8, 0)), Some(at(2026, 10, 26, 9, 0)));
        // Missed for several days: jumps to the next one, not the oldest.
        assert_eq!(Repeat::Daily.next_after(due, at(2026, 10, 30, 12, 0)), Some(at(2026, 10, 31, 9, 0)));
    }

    #[test]
    fn weekdays_skip_weekend() {
        let due = at(2026, 10, 30, 9, 0); // Friday
        assert_eq!(Repeat::Weekdays.next_after(due, due), Some(at(2026, 11, 2, 9, 0)));
    }

    #[test]
    fn weekly_on_chosen_days() {
        let due = at(2026, 10, 26, 18, 30); // Monday
        let rule = Repeat::Weekly { days: vec![0, 3] }; // Mon, Thu
        assert_eq!(rule.next_after(due, due), Some(at(2026, 10, 29, 18, 30)));
        assert_eq!(rule.next_after(due, at(2026, 10, 29, 19, 0)), Some(at(2026, 11, 2, 18, 30)));
    }

    #[test]
    fn weekly_without_days_uses_due_weekday() {
        let due = at(2026, 10, 28, 7, 0); // Wednesday
        let rule = Repeat::Weekly { days: vec![] };
        assert_eq!(rule.next_after(due, due), Some(at(2026, 11, 4, 7, 0)));
    }

    #[test]
    fn every_minutes_keeps_phase() {
        let due = at(2026, 10, 26, 9, 0);
        let rule = Repeat::EveryMinutes { minutes: 45 };
        assert_eq!(rule.next_after(due, at(2026, 10, 26, 9, 0)), Some(at(2026, 10, 26, 9, 45)));
        assert_eq!(rule.next_after(due, at(2026, 10, 26, 10, 50)), Some(at(2026, 10, 26, 11, 15)));
        assert_eq!(rule.next_after(due, at(2026, 10, 26, 8, 0)), Some(due));
    }

    #[test]
    fn complete_and_snooze() {
        let now = at(2026, 10, 26, 9, 1);
        let mut once = Reminder::new(1, "Call mom", at(2026, 10, 26, 9, 0), Repeat::Once);
        assert!(once.is_due(now));
        once.snooze(now, 10);
        assert!(!once.is_due(now));
        assert!(once.is_due(at(2026, 10, 26, 9, 11)));
        once.complete(at(2026, 10, 26, 9, 11));
        assert!(!once.is_due(at(2026, 10, 27, 0, 0)));
        assert!(once.completed_at.is_some());

        let mut daily = Reminder::new(2, "Water", at(2026, 10, 26, 9, 0), Repeat::Daily);
        daily.complete(now);
        assert_eq!(daily.due, at(2026, 10, 27, 9, 0));
        assert!(daily.completed_at.is_none());
    }

    #[test]
    fn disabled_is_never_due() {
        let mut r = Reminder::new(1, "x", at(2026, 1, 1, 0, 0), Repeat::Once);
        r.enabled = false;
        assert!(!r.is_due(at(2027, 1, 1, 0, 0)));
    }
}
