//! Screen-break nudges, the focus (pomodoro) timer, and mouse-shake detection.
//! All timing is in seconds of the app's own clock.

use serde::Serialize;
use std::collections::VecDeque;

// ---------------------------------------------------------------- breaks --

/// Counts continuous screen time and says when a break is due.
#[derive(Debug, Default)]
pub struct BreakTracker {
    /// Seconds of activity since the last real break.
    pub active_secs: f64,
    /// Do not nudge again before this much more activity.
    nudge_at: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakEvent {
    /// Time to suggest a break.
    Nudge,
    /// The user came back from a break of at least the "away" length.
    TookBreak,
}

impl BreakTracker {
    /// `idle_secs`: time since the last keyboard/mouse input.
    pub fn tick(&mut self, dt: f64, idle_secs: u64, every_min: u32, away_min: u32) -> Option<BreakEvent> {
        let away = idle_secs >= u64::from(away_min) * 60;
        if away {
            let was_working = self.active_secs >= 60.0;
            self.active_secs = 0.0;
            self.nudge_at = None;
            // Report once, on the tick the break starts counting.
            return was_working.then_some(BreakEvent::TookBreak);
        }
        if idle_secs < 60 {
            self.active_secs += dt;
        }
        let due = self.nudge_at.unwrap_or(f64::from(every_min) * 60.0);
        if self.active_secs >= due {
            // Ask again in 10 minutes if ignored.
            self.nudge_at = Some(self.active_secs + 10.0 * 60.0);
            return Some(BreakEvent::Nudge);
        }
        None
    }

    pub fn snooze(&mut self, minutes: u32) {
        self.nudge_at = Some(self.active_secs + f64::from(minutes) * 60.0);
    }

    pub fn reset(&mut self) {
        self.active_secs = 0.0;
        self.nudge_at = None;
    }
}

// ----------------------------------------------------------------- focus --

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum FocusPhase {
    Off,
    Focus { ends: f64, total: f64 },
    Break { ends: f64, total: f64, long: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    FocusDone,
    BreakDone,
}

#[derive(Debug)]
pub struct Focus {
    pub phase: FocusPhase,
    /// Focus sessions finished since the last long break.
    pub streak: u32,
}

impl Default for Focus {
    fn default() -> Self {
        Self { phase: FocusPhase::Off, streak: 0 }
    }
}

/// Timer state for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusView {
    /// "off", "focus" or "break".
    pub phase: &'static str,
    pub remaining_secs: u32,
    pub total_secs: u32,
    pub long_break: bool,
    pub streak: u32,
}

impl Focus {
    pub fn start_focus(&mut self, clock: f64, minutes: u32) {
        let total = f64::from(minutes) * 60.0;
        self.phase = FocusPhase::Focus { ends: clock + total, total };
    }

    /// Start the break that follows a session; long every `every` sessions.
    pub fn start_break(&mut self, clock: f64, short_min: u32, long_min: u32, every: u32) {
        let long = self.streak >= every.max(1);
        if long {
            self.streak = 0;
        }
        let total = f64::from(if long { long_min } else { short_min }) * 60.0;
        self.phase = FocusPhase::Break { ends: clock + total, total, long };
    }

    pub fn stop(&mut self) {
        self.phase = FocusPhase::Off;
    }

    pub fn is_focusing(&self) -> bool {
        matches!(self.phase, FocusPhase::Focus { .. })
    }

    pub fn tick(&mut self, clock: f64) -> Option<FocusEvent> {
        match self.phase {
            FocusPhase::Focus { ends, .. } if clock >= ends => {
                self.phase = FocusPhase::Off;
                self.streak += 1;
                Some(FocusEvent::FocusDone)
            }
            FocusPhase::Break { ends, .. } if clock >= ends => {
                self.phase = FocusPhase::Off;
                Some(FocusEvent::BreakDone)
            }
            _ => None,
        }
    }

