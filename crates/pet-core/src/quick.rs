//! Turn a typed sentence into a reminder.
//!
//! "call mom tomorrow at 5pm", "stretch in 20 min", "water every 45 minutes",
//! "standup every mon and wed at 9:30". Whatever is not understood as a time
//! becomes the title. This is a small hand-written parser for common English
//! phrasings, not a general date library.

use crate::reminder::Repeat;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parsed {
    pub title: String,
    /// `None` when no time was found.
    pub due: Option<NaiveDateTime>,
    pub repeat: Repeat,
}

struct Tokens {
    orig: Vec<String>,
    low: Vec<String>,
    used: Vec<bool>,
}

impl Tokens {
    fn new(text: &str) -> Self {
        let orig: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
        let low = orig
            .iter()
            .map(|t| t.to_lowercase().trim_matches(|c: char| matches!(c, ',' | '.' | '!' | '?' | ';')).to_owned())
            .collect::<Vec<_>>();
        let used = vec![false; orig.len()];
        Self { orig, low, used }
    }

    fn len(&self) -> usize {
        self.low.len()
    }

    fn get(&self, i: usize) -> Option<&str> {
        self.low.get(i).filter(|_| !self.used[i]).map(String::as_str)
    }

    fn take(&mut self, range: std::ops::Range<usize>) {
        for i in range {
            self.used[i] = true;
        }
    }

    fn title(&self) -> String {
        let words: Vec<&str> = (0..self.len()).filter(|&i| !self.used[i]).map(|i| self.orig[i].as_str()).collect();
        let mut t = words.join(" ");
        // Drop dangling connectors left behind by removed time phrases.
        loop {
            let lower = t.to_lowercase();
            let before = t.len();
            for p in ["remind me to ", "remind me ", "don't forget to ", "dont forget to ", "to "] {
                if lower.starts_with(p) {
                    t = t[p.len()..].to_owned();
                    break;
                }
            }
            for s in [" at", " on", " in", " by", " for", " and", ","] {
                if t.to_lowercase().ends_with(s) {
                    t.truncate(t.len() - s.len());
                }
            }
            t = t.trim().to_owned();
            if t.len() == before {
                break;
            }
        }
        t
    }
}

