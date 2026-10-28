//! Everything the app saves to disk, kept in one JSON file.

use crate::history::{self, HistoryEntry, HistoryKind};
use crate::mood::Mood;
use crate::reminder::{Reminder, Repeat};
use crate::settings::Settings;
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: u64,
    pub text: String,
    pub created: NaiveDateTime,
    #[serde(default)]
    pub category_id: Option<u64>,
}

/// A label with a colour for grouping reminders and notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: u64,
    pub name: String,
    /// `#rrggbb`
    pub color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    /// Add the file's reminders, notes and categories to what is here.
    Merge,
    /// Replace everything, settings included.
    Replace,
}

/// A reminder as typed in the dashboard, before it gets an id.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraft {
    pub title: String,
    #[serde(default)]
    pub details: String,
    pub due: NaiveDateTime,
    pub repeat: Repeat,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub category_id: Option<u64>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Data {
    pub version: u32,
    pub next_id: u64,
    pub reminders: Vec<Reminder>,
    pub notes: Vec<Note>,
    pub categories: Vec<Category>,
    pub history: Vec<HistoryEntry>,
    pub settings: Settings,
    /// The pet's mood, kept across restarts.
    pub mood: Mood,
    /// Last day the daily summary was shown.
    pub last_summary: Option<NaiveDate>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            reminders: vec![],
            notes: vec![],
            categories: vec![],
            history: vec![],
            settings: Settings::default(),
            mood: Mood::default(),
            last_summary: None,
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Corrupt(serde_json::Error),
    Invalid(&'static str),
    NotFound(u64),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "file error: {e}"),
            StoreError::Corrupt(e) => write!(f, "data file is not valid: {e}"),
            StoreError::Invalid(why) => f.write_str(why),
            StoreError::NotFound(id) => write!(f, "no item with id {id}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl Data {
    /// Load from `path`. A missing file is not an error: it gives fresh data.
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_json(&text),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Data::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Parse and repair data from JSON (a saved file or an export).
    pub fn from_json(text: &str) -> Result<Self, StoreError> {
        // Insist on an object with our version field, so an unrelated JSON
        // file (or `[]`) is never mistaken for empty data.
        let value: serde_json::Value = serde_json::from_str(text).map_err(StoreError::Corrupt)?;
        if value.get("version").and_then(serde_json::Value::as_u64).is_none() {
            return Err(StoreError::Invalid("this is not a DesktopPet data file"));
        }
        let mut data: Data = serde_json::from_value(value).map_err(StoreError::Corrupt)?;
        data.settings = data.settings.sanitized();
        let max_id = data
            .reminders
            .iter()
            .map(|r| r.id)
            .chain(data.notes.iter().map(|n| n.id))
            .chain(data.categories.iter().map(|c| c.id))
            .max();
        data.next_id = data.next_id.max(max_id.map_or(1, |m| m + 1));
        // Drop links to categories that do not exist.
        let known: Vec<u64> = data.categories.iter().map(|c| c.id).collect();
        for r in &mut data.reminders {
            r.category_id = r.category_id.filter(|c| known.contains(c));
        }
        for n in &mut data.notes {
            n.category_id = n.category_id.filter(|c| known.contains(c));
        }
        Ok(data)
    }

    pub fn to_json(&self) -> Result<String, StoreError> {
        serde_json::to_string_pretty(self).map_err(StoreError::Corrupt)
    }

    /// Bring in data from an export file.
    pub fn import(&mut self, other: Data, mode: ImportMode) {
        if mode == ImportMode::Replace {
            *self = other;
            return;
        }
        // Categories are matched by name so importing twice does not duplicate them.
        let mut cat_map: HashMap<u64, u64> = HashMap::new();
        for c in other.categories {
            let existing = self.categories.iter().find(|m| m.name.eq_ignore_ascii_case(&c.name)).map(|m| m.id);
            let id = existing.unwrap_or_else(|| {
                let id = self.take_id();
                self.categories.push(Category { id, ..c.clone() });
                id
            });
            cat_map.insert(c.id, id);
        }
        for mut r in other.reminders {
            r.id = self.take_id();
            r.category_id = r.category_id.and_then(|c| cat_map.get(&c).copied());
            self.reminders.push(r);
        }
        for mut n in other.notes {
            n.id = self.take_id();
            n.category_id = n.category_id.and_then(|c| cat_map.get(&c).copied());
            self.notes.push(n);
        }
        for e in other.history {
            history::push(&mut self.history, HistoryEntry { reminder_id: None, ..e });
        }
        self.history.sort_by_key(|e| e.at);
    }

    /// Write to a temporary file first, then rename, so a crash mid-write
    /// never leaves a half-written data file behind.
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = self.to_json()?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_reminder(&mut self, draft: ReminderDraft) -> Result<u64, StoreError> {
        let draft = self.validate(draft)?;
        let id = self.take_id();
        let mut r = Reminder::new(id, draft.title, draft.due, draft.repeat);
        r.details = draft.details;
        r.enabled = draft.enabled;
        r.category_id = draft.category_id;
        self.reminders.push(r);
        Ok(id)
    }

    /// Replace a reminder's fields. Clears any snooze or completion so the
    /// edited reminder is live again.
    pub fn update_reminder(&mut self, id: u64, draft: ReminderDraft) -> Result<(), StoreError> {
        let draft = self.validate(draft)?;
        let r = self.reminder_mut(id)?;
        r.category_id = draft.category_id;
        r.title = draft.title;
        r.details = draft.details;
        r.due = draft.due;
        r.repeat = draft.repeat;
        r.enabled = draft.enabled;
        r.snoozed_until = None;
        r.completed_at = None;
        Ok(())
    }

    pub fn delete_reminder(&mut self, id: u64) -> Result<(), StoreError> {
        let before = self.reminders.len();
        self.reminders.retain(|r| r.id != id);
        if self.reminders.len() == before {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }

    pub fn reminder_mut(&mut self, id: u64) -> Result<&mut Reminder, StoreError> {
        self.reminders.iter_mut().find(|r| r.id == id).ok_or(StoreError::NotFound(id))
    }

    /// Reminders waiting for the user, oldest first.
    pub fn due_reminders(&self, now: NaiveDateTime) -> Vec<&Reminder> {
        let mut due: Vec<_> = self.reminders.iter().filter(|r| r.is_due(now)).collect();
        due.sort_by_key(|r| (r.fire_at(), r.id));
        due
    }

    /// Answer a due reminder and log it.
    pub fn complete_reminder(&mut self, id: u64, now: NaiveDateTime) -> Result<(), StoreError> {
        let r = self.reminder_mut(id)?;
        let waited = (now - r.fire_at()).num_minutes().max(0) as u32;
        let title = r.title.clone();
        r.complete(now);
        self.log(HistoryEntry {
            at: now,
            kind: HistoryKind::Done,
            title,
            reminder_id: Some(id),
            waited_minutes: waited,
        });
        Ok(())
    }

    pub fn snooze_reminder(&mut self, id: u64, now: NaiveDateTime, minutes: u32) -> Result<(), StoreError> {
        let r = self.reminder_mut(id)?;
        let waited = (now - r.fire_at()).num_minutes().max(0) as u32;
        let title = r.title.clone();
        r.snooze(now, minutes);
        self.log(HistoryEntry {
            at: now,
            kind: HistoryKind::Snoozed,
            title,
            reminder_id: Some(id),
            waited_minutes: waited,
        });
        Ok(())
    }

    pub fn log(&mut self, entry: HistoryEntry) {
        history::push(&mut self.history, entry);
    }

    /// Enabled, not-completed reminders that fire on `day`, in time order.
    pub fn reminders_on(&self, day: NaiveDate) -> Vec<&Reminder> {
        let start = day.and_hms_opt(0, 0, 0).unwrap();
        let mut list: Vec<&Reminder> = self
            .reminders
            .iter()
            .filter(|r| r.enabled && r.completed_at.is_none())
            .filter(|r| {
                let fire = r.fire_at();
                if fire.date() == day {
                    return true;
                }
                // A repeating reminder whose next time is before today may still
                // repeat today.
                fire < start
                    && r.repeat.next_after(r.due, start - chrono::Duration::seconds(1)).is_some_and(|n| n.date() == day)
            })
            .collect();
        list.sort_by_key(|r| r.fire_at().time());
        list
    }

    pub fn add_note(&mut self, text: &str, now: NaiveDateTime) -> Result<u64, StoreError> {
        self.add_note_in(text, now, None)
    }

    pub fn add_note_in(&mut self, text: &str, now: NaiveDateTime, category_id: Option<u64>) -> Result<u64, StoreError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(StoreError::Invalid("note is empty"));
        }
        let category_id = self.check_category(category_id)?;
        let id = self.take_id();
        self.notes.push(Note { id, text: text.chars().take(2000).collect(), created: now, category_id });
        Ok(id)
    }

    pub fn add_category(&mut self, name: &str, color: &str) -> Result<u64, StoreError> {
        let (name, color) = clean_category(name, color)?;
        if self.categories.iter().any(|c| c.name.eq_ignore_ascii_case(&name)) {
            return Err(StoreError::Invalid("a category with that name exists"));
        }
        let id = self.take_id();
        self.categories.push(Category { id, name, color });
        Ok(id)
    }

    pub fn update_category(&mut self, id: u64, name: &str, color: &str) -> Result<(), StoreError> {
        let (name, color) = clean_category(name, color)?;
        if self.categories.iter().any(|c| c.id != id && c.name.eq_ignore_ascii_case(&name)) {
            return Err(StoreError::Invalid("a category with that name exists"));
        }
        let c = self.categories.iter_mut().find(|c| c.id == id).ok_or(StoreError::NotFound(id))?;
        c.name = name;
        c.color = color;
        Ok(())
    }

    /// Delete a category; its reminders and notes become uncategorised.
    pub fn delete_category(&mut self, id: u64) -> Result<(), StoreError> {
        let before = self.categories.len();
        self.categories.retain(|c| c.id != id);
        if self.categories.len() == before {
            return Err(StoreError::NotFound(id));
        }
        for r in self.reminders.iter_mut().filter(|r| r.category_id == Some(id)) {
            r.category_id = None;
        }
        for n in self.notes.iter_mut().filter(|n| n.category_id == Some(id)) {
            n.category_id = None;
        }
        Ok(())
    }

    /// Find or create a category by name (used by reminder presets).
    pub fn category_named(&mut self, name: &str, color: &str) -> Result<u64, StoreError> {
        match self.categories.iter().find(|c| c.name.eq_ignore_ascii_case(name.trim())) {
            Some(c) => Ok(c.id),
            None => self.add_category(name, color),
        }
    }

    fn check_category(&self, id: Option<u64>) -> Result<Option<u64>, StoreError> {
        match id {
            Some(id) if !self.categories.iter().any(|c| c.id == id) => Err(StoreError::NotFound(id)),
            other => Ok(other),
        }
    }

    pub fn delete_note(&mut self, id: u64) -> Result<(), StoreError> {
        let before = self.notes.len();
        self.notes.retain(|n| n.id != id);
        if self.notes.len() == before {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }
}

fn clean_category(name: &str, color: &str) -> Result<(String, String), StoreError> {
    let name: String = name.trim().chars().take(40).collect();
    if name.is_empty() {
        return Err(StoreError::Invalid("category needs a name"));
    }
    let color = color.trim().to_ascii_lowercase();
    let valid = color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(StoreError::Invalid("colour must look like #33aa66"));
    }
    Ok((name, color))
}

