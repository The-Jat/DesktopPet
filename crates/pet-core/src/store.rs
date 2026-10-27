//! Everything the app saves to disk, kept in one JSON file.

use crate::reminder::{Reminder, Repeat};
use crate::settings::Settings;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: u64,
    pub text: String,
    pub created: NaiveDateTime,
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
    pub settings: Settings,
}

impl Default for Data {
    fn default() -> Self {
        Self { version: 1, next_id: 1, reminders: vec![], notes: vec![], settings: Settings::default() }
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
            Ok(text) => {
                let mut data: Data = serde_json::from_str(&text).map_err(StoreError::Corrupt)?;
                data.settings = data.settings.sanitized();
                let max_id = data.reminders.iter().map(|r| r.id).chain(data.notes.iter().map(|n| n.id)).max();
                data.next_id = data.next_id.max(max_id.map_or(1, |m| m + 1));
                Ok(data)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Data::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Write to a temporary file first, then rename, so a crash mid-write
    /// never leaves a half-written data file behind.
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(StoreError::Corrupt)?;
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
        let draft = validate(draft)?;
        let id = self.take_id();
        let mut r = Reminder::new(id, draft.title, draft.due, draft.repeat);
        r.details = draft.details;
        r.enabled = draft.enabled;
        self.reminders.push(r);
        Ok(id)
    }

    /// Replace a reminder's fields. Clears any snooze or completion so the
    /// edited reminder is live again.
    pub fn update_reminder(&mut self, id: u64, draft: ReminderDraft) -> Result<(), StoreError> {
        let draft = validate(draft)?;
        let r = self.reminder_mut(id)?;
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

    pub fn add_note(&mut self, text: &str, now: NaiveDateTime) -> Result<u64, StoreError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(StoreError::Invalid("note is empty"));
        }
        let id = self.take_id();
        self.notes.push(Note { id, text: text.to_owned(), created: now });
        Ok(id)
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

fn validate(mut draft: ReminderDraft) -> Result<ReminderDraft, StoreError> {
    draft.title = draft.title.trim().to_owned();
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
        ReminderDraft { title: title.into(), details: String::new(), due, repeat: Repeat::Once, enabled: true }
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

        data.reminder_mut(early).unwrap().complete(at(10, 0));
        assert_eq!(data.due_reminders(at(10, 0)).len(), 1);
        data.update_reminder(early, draft("early again", at(9, 0))).unwrap();
        assert_eq!(data.due_reminders(at(10, 0)).len(), 2);

        data.delete_reminder(late).unwrap();
        assert!(data.delete_reminder(late).is_err());
    }
}
