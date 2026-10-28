//! User-configurable settings, edited from the dashboard.

use serde::{Deserialize, Serialize};

/// Where the pet is allowed to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoamMode {
    /// Walk along the taskbar.
    #[default]
    BottomEdge,
    /// Wander anywhere on the screen, no gravity.
    WholeScreen,
    /// Sit and walk on top of the active window's title bar.
    ActiveWindow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Id of the pet pack to show.
    pub pet_id: String,
    /// What the pet calls itself in the speech bubble.
    pub pet_name: String,
    /// Sprite size multiplier.
    pub scale: f32,
    /// Walking speed in logical pixels per second.
    pub walk_speed: f32,
    pub emotions_enabled: bool,
    /// Let the pet say things on its own (notes, greetings).
    pub chatter_enabled: bool,
    pub chatter_interval_minutes: u32,
    /// How long a non-reminder bubble stays up.
    pub bubble_seconds: u32,
    pub snooze_minutes: u32,
    /// The pet naps after the user has been idle this long.
    pub idle_sleep_minutes: u32,
    /// No chatter between these hours (reminders still show).
    pub quiet_hours_enabled: bool,
    pub quiet_start_hour: u32,
    pub quiet_end_hour: u32,

    // ---- where it goes (#7) ----
    pub roam_mode: RoamMode,
    /// Monitor index to live on; `None` = whichever it is on now.
    pub monitor_index: Option<u32>,

    // ---- reactions (#8) ----
    /// Now and then walk over to the mouse pointer.
    pub follow_cursor: bool,
    /// React when the mouse is shaken.
    pub shake_reaction: bool,
    /// Greet by time of day and after you come back from a break.
    pub greetings_enabled: bool,

    // ---- care (#9) ----
    /// Hunger, feeding and play.
    pub care_enabled: bool,

    // ---- system usage (#6) ----
    pub system_monitor_enabled: bool,
    /// Say something when a limit is crossed (otherwise only animate).
    pub system_speech: bool,
    pub cpu_alert_percent: u32,
    pub ram_alert_percent: u32,
    pub battery_low_percent: u32,
    pub disk_low_percent: u32,

    // ---- quick add and hotkeys (#3, #15) ----
    /// Global shortcut for the quick-add box; empty = off.
    pub hotkey_quick_add: String,
    /// Global shortcut that saves the clipboard as a note; empty = off.
    pub hotkey_clipboard_note: String,

    // ---- notifications (#5) ----
    /// Also send a Windows notification when the pet can't be seen.
    pub notify_when_hidden: bool,

    // ---- breaks and focus (#11, #12) ----
    pub break_reminders_enabled: bool,
    /// Suggest a break after this much continuous screen time.
    pub break_every_minutes: u32,
    /// Being away this long counts as having taken a break.
    pub break_away_minutes: u32,
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    /// Focus sessions before a long break.
    pub sessions_before_long_break: u32,

    // ---- daily summary (#14) ----
    pub daily_summary_enabled: bool,
    pub daily_summary_hour: u32,
    pub daily_summary_minute: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pet_id: "panda".into(),
            pet_name: "Bamboo".into(),
            scale: 1.0,
            walk_speed: 40.0,
            emotions_enabled: true,
            chatter_enabled: true,
            chatter_interval_minutes: 30,
            bubble_seconds: 8,
            snooze_minutes: 10,
            idle_sleep_minutes: 5,
            quiet_hours_enabled: false,
            quiet_start_hour: 22,
            quiet_end_hour: 8,
            roam_mode: RoamMode::BottomEdge,
            monitor_index: None,
            follow_cursor: true,
            shake_reaction: true,
            greetings_enabled: true,
            care_enabled: true,
            system_monitor_enabled: true,
            system_speech: true,
            cpu_alert_percent: 90,
            ram_alert_percent: 90,
            battery_low_percent: 20,
            disk_low_percent: 10,
            hotkey_quick_add: "Ctrl+Alt+R".into(),
            hotkey_clipboard_note: "Ctrl+Alt+N".into(),
            notify_when_hidden: true,
            break_reminders_enabled: true,
            break_every_minutes: 50,
            break_away_minutes: 5,
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            sessions_before_long_break: 4,
            daily_summary_enabled: true,
            daily_summary_hour: 9,
            daily_summary_minute: 0,
        }
    }
}