impl Data {
    fn validate(&self, draft: ReminderDraft) -> Result<ReminderDraft, StoreError> {
        let mut draft = validate(draft)?;
        draft.category_id = self.check_category(draft.category_id)?;
        Ok(draft)
    }
}

fn validate(mut draft: ReminderDraft) -> Result<ReminderDraft, StoreError> {
    draft.title = draft.title.trim().chars().take(200).collect();
    if draft.title.is_empty() {
        return Err(StoreError::Invalid("reminder needs a title"));
    }
    if let Repeat::EveryMinutes { minutes: 0 } = draft.repeat {
        return Err(StoreError::Invalid("interval must be at least 1 minute"));
    }
    if let Repeat::Weekly { days } = &mut draft.repeat {
        days.retain(|d| *d < 7);
        days.sort_unstable();
        days.dedup();
    }
    Ok(draft)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 27).unwrap().and_hms_opt(h, m, 0).unwrap()
    }

    fn draft(title: &str, due: NaiveDateTime) -> ReminderDraft {
        ReminderDraft {
            title: title.into(),
            details: String::new(),
            due,
            repeat: Repeat::Once,
            enabled: true,
            category_id: None,
        }
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("data.json");
        assert_eq!(Data::load(&path).unwrap(), Data::default());

        let mut data = Data::default();
        data.add_reminder(draft("Stretch", at(10, 0))).unwrap();
        data.add_note("Buy bamboo", at(9, 0)).unwrap();
        data.settings.pet_name = "Momo".into();
        data.save(&path).unwrap();

        let loaded = Data::load(&path).unwrap();
        assert_eq!(loaded, data);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn corrupt_file_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, "{ not json").unwrap();
        assert!(matches!(Data::load(&path), Err(StoreError::Corrupt(_))));
    }

    #[test]
    fn next_id_repaired_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        let mut data = Data::default();
        data.add_reminder(draft("a", at(1, 0))).unwrap();
        data.add_reminder(draft("b", at(1, 0))).unwrap();
        data.next_id = 1;
        data.save(&path).unwrap();
        let mut loaded = Data::load(&path).unwrap();
        let id = loaded.add_note("c", at(1, 0)).unwrap();
        assert_eq!(id, 3);
    }

    #[test]
    fn validation() {
        let mut data = Data::default();
        assert!(data.add_reminder(draft("   ", at(1, 0))).is_err());
        let mut d = draft("x", at(1, 0));
        d.repeat = Repeat::EveryMinutes { minutes: 0 };
        assert!(data.add_reminder(d).is_err());
        let mut d = draft("x", at(1, 0));
        d.repeat = Repeat::Weekly { days: vec![4, 9, 1, 4] };
        let id = data.add_reminder(d).unwrap();
        assert_eq!(data.reminder_mut(id).unwrap().repeat, Repeat::Weekly { days: vec![1, 4] });
        assert!(data.add_note("  ", at(1, 0)).is_err());
    }

    #[test]
    fn due_list_is_oldest_first_and_edit_revives() {
        let mut data = Data::default();
        let late = data.add_reminder(draft("late", at(9, 30))).unwrap();
        let early = data.add_reminder(draft("early", at(9, 0))).unwrap();
        data.add_reminder(draft("future", at(12, 0))).unwrap();
        let ids: Vec<_> = data.due_reminders(at(10, 0)).iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![early, late]);

        data.complete_reminder(early, at(10, 0)).unwrap();
        assert_eq!(data.history.len(), 1);
        assert_eq!(data.history[0].waited_minutes, 60);
        assert_eq!(data.due_reminders(at(10, 0)).len(), 1);
        data.update_reminder(early, draft("early again", at(9, 0))).unwrap();
        assert_eq!(data.due_reminders(at(10, 0)).len(), 2);

        data.delete_reminder(late).unwrap();
        assert!(data.delete_reminder(late).is_err());
    }

    #[test]
    fn categories() {
        let mut data = Data::default();
        let work = data.add_category("Work", "#3366CC").unwrap();
        assert!(data.add_category("work", "#000000").is_err(), "duplicate name");
        assert!(data.add_category("Bad", "blue").is_err());
        let mut d = draft("Report", at(9, 0));
        d.category_id = Some(work);
        let r = data.add_reminder(d).unwrap();
        let n = data.add_note_in("ideas", at(9, 0), Some(work)).unwrap();
        let mut bad = draft("x", at(9, 0));
        bad.category_id = Some(999);
        assert!(data.add_reminder(bad).is_err());

        data.update_category(work, "Job", "#112233").unwrap();
        assert_eq!(data.categories[0].color, "#112233");
        assert_eq!(data.category_named("JOB", "#000000").unwrap(), work);

        data.delete_category(work).unwrap();
        assert_eq!(data.reminder_mut(r).unwrap().category_id, None);
        assert_eq!(data.notes.iter().find(|x| x.id == n).unwrap().category_id, None);
    }

    #[test]
    fn export_and_import() {
        let mut a = Data::default();
        let health = a.add_category("Health", "#22aa55").unwrap();
        let mut d = draft("Water", at(9, 0));
        d.category_id = Some(health);
        a.add_reminder(d).unwrap();
        a.add_note("hello", at(9, 0)).unwrap();
        let json = a.to_json().unwrap();

        // Merge into data that already has a "health" category and items.
        let mut b = Data::default();
        let existing = b.add_category("health", "#000000").unwrap();
        b.add_reminder(draft("Existing", at(8, 0))).unwrap();
        b.import(Data::from_json(&json).unwrap(), ImportMode::Merge);
        assert_eq!(b.categories.len(), 1);
        assert_eq!(b.reminders.len(), 2);
        assert_eq!(b.reminders[1].category_id, Some(existing));
        let mut ids: Vec<u64> = b.reminders.iter().map(|r| r.id).chain(b.notes.iter().map(|n| n.id)).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 3, "ids stay unique");

        b.import(Data::from_json(&json).unwrap(), ImportMode::Replace);
        assert_eq!(b, Data::from_json(&json).unwrap());

        assert!(Data::from_json("[]").is_err());
        assert!(Data::from_json("{}").is_err());
    }

    #[test]
    fn reminders_on_a_day_include_repeats() {
        let mut data = Data::default();
        let mut d = draft("Daily", at(8, 0));
        d.due = at(8, 0) - chrono::Duration::days(3);
        d.repeat = Repeat::Daily;
        data.add_reminder(d).unwrap();
        data.add_reminder(draft("Today", at(15, 0))).unwrap();
        data.add_reminder(draft("Tomorrow", at(15, 0) + chrono::Duration::days(1))).unwrap();
        let titles: Vec<&str> = data.reminders_on(at(0, 0).date()).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, vec!["Daily", "Today"]);
    }

    #[test]
    fn older_files_still_load() {
        let old = r#"{"version":1,"nextId":3,"reminders":[{"id":1,"title":"a","due":"2026-10-27T09:00:00","repeat":{"kind":"once"}}],"notes":[{"id":2,"text":"n","created":"2026-10-27T09:00:00"}],"settings":{}}"#;
        let data = Data::from_json(old).unwrap();
        assert_eq!(data.reminders[0].category_id, None);
        assert!(data.history.is_empty());
        assert_eq!(data.mood, Mood::default());
    }
}
