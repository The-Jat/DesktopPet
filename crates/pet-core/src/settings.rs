//! User-configurable settings, edited from the dashboard.

use serde::{Deserialize, Serialize};

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