fn number(word: &str) -> Option<u32> {
    if let Ok(n) = word.parse::<u32>() {
        return Some(n);
    }
    let words = ["a", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
    match word {
        "an" => Some(1),
        _ => words.iter().position(|w| *w == word).map(|i| i.max(1) as u32),
    }
}

/// Minutes in a unit word, e.g. "min" -> 1, "hours" -> 60.
fn unit_minutes(word: &str) -> Option<u32> {
    match word {
        "m" | "min" | "mins" | "minute" | "minutes" => Some(1),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(60),
        "d" | "day" | "days" => Some(24 * 60),
        "w" | "week" | "weeks" => Some(7 * 24 * 60),
        _ => None,
    }
}

/// "20min", "2h" written as one token.
fn joined_amount(word: &str) -> Option<u32> {
    let split = word.find(|c: char| !c.is_ascii_digit())?;
    let (n, unit) = word.split_at(split);
    Some(n.parse::<u32>().ok()? * unit_minutes(unit)?)
}

fn weekday(word: &str) -> Option<Weekday> {
    let w = word.trim_end_matches('s'); // "mondays"
    Some(match w {
        "mon" | "monday" => Weekday::Mon,
        "tue" | "tues" | "tuesday" => Weekday::Tue,
        "wed" | "wednesday" => Weekday::Wed,
        "thu" | "thur" | "thurs" | "thursday" => Weekday::Thu,
        "fri" | "friday" => Weekday::Fri,
        "sat" | "saturday" => Weekday::Sat,
        "sun" | "sunday" => Weekday::Sun,
        _ => return None,
    })
}

fn month(word: &str) -> Option<u32> {
    let names = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    if word.len() < 3 {
        return None;
    }
    let full = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    names
        .iter()
        .zip(full.iter())
        .position(|(short, long)| word == *short || word == *long || (word == "sept" && *short == "sep"))
        .map(|i| i as u32 + 1)
}

fn day_of_month(word: &str) -> Option<u32> {
    let digits = word.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &word[digits.len()..];
    if !matches!(suffix, "" | "st" | "nd" | "rd" | "th") {
        return None;
    }
    digits.parse::<u32>().ok().filter(|d| (1..=31).contains(d))
}

/// A clock time. `ampm` is `None` when the user did not say am/pm.
#[derive(Clone, Copy)]
struct Clock {
    hour: u32,
    minute: u32,
    ampm: Option<bool>, // Some(true) = pm
}

fn clock(word: &str, next: Option<&str>) -> Option<(Clock, bool)> {
    let (body, mut ampm) = if let Some(b) = word.strip_suffix("am") {
        (b, Some(false))
    } else if let Some(b) = word.strip_suffix("pm") {
        (b, Some(true))
    } else {
        (word, None)
    };
    let mut used_next = false;
    if ampm.is_none() {
        match next {
            Some("am" | "a.m" | "a.m.") => {
                ampm = Some(false);
                used_next = true;
            }
            Some("pm" | "p.m" | "p.m.") => {
                ampm = Some(true);
                used_next = true;
            }
            _ => {}
        }
    }
    let (h, m) = match body.split_once([':', '.']) {
        Some((h, m)) => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        None => (body.parse::<u32>().ok()?, 0),
    };
    if m > 59 || h > 23 || (ampm.is_some() && !(1..=12).contains(&h)) {
        return None;
    }
    Some((Clock { hour: h, minute: m, ampm }, used_next))
}

/// Parse `text` relative to `now`.
pub fn parse(text: &str, now: NaiveDateTime) -> Parsed {
    let mut t = Tokens::new(text);
    let now = now.with_second(0).unwrap_or(now).with_nanosecond(0).unwrap_or(now);

    let mut repeat = Repeat::Once;
    let mut relative: Option<Duration> = None;
    let mut date: Option<NaiveDate> = None;
    let mut time: Option<Clock> = None;
    let mut default_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();

    let mut i = 0;
    while i < t.len() {
        let Some(w) = t.get(i).map(str::to_owned) else {
            i += 1;
            continue;
        };
        let n1 = t.get(i + 1).map(str::to_owned);
        let n2 = t.get(i + 2).map(str::to_owned);

        // ---- repeats ----
        if w == "daily" || w == "everyday" {
            repeat = Repeat::Daily;
            t.take(i..i + 1);
        } else if w == "hourly" {
            repeat = Repeat::EveryMinutes { minutes: 60 };
            t.take(i..i + 1);
        } else if w == "weekdays" || (w == "on" && n1.as_deref() == Some("weekdays")) {
            repeat = Repeat::Weekdays;
            t.take(i..i + if w == "on" { 2 } else { 1 });
        } else if w == "every" || w == "each" {
            let n1s = n1.as_deref().unwrap_or("");
            if matches!(n1s, "day" | "morning" | "evening" | "night") {
                repeat = Repeat::Daily;
                match n1s {
                    "evening" => default_time = NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                    "night" => default_time = NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
                    _ => {}
                }
                t.take(i..i + 2);
            } else if matches!(n1s, "weekday" | "weekdays" | "workday") {
                repeat = Repeat::Weekdays;
                t.take(i..i + 2);
            } else if n1s == "half" && n2.as_deref() == Some("hour") {
                repeat = Repeat::EveryMinutes { minutes: 30 };
                t.take(i..i + 3);
            } else if let Some(m) = unit_minutes(n1s) {
                repeat = if m == 24 * 60 { Repeat::Daily } else { Repeat::EveryMinutes { minutes: m } };
                t.take(i..i + 2);
            } else if let Some(m) = joined_amount(n1s) {
                repeat = Repeat::EveryMinutes { minutes: m };
                t.take(i..i + 2);
            } else if let (Some(n), Some(m)) = (number(n1s), n2.as_deref().and_then(unit_minutes)) {
                repeat = if m == 24 * 60 && n == 1 { Repeat::Daily } else { Repeat::EveryMinutes { minutes: n * m } };
                t.take(i..i + 3);
            } else if weekday(n1s).is_some() {
                // every mon, wed and fri
                let mut days = vec![];
                let mut j = i + 1;
                while let Some(tok) = t.get(j) {
                    if let Some(d) = weekday(tok) {
                        days.push(d.num_days_from_monday() as u8);
                    } else if !matches!(tok, "and" | "&" | ",") {
                        break;
                    }
                    j += 1;
                }
                // Do not swallow a trailing "and".
                while j > i + 1 && matches!(t.get(j - 1), Some("and" | "&" | ",")) {
                    j -= 1;
                }
                days.sort_unstable();
                days.dedup();
                repeat = Repeat::Weekly { days };
                t.take(i..j);
                i = j;
                continue;
            }
        }
        // ---- relative: in 20 min / in an hour / in half an hour ----
        else if w == "in" {
            let n1s = n1.as_deref().unwrap_or("");
            if n1s == "half" && n2.as_deref() == Some("an") && t.get(i + 3) == Some("hour") {
                relative = Some(Duration::minutes(30));
                t.take(i..i + 4);
            } else if let Some(m) = joined_amount(n1s) {
                relative = Some(Duration::minutes(i64::from(m)));
                t.take(i..i + 2);
            } else if let (Some(n), Some(m)) = (number(n1s), n2.as_deref().and_then(unit_minutes)) {
                relative = Some(Duration::minutes(i64::from(n * m)));
                t.take(i..i + 3);
            } else if n1s == "the" {
                if let Some(tod) = n2.as_deref().and_then(part_of_day) {
                    default_time = tod;
                    t.take(i..i + 3);
                }
            }
        }
        // ---- days ----
        else if w == "today" {
            date = Some(now.date());
            t.take(i..i + 1);
        } else if w == "tonight" {
            date = Some(now.date());
            default_time = NaiveTime::from_hms_opt(20, 0, 0).unwrap();
            t.take(i..i + 1);
        } else if matches!(w.as_str(), "tomorrow" | "tmr" | "tmrw" | "tomorow") {
            date = Some(now.date() + Duration::days(1));
            t.take(i..i + 1);
        } else if (w == "on" || w == "next" || w == "this") && n1.as_deref().and_then(weekday).is_some() {
            let d = weekday(n1.as_deref().unwrap()).unwrap();
            date = Some(next_weekday(now.date(), d, w == "next"));
            t.take(i..i + 2);
        } else if let Some(d) = weekday(&w) {
            date = Some(next_weekday(now.date(), d, false));
            t.take(i..i + 1);
        } else if let Some(d) = parse_iso_date(&w) {
            date = Some(d);
            t.take(i..i + 1);
        } else if let (Some(m), Some(d)) = (month(&w), n1.as_deref().and_then(day_of_month)) {
            date = month_day(now.date(), m, d);
            t.take(i..i + 2);
        } else if let (Some(d), Some(m)) = (day_of_month(&w), n1.as_deref().and_then(month)) {
            date = month_day(now.date(), m, d);
            let len = if i > 0 && t.get(i - 1) == Some("on") { i - 1 } else { i };
            t.take(len..i + 2);
        }
        // ---- times ----
        else if w == "noon" || w == "midday" {
            time = Some(Clock { hour: 12, minute: 0, ampm: None });
            t.take(i..i + 1);
        } else if w == "midnight" {
            time = Some(Clock { hour: 0, minute: 0, ampm: None });
            t.take(i..i + 1);
        } else if w == "at" || w == "@" {
            if let Some((c, used_next)) = n1.as_deref().and_then(|n| clock(n, n2.as_deref())) {
                time = Some(c);
                t.take(i..i + 2 + usize::from(used_next));
            } else if matches!(n1.as_deref(), Some("noon" | "midday")) {
                time = Some(Clock { hour: 12, minute: 0, ampm: None });
                t.take(i..i + 2);
            }
        } else if let Some((c, used_next)) = clock(&w, n1.as_deref()) {
            // A bare number is only a time with am/pm or a colon ("5pm", "17:30").
            if c.ampm.is_some() || w.contains(':') {
                time = Some(c);
                t.take(i..i + 1 + usize::from(used_next));
            }
        } else if let Some(tod) = part_of_day(&w) {
            // "tomorrow morning", "this evening"
            let prev = if i > 0 { t.low.get(i - 1).map(String::as_str) } else { None };
            if t.used.get(i.wrapping_sub(1)).copied().unwrap_or(false) || prev == Some("this") {
                default_time = tod;
                let start = if prev == Some("this") { i - 1 } else { i };
                t.take(start..i + 1);
            }
        }
        i += 1;
    }

    let title = t.title();
    let title = if title.is_empty() { "Reminder".to_owned() } else { title };

    let due = if let Some(d) = relative { Some(now + d) } else { resolve(now, date, time, default_time, &repeat) };
    Parsed { title, due, repeat }
}

fn part_of_day(word: &str) -> Option<NaiveTime> {
    let h = match word {
        "morning" => 9,
        "afternoon" => 15,
        "evening" => 18,
        "night" => 21,
        _ => return None,
    };
    NaiveTime::from_hms_opt(h, 0, 0)
}

fn next_weekday(today: NaiveDate, day: Weekday, skip_this_week: bool) -> NaiveDate {
    // Today counts for a plain weekday name; "next friday" on a Friday means a week out.
    let ahead = (7 + day.num_days_from_monday() as i64 - today.weekday().num_days_from_monday() as i64) % 7;
    let ahead = if ahead == 0 && skip_this_week { 7 } else { ahead };
    today + Duration::days(ahead)
}

fn parse_iso_date(word: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(word, "%Y-%m-%d").ok()
}

/// The next date with this month and day (this year, or next year if passed).
fn month_day(today: NaiveDate, month: u32, day: u32) -> Option<NaiveDate> {
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if this_year >= today {
        Some(this_year)
    } else {
        NaiveDate::from_ymd_opt(today.year() + 1, month, day)
    }
}

fn resolve(
    now: NaiveDateTime,
    date: Option<NaiveDate>,
    time: Option<Clock>,
    default_time: NaiveTime,
    repeat: &Repeat,
) -> Option<NaiveDateTime> {
    let has_time_hint = time.is_some() || date.is_some() || default_time != NaiveTime::from_hms_opt(9, 0, 0).unwrap();
    if let Repeat::EveryMinutes { minutes } = repeat {
        if time.is_none() && date.is_none() {
            return Some(now + Duration::minutes(i64::from(*minutes)));
        }
    }
    if !has_time_hint && *repeat == Repeat::Once {
        return None;
    }

    let day = date.unwrap_or(now.date());
    let candidate = match time {
        Some(Clock { hour, minute, ampm: Some(pm) }) => {
            let h = (hour % 12) + if pm { 12 } else { 0 };
            day.and_hms_opt(h, minute, 0)?
        }
        Some(Clock { hour, minute, ampm: None })
            if hour <= 12 && hour != 0 && date.is_none() && *repeat == Repeat::Once =>
        {
            // "at 5": the next 5 o'clock, morning or evening.
            let am = day.and_hms_opt(hour % 12, minute, 0)?;
            let pm = day.and_hms_opt(hour % 12 + 12, minute, 0)?;
            if am > now {
                am
            } else if pm > now {
                pm
            } else {
                am + Duration::days(1)
            }
        }
        Some(Clock { hour, minute, ampm: None }) => {
            // With a date or a repeat, a bare 1..7 most likely means the afternoon.
            let h = if (1..=7).contains(&hour) { hour + 12 } else { hour };
            day.and_hms_opt(h, minute, 0)?
        }
        None => day.and_time(default_time),
    };

    match repeat {
        Repeat::Once => {
            if candidate <= now && date.is_none() {
                Some(candidate + Duration::days(1))
            } else {
                Some(candidate)
            }
        }
        _ => {
            // First occurrence of the rule at or after the candidate.
            let before = candidate - Duration::seconds(1);
            repeat.next_after(candidate, before.max(now)).or(Some(candidate))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-10-28 is a Wednesday.
    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(10, 15, 42).unwrap()
    }

    fn at(m: u32, d: u32, h: u32, min: u32) -> Option<NaiveDateTime> {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap().and_hms_opt(h, min, 0)
    }

    fn p(text: &str) -> Parsed {
        parse(text, now())
    }

    #[test]
    fn relative_times() {
        assert_eq!(
            p("stretch in 20 min"),
            Parsed { title: "stretch".into(), due: at(10, 28, 10, 35), repeat: Repeat::Once }
        );
        assert_eq!(p("in an hour check oven").due, at(10, 28, 11, 15));
        assert_eq!(p("in half an hour call back").due, at(10, 28, 10, 45));
        assert_eq!(p("tea in 5m").due, at(10, 28, 10, 20));
        assert_eq!(p("pay rent in 2 days").due, at(10, 30, 10, 15));
        assert_eq!(p("tea in 5m").title, "tea");
    }

    #[test]
    fn days_and_clock_times() {
        let r = p("Call mom tomorrow at 5pm");
        assert_eq!((r.title.as_str(), r.due), ("Call mom", at(10, 29, 17, 0)));
        assert_eq!(p("meeting at 17:30").due, at(10, 28, 17, 30));
        assert_eq!(p("meeting at 9:30").due, at(10, 28, 21, 30), "9:30 already passed today, so 21:30");
        assert_eq!(p("lunch at noon").due, at(10, 28, 12, 0));
        assert_eq!(p("lunch at 1").due, at(10, 28, 13, 0));
        assert_eq!(p("standup 9 am tomorrow").due, at(10, 29, 9, 0));
        assert_eq!(p("gym tonight").due, at(10, 28, 20, 0));
        assert_eq!(p("report tomorrow morning").due, at(10, 29, 9, 0));
        assert_eq!(p("call this evening").due, at(10, 28, 18, 0));
        assert_eq!(p("dentist friday at 4").due, at(10, 30, 16, 0));
        assert_eq!(p("dentist on friday").due, at(10, 30, 9, 0));
        assert_eq!(p("review next wednesday 3pm").due, at(11, 4, 15, 0));
        assert_eq!(p("party dec 25th 7pm").due, at(12, 25, 19, 0));
        assert_eq!(p("party on 5 nov").due, at(11, 5, 9, 0));
        assert_eq!(p("party on 5 nov").title, "party");
        assert_eq!(p("taxes 2026-11-30").due, at(11, 30, 9, 0));
        assert_eq!(p("Remind me to buy milk at 6pm").title, "buy milk");
    }

    #[test]
    fn repeats() {
        let r = p("water every 45 minutes");
        assert_eq!(
            (r.title.as_str(), r.repeat.clone(), r.due),
            ("water", Repeat::EveryMinutes { minutes: 45 }, at(10, 28, 11, 0))
        );
        assert_eq!(p("stand up every hour").repeat, Repeat::EveryMinutes { minutes: 60 });
        assert_eq!(p("stand up every 2h").repeat, Repeat::EveryMinutes { minutes: 120 });
        assert_eq!(p("blink every half hour").repeat, Repeat::EveryMinutes { minutes: 30 });

        let r = p("vitamins every day at 8am");
        assert_eq!((r.repeat.clone(), r.due), (Repeat::Daily, at(10, 29, 8, 0)));
        let r = p("vitamins daily at 8pm");
        assert_eq!((r.repeat.clone(), r.due), (Repeat::Daily, at(10, 28, 20, 0)));

        let r = p("standup every mon and wed at 9:30");
        assert_eq!(r.title, "standup");
        assert_eq!(r.repeat, Repeat::Weekly { days: vec![0, 2] });
        assert_eq!(r.due, at(11, 2, 9, 30), "Wednesday 9:30 has passed, so next Monday");

        let r = p("timesheet every weekday at 5pm");
        assert_eq!((r.repeat.clone(), r.due), (Repeat::Weekdays, at(10, 28, 17, 0)));
        assert_eq!(p("journal every evening").due, at(10, 28, 18, 0));
    }

    #[test]
    fn plain_text_has_no_time() {
        let r = p("buy 5 apples");
        assert_eq!((r.title.as_str(), r.due), ("buy 5 apples", None));
        assert_eq!(p("").title, "Reminder");
        assert_eq!(p("at").due, None);
    }
}
