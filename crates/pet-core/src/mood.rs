//! The pet's emotional state.
//!
//! Four stats in 0..=100 drift over time and jump on events. The dominant
//! [`Feeling`] picked from them decides which animations the pet prefers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Feeling {
    Happy,
    Neutral,
    Bored,
    Sad,
    Sleepy,
    Hungry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoodEvent {
    Poked,
    Dragged,
    ReminderDone,
    ReminderSnoozed,
    Fed,
    Played,
}

/// What the outside world looks like this tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct MoodContext {
    /// Local hour, 0..=23.
    pub hour: u32,
    pub sleeping: bool,
    /// How long the oldest unanswered reminder has been waiting.
    pub ignored_reminder_secs: u64,
    /// Care mode: the pet gets hungry and wants feeding.
    pub care: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Mood {
    pub happiness: f32,
    pub energy: f32,
    pub boredom: f32,
    /// 100 = just ate. Only drops in care mode.
    pub fullness: f32,
}

impl Default for Mood {
    fn default() -> Self {
        Self { happiness: 65.0, energy: 80.0, boredom: 20.0, fullness: 80.0 }
    }
}

/// Below this the pet is hungry (care mode only).
pub const HUNGRY_BELOW: f32 = 20.0;

/// Reminders ignored for longer than this start to make the pet sad.
const IGNORED_GRACE_SECS: u64 = 5 * 60;

impl Mood {
    /// Advance the stats by `dt` seconds.
    pub fn tick(&mut self, dt: f32, ctx: MoodContext) {
        let per_min = dt / 60.0;
        let night = ctx.hour >= 23 || ctx.hour < 6;

        if ctx.sleeping {
            self.energy += 6.0 * per_min;
            self.boredom -= 2.0 * per_min;
        } else {
            self.energy -= if night { 1.5 } else { 0.5 } * per_min;
            self.boredom += 1.0 * per_min;
        }

        // Happiness slowly returns to a content baseline...
        let drift = 0.5 * per_min;
        self.happiness += (60.0 - self.happiness).clamp(-drift, drift);
        // ...unless the pet is being ignored.
        if ctx.ignored_reminder_secs > IGNORED_GRACE_SECS {
            self.happiness -= 2.0 * per_min;
        }
        if ctx.care {
            // Empty in roughly 6-7 waking hours.
            self.fullness -= if ctx.sleeping { 0.1 } else { 0.25 } * per_min;
            if self.fullness < HUNGRY_BELOW {
                self.happiness -= 1.0 * per_min;
            }
        }
        self.clamp();
    }

    pub fn apply(&mut self, event: MoodEvent) {
        match event {
            MoodEvent::Poked => {
                self.happiness += 8.0;
                self.boredom -= 25.0;
                self.energy -= 1.0;
            }
            MoodEvent::Dragged => self.boredom -= 10.0,
            MoodEvent::ReminderDone => self.happiness += 10.0,
            MoodEvent::ReminderSnoozed => self.happiness -= 2.0,
            MoodEvent::Fed => {
                self.fullness += 35.0;
                self.happiness += 5.0;
            }
            MoodEvent::Played => {
                self.boredom -= 40.0;
                self.happiness += 10.0;
                self.energy -= 5.0;
            }
        }
        self.clamp();
    }

    /// The dominant feeling. Hunger only counts in care mode.
    pub fn feeling(&self, care: bool) -> Feeling {
        if self.energy < 15.0 {
            Feeling::Sleepy
        } else if care && self.fullness < HUNGRY_BELOW {
            Feeling::Hungry
        } else if self.happiness < 30.0 {
            Feeling::Sad
        } else if self.boredom > 70.0 {
            Feeling::Bored
        } else if self.happiness > 75.0 {
            Feeling::Happy
        } else {
            Feeling::Neutral
        }
    }

    fn clamp(&mut self) {
        for v in [&mut self.happiness, &mut self.energy, &mut self.boredom, &mut self.fullness] {
            *v = v.clamp(0.0, 100.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn awake(hour: u32) -> MoodContext {
        MoodContext { hour, ..Default::default() }
    }

    #[test]
    fn energy_drains_faster_at_night() {
        let (mut day, mut night) = (Mood::default(), Mood::default());
        day.tick(600.0, awake(14));
        night.tick(600.0, awake(1));
        assert!(night.energy < day.energy);
        assert!(day.energy < Mood::default().energy);
    }

    #[test]
    fn sleeping_restores_energy() {
        let mut m = Mood { energy: 10.0, ..Default::default() };
        assert_eq!(m.feeling(false), Feeling::Sleepy);
        m.tick(600.0, MoodContext { sleeping: true, ..awake(14) });
        assert!(m.energy >= 60.0);
    }

    #[test]
    fn ignored_reminders_make_it_sad() {
        let mut m = Mood { happiness: 40.0, ..Default::default() };
        let ctx = MoodContext { ignored_reminder_secs: 3600, ..awake(12) };
        for _ in 0..10 {
            m.tick(60.0, ctx);
        }
        assert_eq!(m.feeling(false), Feeling::Sad);
    }

    #[test]
    fn poking_cheers_up_and_cures_boredom() {
        let mut m = Mood { boredom: 90.0, ..Default::default() };
        assert_eq!(m.feeling(false), Feeling::Bored);
        m.apply(MoodEvent::Poked);
        m.apply(MoodEvent::Poked);
        assert!(m.boredom <= 40.0);
        assert!(m.happiness > Mood::default().happiness);
    }

    #[test]
    fn care_mode_gets_hungry_and_feeding_helps() {
        let mut m = Mood { fullness: 25.0, ..Default::default() };
        let ctx = MoodContext { care: true, ..awake(12) };
        for _ in 0..30 {
            m.tick(60.0, ctx);
        }
        assert_eq!(m.feeling(true), Feeling::Hungry);
        assert_ne!(m.feeling(false), Feeling::Hungry);
        m.apply(MoodEvent::Fed);
        assert_ne!(m.feeling(true), Feeling::Hungry);

        // Without care mode fullness never drops.
        let mut m = Mood::default();
        m.tick(6000.0, awake(12));
        assert_eq!(m.fullness, Mood::default().fullness);
    }

    #[test]
    fn old_saved_mood_without_fullness_loads() {
        let m: Mood = serde_json::from_str(r#"{"happiness":50,"energy":40,"boredom":10}"#).unwrap();
        assert_eq!(m.fullness, Mood::default().fullness);
    }

    #[test]
    fn stats_stay_in_range() {
        let mut m = Mood::default();
        for _ in 0..50 {
            m.apply(MoodEvent::Poked);
        }
        assert_eq!(m.happiness, 100.0);
        assert_eq!(m.boredom, 0.0);
    }
}