impl Settings {
    /// Pull every value into a range the app can work with.
    pub fn sanitized(mut self) -> Self {
        self.scale = if self.scale.is_finite() { self.scale.clamp(0.5, 3.0) } else { 1.0 };
        self.walk_speed = if self.walk_speed.is_finite() { self.walk_speed.clamp(0.0, 400.0) } else { 40.0 };
        self.chatter_interval_minutes = self.chatter_interval_minutes.clamp(1, 24 * 60);
        self.bubble_seconds = self.bubble_seconds.clamp(2, 120);
        self.snooze_minutes = self.snooze_minutes.clamp(1, 24 * 60);
        self.idle_sleep_minutes = self.idle_sleep_minutes.clamp(1, 24 * 60);
        self.quiet_start_hour %= 24;
        self.quiet_end_hour %= 24;
        self.cpu_alert_percent = self.cpu_alert_percent.clamp(10, 100);
        self.ram_alert_percent = self.ram_alert_percent.clamp(10, 100);
        self.battery_low_percent = self.battery_low_percent.clamp(1, 90);
        self.disk_low_percent = self.disk_low_percent.clamp(1, 90);
        self.break_every_minutes = self.break_every_minutes.clamp(5, 8 * 60);
        self.break_away_minutes = self.break_away_minutes.clamp(1, 120);
        self.focus_minutes = self.focus_minutes.clamp(1, 240);
        self.short_break_minutes = self.short_break_minutes.clamp(1, 120);
        self.long_break_minutes = self.long_break_minutes.clamp(1, 240);
        self.sessions_before_long_break = self.sessions_before_long_break.clamp(1, 12);
        self.daily_summary_hour %= 24;
        self.daily_summary_minute %= 60;
        self.monitor_index = self.monitor_index.filter(|i| *i < 16);
        self.hotkey_quick_add = self.hotkey_quick_add.trim().to_owned();
        self.hotkey_clipboard_note = self.hotkey_clipboard_note.trim().to_owned();
        self.pet_name = self.pet_name.trim().chars().take(32).collect();
        if self.pet_name.is_empty() {
            self.pet_name = Settings::default().pet_name;
        }
        self
    }

    /// Whether `hour` falls in quiet hours. The range may wrap past midnight.
    pub fn is_quiet(&self, hour: u32) -> bool {
        if !self.quiet_hours_enabled || self.quiet_start_hour == self.quiet_end_hour {
            return false;
        }
        let (s, e) = (self.quiet_start_hour, self.quiet_end_hour);
        if s < e {
            (s..e).contains(&hour)
        } else {
            hour >= s || hour < e
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_wrap_midnight() {
        let s = Settings { quiet_hours_enabled: true, quiet_start_hour: 22, quiet_end_hour: 8, ..Default::default() };
        assert!(s.is_quiet(23));
        assert!(s.is_quiet(3));
        assert!(!s.is_quiet(8));
        assert!(!s.is_quiet(12));
    }

    #[test]
    fn sanitize_clamps() {
        let s = Settings { scale: 100.0, bubble_seconds: 0, pet_name: "  ".into(), ..Default::default() }.sanitized();
        assert_eq!(s.scale, 3.0);
        assert_eq!(s.bubble_seconds, 2);
        assert_eq!(s.pet_name, "Bamboo");
    }

    #[test]
    fn missing_fields_use_defaults() {
        let s: Settings = serde_json::from_str(r#"{"petName":"Momo"}"#).unwrap();
        assert_eq!(s.pet_name, "Momo");
        assert_eq!(s.pet_id, "panda");
    }
}