    pub fn view(&self, clock: f64) -> FocusView {
        let (phase, ends, total, long) = match self.phase {
            FocusPhase::Off => ("off", clock, 0.0, false),
            FocusPhase::Focus { ends, total } => ("focus", ends, total, false),
            FocusPhase::Break { ends, total, long } => ("break", ends, total, long),
        };
        FocusView {
            phase,
            remaining_secs: (ends - clock).max(0.0).ceil() as u32,
            total_secs: total as u32,
            long_break: long,
            streak: self.streak,
        }
    }
}

// ----------------------------------------------------------------- shake --

/// Notices the mouse being shaken left and right quickly.
#[derive(Debug, Default)]
pub struct ShakeDetector {
    samples: VecDeque<(f64, f64)>,
    cooldown_until: f64,
}

const SHAKE_WINDOW: f64 = 1.0;
const SHAKE_TURNS: usize = 4;
const SHAKE_TRAVEL: f64 = 900.0;

impl ShakeDetector {
    /// Add a pointer x position (physical px) at `clock`. True on a shake.
    pub fn push(&mut self, clock: f64, x: f64, scale: f64) -> bool {
        self.samples.push_back((clock, x));
        while self.samples.front().is_some_and(|(t, _)| clock - t > SHAKE_WINDOW) {
            self.samples.pop_front();
        }
        if clock < self.cooldown_until {
            return false;
        }
        let mut turns = 0;
        let mut travel = 0.0;
        let mut last_dir = 0.0f64;
        let min_step = 4.0 * scale;
        for w in self.samples.iter().collect::<Vec<_>>().windows(2) {
            let dx = w[1].1 - w[0].1;
            if dx.abs() < min_step {
                continue;
            }
            travel += dx.abs();
            let dir = dx.signum();
            if last_dir != 0.0 && dir != last_dir {
                turns += 1;
            }
            last_dir = dir;
        }
        let shaken = turns >= SHAKE_TURNS && travel >= SHAKE_TRAVEL * scale;
        if shaken {
            self.cooldown_until = clock + 5.0;
            self.samples.clear();
        }
        shaken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_nudge_snooze_and_reset() {
        let mut b = BreakTracker::default();
        let mut events = vec![];
        // 50 minutes of activity in 1 s steps.
        for _ in 0..50 * 60 {
            events.extend(b.tick(1.0, 0, 50, 5));
        }
        assert_eq!(events, vec![BreakEvent::Nudge]);
        // Ignored: asks again 10 minutes later.
        events.clear();
        for _ in 0..10 * 60 {
            events.extend(b.tick(1.0, 0, 50, 5));
        }
        assert_eq!(events, vec![BreakEvent::Nudge]);
        // Stepping away for 5 minutes counts as the break.
        assert_eq!(b.tick(1.0, 5 * 60, 50, 5), Some(BreakEvent::TookBreak));
        assert_eq!(b.tick(1.0, 5 * 60 + 1, 50, 5), None);
        assert_eq!(b.active_secs, 0.0);
    }

    #[test]
    fn idle_but_present_time_does_not_count() {
        let mut b = BreakTracker::default();
        for _ in 0..100 * 60 {
            assert_eq!(b.tick(1.0, 120, 50, 5), None); // reading, not touching anything
        }
    }

    #[test]
    fn pomodoro_cycle() {
        let mut f = Focus::default();
        let mut clock = 0.0;
        for session in 1..=4 {
            f.start_focus(clock, 25);
            assert_eq!(f.view(clock).phase, "focus");
            clock += 25.0 * 60.0;
            assert_eq!(f.tick(clock), Some(FocusEvent::FocusDone));
            f.start_break(clock, 5, 15, 4);
            let v = f.view(clock);
            assert_eq!(v.long_break, session == 4, "session {session}");
            clock += f64::from(v.remaining_secs);
            assert_eq!(f.tick(clock), Some(FocusEvent::BreakDone));
        }
        assert_eq!(f.streak, 0);
        f.start_focus(clock, 25);
        f.stop();
        assert_eq!(f.tick(clock + 10_000.0), None);
    }

    #[test]
    fn shake_needs_fast_back_and_forth() {
        let mut s = ShakeDetector::default();
        // Slow drift: no.
        for i in 0..100 {
            assert!(!s.push(f64::from(i) * 0.05, f64::from(i) * 10.0, 1.0));
        }
        // Fast zig-zag: yes.
        let mut shaken = false;
        for i in 0..20 {
            let x = if i % 2 == 0 { 400.0 } else { 700.0 };
            shaken |= s.push(10.0 + f64::from(i) * 0.04, x, 1.0);
        }
        assert!(shaken);
        // Then a cooldown.
        for i in 0..20 {
            let x = if i % 2 == 0 { 400.0 } else { 700.0 };
            assert!(!s.push(11.0 + f64::from(i) * 0.04, x, 1.0));
        }
    }
}
