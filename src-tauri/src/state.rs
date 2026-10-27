//! App state and the per-frame update.
//!
//! `AppState::tick` is pure bookkeeping: it reads the environment it is given
//! and returns what should change on screen. The engine thread applies that
//! output to the real window *after* releasing the lock, so window calls
//! never happen while the state is locked.

use crate::platform::Rect;
use chrono::{NaiveDateTime, Timelike};
use pet_core::behavior::{Anim, Bounds, Facing, Inputs, Pet};
use pet_core::mood::{Feeling, Mood, MoodContext, MoodEvent};
use pet_core::pack::{self, LoadedPack, Pack};
use pet_core::store::{Data, StoreError};
use serde::Serialize;
use std::path::PathBuf;

/// Room above the sprite for the speech bubble, in CSS pixels.
pub const BUBBLE_W: f64 = 240.0;
pub const BUBBLE_H: f64 = 130.0;
/// Pointer movement (logical px) before a press becomes a drag.
const DRAG_THRESHOLD: f64 = 4.0;

pub struct Env {
    pub now: NaiveDateTime,
    pub cursor: Option<(f64, f64)>,
    pub left_down: Option<bool>,
    pub idle_secs: u64,
    pub work_area: Rect,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameView {
    pub anim: Anim,
    pub facing: Facing,
    /// Sprite placement inside the pet window, CSS pixels.
    pub sprite_left: f64,
    pub sprite_width: f64,
    pub sprite_height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BubbleKind {
    Reminder,
    Say,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BubbleView {
    pub kind: BubbleKind,
    pub reminder_id: Option<u64>,
    pub title: String,
    pub text: String,
}

struct Bubble {
    view: BubbleView,
    /// Clock time to hide at; `None` stays until answered.
    expires: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlan {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub ignore_cursor: bool,
}

#[derive(Default)]
pub struct TickOutput {
    pub window: Option<WindowPlan>,
    pub frame: Option<FrameView>,
    pub bubble: Option<Option<BubbleView>>,
    /// A reminder just came due: make sure the pet is visible.
    pub show_pet: bool,
}

struct Pointer {
    grab: (f64, f64),
    start: (f64, f64),
    dragging: bool,
}

pub struct AppState {
    pub data: Data,
    pub data_path: PathBuf,
    pub pack_roots: Vec<PathBuf>,
    pub packs: Vec<Pack>,
    pub pack_errors: Vec<String>,
    pub pet: Pet,
    pub mood: Mood,
    rng: fastrand::Rng,
    /// Seconds since start, advanced by `tick`.
    clock: f64,
    next_slow_tick: f64,
    next_chatter: f64,
    bubble: Option<Bubble>,
    bubble_dirty: bool,
    pointer: Option<Pointer>,
    alerting: bool,
    scale_factor: f64,
    pub last_frame: Option<FrameView>,
    last_window: Option<WindowPlan>,
}

impl AppState {
    pub fn new(data: Data, data_path: PathBuf, pack_roots: Vec<PathBuf>) -> Self {
        let interval = f64::from(data.settings.chatter_interval_minutes) * 60.0;
        let mut s = Self {
            data,
            data_path,
            pack_roots,
            packs: vec![],
            pack_errors: vec![],
            pet: Pet::new(200.0, 0.0),
            mood: Mood::default(),
            rng: fastrand::Rng::new(),
            clock: 0.0,
            next_slow_tick: 0.0,
            next_chatter: interval,
            bubble: None,
            bubble_dirty: false,
            pointer: None,
            alerting: false,
            scale_factor: 1.0,
            last_frame: None,
            last_window: None,
        };
        s.reload_packs();
        let greeting = format!("Hi! I'm {}.", s.data.settings.pet_name);
        s.say(greeting);
        s
    }

    pub fn save(&self) -> Result<(), String> {
        self.data.save(&self.data_path).map_err(|e: StoreError| e.to_string())
    }

    pub fn reload_packs(&mut self) {
        let (packs, errors) = pack::discover(&self.pack_roots);
        self.packs = packs;
        self.pack_errors = errors.iter().map(ToString::to_string).collect();
        self.invalidate_view();
    }

    /// The selected pack, or the first one found if it is missing.
    pub fn current_pack(&self) -> Option<&Pack> {
        let id = &self.data.settings.pet_id;
        self.packs.iter().find(|p| &p.manifest.id == id).or(self.packs.first())
    }

    pub fn loaded_pack(&self) -> Result<LoadedPack, String> {
        let pack = self
            .current_pack()
            .ok_or_else(|| format!("No pet packs found. Looked in: {}", join_paths(&self.pack_roots)))?;
        pack.load_frames().map_err(|e| e.to_string())
    }

    /// Force the next tick to resend the window plan and frame.
    pub fn invalidate_view(&mut self) {
        self.last_frame = None;
        self.last_window = None;
    }

    pub fn bubble_view(&self) -> Option<BubbleView> {
        self.bubble.as_ref().map(|b| b.view.clone())
    }

    pub fn feeling(&self) -> Feeling {
        if self.data.settings.emotions_enabled {
            self.mood.feeling()
        } else {
            Feeling::Neutral
        }
    }

    /// Show a passing message. Never replaces a reminder.
    pub fn say(&mut self, text: impl Into<String>) {
        if matches!(&self.bubble, Some(b) if b.view.kind == BubbleKind::Reminder) {
            return;
        }
        let secs = f64::from(self.data.settings.bubble_seconds);
        self.set_bubble(Some(Bubble {
            view: BubbleView { kind: BubbleKind::Say, reminder_id: None, title: String::new(), text: text.into() },
            expires: Some(self.clock + secs),
        }));
    }

    fn set_bubble(&mut self, bubble: Option<Bubble>) {
        self.bubble = bubble;
        self.bubble_dirty = true;
    }

    /// Physical sprite size for the current pack, settings and monitor.
    fn sprite_px(&self) -> (f64, f64) {
        let (w, h) = self.current_pack().map_or((128, 128), |p| (p.manifest.width, p.manifest.height));
        let k = f64::from(self.data.settings.scale) * self.scale_factor;
        ((f64::from(w) * k).round(), (f64::from(h) * k).round())
    }

    pub fn pointer_down(&mut self, cursor: Option<(f64, f64)>) {
        let c = cursor.unwrap_or_default();
        self.pointer = Some(Pointer { grab: (c.0 - self.pet.x, c.1 - self.pet.y), start: c, dragging: false });
    }

    /// Release after a press: a drop if it moved, otherwise a poke.
    pub fn pointer_up(&mut self) {
        let Some(p) = self.pointer.take() else { return };
        if p.dragging {
            self.pet.end_drag();
        } else {
            self.pet.poke();
            self.mood.apply(MoodEvent::Poked);
            if matches!(&self.bubble, Some(b) if b.view.kind == BubbleKind::Say) {
                self.set_bubble(None);
            }
        }
    }

    pub fn reminder_done(&mut self, id: u64, now: NaiveDateTime) -> Result<(), String> {
        self.data.reminder_mut(id).map_err(|e| e.to_string())?.complete(now);
        self.mood.apply(MoodEvent::ReminderDone);
        self.pet.react(Anim::Happy, 1.5);
        self.clear_reminder_bubble(id);
        self.save()
    }

    pub fn reminder_snooze(&mut self, id: u64, now: NaiveDateTime, minutes: Option<u32>) -> Result<(), String> {
        let minutes = minutes.unwrap_or(self.data.settings.snooze_minutes);
        self.data.reminder_mut(id).map_err(|e| e.to_string())?.snooze(now, minutes);
        self.mood.apply(MoodEvent::ReminderSnoozed);
        self.clear_reminder_bubble(id);
        self.save()
    }

    /// Drop the bubble for a reminder that was answered, edited or deleted.
    /// The next slow tick shows the next due reminder, if any.
    pub fn clear_reminder_bubble(&mut self, id: u64) {
        if matches!(&self.bubble, Some(b) if b.view.reminder_id == Some(id)) {
            self.set_bubble(None);
            self.next_slow_tick = self.clock;
        }
    }

    pub fn tick(&mut self, dt: f64, env: &Env) -> TickOutput {
        let mut out = TickOutput::default();
        self.clock += dt;
        self.scale_factor = env.scale_factor;

        if self.clock >= self.next_slow_tick {
            self.next_slow_tick = self.clock + 1.0;
            self.slow_tick(env, &mut out);
        }

        if let Some(p) = &mut self.pointer {
            if let Some(c) = env.cursor {
                let moved = (c.0 - p.start.0).hypot(c.1 - p.start.1);
                if !p.dragging && moved > DRAG_THRESHOLD * env.scale_factor {
                    p.dragging = true;
                    self.pet.start_drag();
                    self.mood.apply(MoodEvent::Dragged);
                }
                if p.dragging {
                    self.pet.drag_to(c.0 - p.grab.0, c.1 - p.grab.1);
                }
            }
            // Backup for a pointerup the webview never saw (fast drags).
            if env.left_down == Some(false) {
                self.pointer_up();
            }
        }

        let (sw, sh) = self.sprite_px();
        let wa = env.work_area;
        let s = &self.data.settings;
        let inputs = Inputs {
            dt,
            bounds: Bounds { left: wa.left, right: wa.right - sw, floor: wa.bottom - sh },
            feeling: self.feeling(),
            user_away: env.idle_secs >= u64::from(s.idle_sleep_minutes) * 60,
            alert: self.alerting,
            speed: f64::from(s.walk_speed) * env.scale_factor,
        };
        self.pet.tick(inputs, &mut self.rng);

        if let Some(b) = &self.bubble {
            if b.expires.is_some_and(|t| self.clock >= t) {
                self.set_bubble(None);
            }
        }

        // Window geometry: sprite at the bottom, bubble room above it.
        let sf = env.scale_factor;
        let (bw, bh) = ((BUBBLE_W * sf).round(), (BUBBLE_H * sf).round());
        let win_w = sw.max(bw);
        let win_h = sh + bh;
        let mut win_x = self.pet.x + sw / 2.0 - win_w / 2.0;
        if !self.pet.is_dragged() {
            win_x = win_x.clamp(wa.left, (wa.right - win_w).max(wa.left));
        }
        let win_y = self.pet.y - bh;

        let sprite = Rect { left: self.pet.x, top: self.pet.y, right: self.pet.x + sw, bottom: self.pet.y + sh };
        let bubble_area = Rect { left: win_x, top: win_y, right: win_x + win_w, bottom: self.pet.y };
        let over_pet =
            |(x, y): (f64, f64)| sprite.contains(x, y) || (self.bubble.is_some() && bubble_area.contains(x, y));
        // Without a cursor position we cannot hit-test, so keep the window clickable.
        let ignore_cursor = self.pointer.is_none() && env.cursor.is_some_and(|c| !over_pet(c));

        let plan = WindowPlan {
            x: win_x.round() as i32,
            y: win_y.round() as i32,
            width: win_w as u32,
            height: win_h as u32,
            ignore_cursor,
        };
        if self.last_window != Some(plan) {
            self.last_window = Some(plan);
            out.window = Some(plan);
        }

        let frame = FrameView {
            anim: self.pet.anim(),
            facing: self.pet.facing,
            sprite_left: ((self.pet.x - win_x) / sf).round(),
            sprite_width: sw / sf,
            sprite_height: sh / sf,
        };
        if self.last_frame.as_ref() != Some(&frame) {
            self.last_frame = Some(frame.clone());
            out.frame = Some(frame);
        }

        if std::mem::take(&mut self.bubble_dirty) {
            out.bubble = Some(self.bubble_view());
        }
        out
    }

    /// Once a second: reminders, mood and chatter.
    fn slow_tick(&mut self, env: &Env, out: &mut TickOutput) {
        let now = env.now;
        let first = self
            .data
            .due_reminders(now)
            .first()
            .map(|r| (r.id, r.title.clone(), r.details.clone(), (now - r.fire_at()).num_seconds().max(0) as u64));

        let was_alerting = self.alerting;
        self.alerting = first.is_some();
        let mut ignored_secs = 0;
        match first {
            Some((id, title, details, waited)) => {
                ignored_secs = waited;
                let showing = matches!(&self.bubble, Some(b) if b.view.reminder_id == Some(id));
                if !showing {
                    self.set_bubble(Some(Bubble {
                        view: BubbleView { kind: BubbleKind::Reminder, reminder_id: Some(id), title, text: details },
                        expires: None,
                    }));
                }
                out.show_pet |= !was_alerting;
            }
            None => {
                if matches!(&self.bubble, Some(b) if b.view.kind == BubbleKind::Reminder) {
                    self.set_bubble(None);
                }
            }
        }

        if self.data.settings.emotions_enabled {
            let ctx =
                MoodContext { hour: now.hour(), sleeping: self.pet.is_sleeping(), ignored_reminder_secs: ignored_secs };
            self.mood.tick(1.0, ctx);
        }

        if self.clock >= self.next_chatter {
            let interval = f64::from(self.data.settings.chatter_interval_minutes) * 60.0;
            self.next_chatter = self.clock + interval * (0.75 + self.rng.f64() * 0.5);
            let s = &self.data.settings;
            if s.chatter_enabled
                && !s.is_quiet(now.hour())
                && self.bubble.is_none()
                && !self.alerting
                && !self.pet.is_sleeping()
            {
                let line = self.random_line();
                self.say(line);
            }
        }
    }

    fn random_line(&mut self) -> String {
        let notes = &self.data.notes;
        if !notes.is_empty() && self.rng.u8(0..10) < 6 {
            return format!("Don't forget: {}", notes[self.rng.usize(..notes.len())].text);
        }
        let lines: &[&str] = match self.feeling() {
            Feeling::Happy => &["Today is a good day!", "*happy wiggle*", "You're doing great!"],
            Feeling::Sad => &["I feel a bit lonely...", "Could you poke me?"],
            Feeling::Bored => &["I'm bored... poke me?", "*yawns*", "Anything fun to do?"],
            Feeling::Neutral | Feeling::Sleepy => {
                &["*stretches*", "I'm right here if you need me.", "Right-click me to open the dashboard."]
            }
        };
        lines[self.rng.usize(..lines.len())].to_owned()
    }
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use pet_core::reminder::Repeat;
    use pet_core::store::ReminderDraft;

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 27).unwrap().and_hms_opt(h, m, 0).unwrap()
    }

    fn env(now: NaiveDateTime) -> Env {
        Env {
            now,
            cursor: None,
            left_down: None,
            idle_secs: 0,
            work_area: Rect { left: 0.0, top: 0.0, right: 1920.0, bottom: 1040.0 },
            scale_factor: 1.0,
        }
    }

    fn state() -> AppState {
        let packs = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../pets"));
        let dir = std::env::temp_dir().join(format!("desktoppet-test-{}", fastrand::u64(..)));
        AppState::new(Data::default(), dir.join("data.json"), vec![packs])
    }

    #[test]
    fn built_in_packs_load() {
        let s = state();
        assert!(s.pack_errors.is_empty(), "{:?}", s.pack_errors);
        let ids: Vec<_> = s.packs.iter().map(|p| p.manifest.id.as_str()).collect();
        assert!(ids.contains(&"panda") && ids.contains(&"blue-cat"), "{ids:?}");
        assert_eq!(s.loaded_pack().unwrap().id, "panda");
    }

    #[test]
    fn greets_then_shows_due_reminder_until_done() {
        let mut s = state();
        let out = s.tick(0.03, &env(at(9, 0)));
        assert_eq!(out.bubble.unwrap().unwrap().kind, BubbleKind::Say);

        let draft = ReminderDraft {
            title: "Call mom".into(),
            details: "".into(),
            due: at(9, 5),
            repeat: Repeat::Once,
            enabled: true,
        };
        let id = s.data.add_reminder(draft).unwrap();
        for _ in 0..40 {
            s.tick(0.03, &env(at(9, 4)));
        }
        assert!(!s.alerting);

        let mut shown = None;
        for _ in 0..40 {
            let out = s.tick(0.03, &env(at(9, 6)));
            if let Some(b) = out.bubble {
                shown = b;
            }
            if out.show_pet {
                assert!(s.alerting);
            }
        }
        let b = shown.expect("reminder bubble");
        assert_eq!((b.kind, b.reminder_id, b.title.as_str()), (BubbleKind::Reminder, Some(id), "Call mom"));
        assert_eq!(s.pet.anim(), Anim::Alert);

        s.reminder_done(id, at(9, 6)).unwrap();
        assert!(s.bubble_view().is_none());
        for _ in 0..40 {
            s.tick(0.03, &env(at(9, 7)));
        }
        assert!(!s.alerting);
        assert!(s.bubble_view().is_none());
    }

    #[test]
    fn window_sits_on_taskbar_and_click_through_outside_pet() {
        let mut s = state();
        let mut e = env(at(12, 0));
        e.cursor = Some((5.0, 5.0));
        let mut plan = None;
        // The pet drops in from the top of the screen first.
        for _ in 0..100 {
            if let Some(p) = s.tick(0.03, &e).window {
                plan = Some(p);
            }
        }
        let plan = plan.unwrap();
        assert_eq!(plan.y + plan.height as i32, 1040);
        assert!(plan.ignore_cursor);

        // Hovering the sprite makes the window clickable again.
        e.cursor = Some((s.pet.x + 10.0, s.pet.y + 10.0));
        let plan = s.tick(0.03, &e).window.unwrap();
        assert!(!plan.ignore_cursor);
    }

    #[test]
    fn press_without_move_pokes_and_with_move_drags() {
        let mut s = state();
        let mut e = env(at(12, 0));
        for _ in 0..100 {
            s.tick(0.03, &e);
        }
        let start = (s.pet.x + 5.0, s.pet.y + 5.0);
        s.pointer_down(Some(start));
        s.pointer_up();
        assert_eq!(s.pet.anim(), Anim::Happy);

        s.pointer_down(Some(start));
        e.cursor = Some((start.0 + 300.0, start.1 - 400.0));
        e.left_down = Some(true);
        s.tick(0.03, &e);
        assert!(s.pet.is_dragged());
        e.left_down = Some(false);
        s.tick(0.03, &e);
        assert!(!s.pet.is_dragged());
        for _ in 0..60 {
            s.tick(0.03, &e);
        }
        assert_eq!(s.pet.y, 1040.0 - 128.0);
    }
}
