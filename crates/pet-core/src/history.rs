//! A log of what happened to reminders and focus sessions, and the stats
//! shown in the dashboard's History tab.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// Oldest entries are dropped past this many.
pub const MAX_ENTRIES: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryKind {
    /// A reminder marked done.
    Done,
    Snoozed,
    /// A focus session completed.
    Focus,
    /// A suggested screen break that was taken.
    Break,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub at: NaiveDateTime,
    pub kind: HistoryKind,
    pub title: String,
    #[serde(default)]
    pub reminder_id: Option<u64>,
    /// How long a reminder waited before being answered.
    #[serde(default)]
    pub waited_minutes: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub done_today: u32,
    pub done_this_week: u32,
    pub snoozed_today: u32,
    pub focus_today: u32,
    pub breaks_today: u32,
    /// Done reminders answered within 5 minutes, as a share of all done (0..=100).
    pub on_time_percent: u32,
    /// Consecutive days, up to today, with at least one done reminder or focus session.
    pub current_streak: u32,
    pub best_streak: u32,
}

pub fn push(history: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    history.push(entry);
    if history.len() > MAX_ENTRIES {
        let extra = history.len() - MAX_ENTRIES;
        history.drain(..extra);
    }
}

pub fn stats(history: &[HistoryEntry], today: NaiveDate) -> Stats {
    let week_start = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let mut s = Stats::default();
    let mut done_total = 0u32;
    let mut on_time = 0u32;
    let mut active_days: Vec<NaiveDate> = Vec::new();

    for e in history {
        let day = e.at.date();
        let is_today = day == today;
        match e.kind {
            HistoryKind::Done => {
                done_total += 1;
                if e.waited_minutes <= 5 {
                    on_time += 1;
                }
                s.done_today += u32::from(is_today);
                s.done_this_week += u32::from(day >= week_start && day <= today);
                active_days.push(day);
            }
            HistoryKind::Snoozed => s.snoozed_today += u32::from(is_today),
            HistoryKind::Focus => {
                s.focus_today += u32::from(is_today);
                active_days.push(day);
            }
            HistoryKind::Break => s.breaks_today += u32::from(is_today),
        }
    }
    s.on_time_percent = (on_time * 100).checked_div(done_total).unwrap_or(0);

    active_days.sort_unstable();
    active_days.dedup();
    let mut run = 0u32;
    let mut prev: Option<NaiveDate> = None;
    for d in &active_days {
        run = if prev == Some(*d - Duration::days(1)) { run + 1 } else { 1 };
        s.best_streak = s.best_streak.max(run);
        prev = Some(*d);
    }
    // The streak is still alive today even if nothing is done yet.
    if let Some(last) = prev {
        if last == today || last == today - Duration::days(1) {
            s.current_streak = run;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn entry(d: u32, kind: HistoryKind, waited: u32) -> HistoryEntry {
        HistoryEntry {
            at: day(d).and_hms_opt(12, 0, 0).unwrap(),
            kind,
            title: "x".into(),
            reminder_id: None,
            waited_minutes: waited,
        }
    }

    #[test]
    fn counts_and_streaks() {
        use HistoryKind::*;
        // 2026-10-28 is a Wednesday; the week started on the 26th.
        let h = vec![
            entry(20, Done, 0),
            entry(21, Done, 0),
            entry(22, Focus, 0),
            entry(25, Done, 30),
            entry(26, Done, 0),
            entry(27, Done, 0),
            entry(28, Done, 2),
            entry(28, Snoozed, 0),
            entry(28, Focus, 0),
            entry(28, Break, 0),
        ];
        let s = stats(&h, day(28));
        assert_eq!((s.done_today, s.done_this_week, s.snoozed_today, s.focus_today, s.breaks_today), (1, 3, 1, 1, 1));
        assert_eq!((s.current_streak, s.best_streak), (4, 4));
        assert_eq!(s.on_time_percent, 5 * 100 / 6);

        // Nothing yet today: yesterday's streak still counts.
        let s = stats(&h[..6], day(28));
        assert_eq!(s.current_streak, 3);
        // A missed day breaks it.
        let s = stats(&h[..6], day(29));
        assert_eq!(s.current_streak, 0);
    }

    #[test]
    fn history_is_capped() {
        let mut h = Vec::new();
        for _ in 0..MAX_ENTRIES + 10 {
            push(&mut h, entry(1, HistoryKind::Done, 0));
        }
        assert_eq!(h.len(), MAX_ENTRIES);
    }
}
