//! App state and the per-frame update.
//!
//! `AppState::tick` is pure bookkeeping: it reads the environment it is given
//! and returns what should change on screen. The engine thread applies that
//! output to the real window *after* releasing the lock, so window calls
//! never happen while the state is locked.

use crate::platform::{Foreground, Rect};
use chrono::{NaiveDateTime, Timelike};
use pet_core::behavior::{Anim, Bounds, Facing, Inputs, Pet, Surface, WindowRect};
use pet_core::history::{HistoryEntry, HistoryKind};
use pet_core::mood::{Feeling, MoodContext, MoodEvent};
use pet_core::pack::{self, LoadedPack, Pack};
use pet_core::productivity::{BreakEvent, BreakTracker, Focus, FocusEvent, ShakeDetector};
use pet_core::settings::RoamMode;
use pet_core::store::{Data, StoreError};
use pet_core::system::{self, Sample, Watch};
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;

/// Room above the sprite for the speech bubble, in CSS pixels.
pub const BUBBLE_W: f64 = 240.0;
pub const BUBBLE_H: f64 = 150.0;
/// Pointer movement (logical px) before a press becomes a drag.
const DRAG_THRESHOLD: f64 = 4.0;
/// How often the mood is written to disk.
const MOOD_SAVE_SECS: f64 = 60.0;

pub struct Env {
    pub now: NaiveDateTime,
    pub cursor: Option<(f64, f64)>,
    pub left_down: Option<bool>,
    pub idle_secs: u64,
    /// Work area of the monitor the pet should be on.
    pub work_area: Rect,
    pub scale_factor: f64,
    pub foreground: Foreground,
    pub pet_visible: bool,
    pub fullscreen: bool,
    /// A fresh system reading, taken `system_dt` seconds after the last one.
    pub system: Option<Sample>,
    pub system_dt: f32,
    /// Recently used windows the pet may climb (empty when climbing is off).
    pub windows: Vec<WindowRect>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameView {
    pub anim: Anim,
    pub facing: Facing,
    /// Pet window size and sprite placement inside it, CSS pixels.
    pub window_width: f64,
    pub window_height: f64,
    pub sprite_left: f64,
    pub sprite_top: f64,
    pub sprite_width: f64,
    pub sprite_height: f64,
    /// A ladder standing against a window, inside the pet window.
    pub ladder: Option<LadderView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LadderView {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BubbleKind {
    Reminder,
    /// A suggestion with buttons: breaks, focus timer, daily summary.
    Nudge,
    Say,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BubbleAction {
    pub id: String,
    pub label: String,
}

fn action(id: impl Into<String>, label: &str) -> BubbleAction {
    BubbleAction { id: id.into(), label: label.to_owned() }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BubbleView {
    pub kind: BubbleKind,
    pub reminder_id: Option<u64>,
    pub title: String,
    pub text: String,
    pub actions: Vec<BubbleAction>,
}

#[derive(Debug, Clone)]
struct Bubble {
    view: BubbleView,
    /// Seconds to stay up once shown; `None` = until answered.
    ttl: Option<f64>,
    /// Clock time to hide at, set when shown.
    expires: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlan {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct TickOutput {
    pub window: Option<WindowPlan>,
    pub frame: Option<FrameView>,
    pub bubble: Option<Option<BubbleView>>,
    /// A reminder just came due: make sure the pet is visible.
    pub show_pet: bool,
    /// Windows notifications to send: (title, body).
    pub notify: Vec<(String, String)>,
    /// Stored data changed (history, summary date): refresh the dashboard.
    pub data_changed: bool,
    /// New tray tooltip.
    pub tooltip: Option<String>,
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
    rng: fastrand::Rng,
    /// Seconds since start, advanced by `tick`.
    clock: f64,
    next_slow_tick: f64,
    next_chatter: f64,
    next_mood_save: f64,
    bubble: Option<Bubble>,
    queue: VecDeque<Bubble>,
    bubble_dirty: bool,
    pointer: Option<Pointer>,
    alerting: Option<u64>,
    scale_factor: f64,
    pub last_frame: Option<FrameView>,
    last_window: Option<WindowPlan>,
    pub system: Watch,
    pub breaks: BreakTracker,
    break_nudged: bool,
    pub focus: Focus,
    shake: ShakeDetector,
    /// Last foreground window rect (kept while one of ours is focused).
    perch: Option<Rect>,
    hungry_said_at: Option<f64>,
    last_tooltip: String,
    greeted: bool,
    /// The user was away (idle past the break length) at the last check.
    away_marker: bool,
    /// How many windows were climbable at the last tick.
    climbable: usize,
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
            rng: fastrand::Rng::new(),
            clock: 0.0,
            next_slow_tick: 0.0,
            next_chatter: interval,
            next_mood_save: MOOD_SAVE_SECS,
            bubble: None,
            queue: VecDeque::new(),
            bubble_dirty: false,
            pointer: None,
            alerting: None,
            scale_factor: 1.0,
            last_frame: None,
            last_window: None,
            system: Watch::default(),
            breaks: BreakTracker::default(),
            break_nudged: false,
            focus: Focus::default(),
            shake: ShakeDetector::default(),
            perch: None,
            hungry_said_at: None,
            last_tooltip: String::new(),
            greeted: false,
            away_marker: false,
            climbable: 0,
        };
        s.reload_packs();
        s
    }

    pub fn clock(&self) -> f64 {
        self.clock
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
        let s = &self.data.settings;
        if !s.emotions_enabled {
            return Feeling::Neutral;
        }
        if s.system_monitor_enabled && self.system.condition.low_battery {
            return Feeling::Sleepy;
        }
        self.data.mood.feeling(s.care_enabled)
    }

    // ------------------------------------------------------------ bubbles --

    fn show_bubble(&mut self, mut b: Bubble) {
        b.expires = b.ttl.map(|t| self.clock + t);
        self.bubble = Some(b);
        self.bubble_dirty = true;
    }

    fn clear_bubble(&mut self) {
        if self.bubble.take().is_some() {
            self.bubble_dirty = true;
        }
    }

    /// Show a passing message, unless something more important is up.
    pub fn say(&mut self, text: impl Into<String>) {
        self.say_for(text, f64::from(self.data.settings.bubble_seconds));
    }

    fn say_for(&mut self, text: impl Into<String>, secs: f64) {
        if matches!(&self.bubble, Some(b) if b.view.kind != BubbleKind::Say) {
            return;
        }
        let view = BubbleView {
            kind: BubbleKind::Say,
            reminder_id: None,
            title: String::new(),
            text: text.into(),
            actions: vec![],
        };
        self.show_bubble(Bubble { view, ttl: Some(secs), expires: None });
    }

    /// Show a suggestion with buttons now, or after the current one.
    fn nudge(&mut self, title: &str, text: String, actions: Vec<BubbleAction>, ttl: Option<f64>) {
        let view = BubbleView { kind: BubbleKind::Nudge, reminder_id: None, title: title.to_owned(), text, actions };
        let b = Bubble { view, ttl, expires: None };
        match &self.bubble {
            None => self.show_bubble(b),
            Some(cur) if cur.view.kind == BubbleKind::Say => self.show_bubble(b),
            Some(_) => self.queue.push_back(b),
        }
    }

    /// Dismiss the current bubble; the next queued one shows on the next tick.
    pub fn dismiss_bubble(&mut self) {
        self.clear_bubble();
    }

    // ------------------------------------------------------------- inputs --

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
            self.data.mood.apply(MoodEvent::Poked);
            if matches!(&self.bubble, Some(b) if b.view.kind == BubbleKind::Say) {
                self.clear_bubble();
            }
        }
    }

    pub fn feed(&mut self) {
        self.data.mood.apply(MoodEvent::Fed);
        self.pet.react(Anim::Eat, 2.5);
        let line = self.pick(&["Yum! Thank you!", "Nom nom nom...", "Delicious!"]);
        self.say(line);
    }

    pub fn play(&mut self) {
        self.data.mood.apply(MoodEvent::Played);
        self.pet.react(Anim::Play, 3.0);
        let line = self.pick(&["Wheee!", "Again, again!", "That was fun!"]);
        self.say(line);
    }

    /// Window climbing applies in the default taskbar mode.
    pub fn climbing(&self) -> bool {
        let s = &self.data.settings;
        s.climb_windows && s.roam_mode == RoamMode::BottomEdge
    }

    /// "Climb a window" from the menu, with a word on why not if it can't.
    pub fn climb_now(&mut self) {
        let s = &self.data.settings;
        if s.roam_mode != RoamMode::BottomEdge {
            self.say("I only climb windows when I live along the taskbar (Dashboard → Pet → Where it goes).");
        } else if !s.climb_windows {
            self.say("Climbing is turned off (Dashboard → Pet).");
        } else if self.climbable == 0 {
            self.say("I don't see a window I can climb. Maximised windows have no room on top, and I only use windows you've clicked in the last 10 minutes.");
        } else {
            self.pet.climb_now();
            self.say("Here I go!");
        }
    }

    /// "Do something fun" from the menu.
    pub fn fun_now(&mut self) {
        self.pet.fun_now();
    }

    /// The window the pet stands on, so it stays climbable while in use.
    pub fn standing_on(&self) -> Option<u64> {
        match self.pet.surface() {
            Surface::Window(id) => Some(id),
            Surface::Ground => None,
        }
    }

    // ------------------------------------------------------------ actions --

    /// A button in the speech bubble was pressed.
    pub fn bubble_action(&mut self, id: &str, now: NaiveDateTime) -> Result<(), String> {
        let (verb, arg) = id.split_once(':').unwrap_or((id, ""));
        let s = self.data.settings.clone();
        match (verb, arg) {
            ("done", n) => return self.reminder_done(parse_id(n)?, now),
            ("snooze", n) => return self.reminder_snooze(parse_id(n)?, now, None),
            ("ok", _) => self.clear_bubble(),
            ("break", "take") => {
                self.clear_bubble();
                self.breaks.reset();
                self.break_nudged = false;
                self.data.log(HistoryEntry {
                    at: now,
                    kind: HistoryKind::Break,
                    title: "Screen break".into(),
                    reminder_id: None,
                    waited_minutes: 0,
                });
                self.say("Enjoy your break! Stretch, drink some water.");
                self.save()?;
            }
            ("break", "later") => {
                self.clear_bubble();
                self.breaks.snooze(15);
            }
            ("focus", "break") => {
                self.clear_bubble();
                self.focus.start_break(
                    self.clock,
                    s.short_break_minutes,
                    s.long_break_minutes,
                    s.sessions_before_long_break,
                );
                let mins = self.focus.view(self.clock).total_secs / 60;
                self.say(format!("Break time: {mins} minutes. I'll call you back."));
            }
            ("focus", "again") => {
                self.clear_bubble();
                self.start_focus();
            }
            ("focus", _) => self.clear_bubble(),
            _ => return Err(format!("unknown action {id}")),
        }
        Ok(())
    }

    pub fn start_focus(&mut self) {
        let minutes = self.data.settings.focus_minutes;
        self.focus.start_focus(self.clock, minutes);
        self.say(format!("Focus time! {minutes} minutes. I'll keep quiet."));
    }

    pub fn stop_focus(&mut self) {
        self.focus.stop();
        self.say("Timer stopped.");
    }

    pub fn reminder_done(&mut self, id: u64, now: NaiveDateTime) -> Result<(), String> {
        self.data.complete_reminder(id, now).map_err(|e| e.to_string())?;
        self.data.mood.apply(MoodEvent::ReminderDone);
        self.pet.react(Anim::Happy, 1.5);
        self.clear_reminder_bubble(id);
        self.save()
    }

    pub fn reminder_snooze(&mut self, id: u64, now: NaiveDateTime, minutes: Option<u32>) -> Result<(), String> {
        let minutes = minutes.unwrap_or(self.data.settings.snooze_minutes);
        self.data.snooze_reminder(id, now, minutes).map_err(|e| e.to_string())?;
        self.data.mood.apply(MoodEvent::ReminderSnoozed);
        self.clear_reminder_bubble(id);
        self.save()
    }

    /// Drop the bubble for a reminder that was answered, edited or deleted.
    /// The next slow tick shows the next due reminder, if any.
    pub fn clear_reminder_bubble(&mut self, id: u64) {
        if matches!(&self.bubble, Some(b) if b.view.reminder_id == Some(id)) {
            self.clear_bubble();
        }
        if self.alerting == Some(id) {
            self.alerting = None;
        }
        self.next_slow_tick = self.clock;
    }

    // --------------------------------------------------------------- tick --

    pub fn tick(&mut self, dt: f64, env: &Env) -> TickOutput {
        let mut out = TickOutput::default();
        self.clock += dt;
        self.scale_factor = env.scale_factor;

        if !self.greeted {
            self.greeted = true;
            let name = self.data.settings.pet_name.clone();
            let hello = if self.data.settings.greetings_enabled {
                format!("{} I'm {name}.", greeting(env.now.hour()))
            } else {
                format!("Hi! I'm {name}.")
            };
            self.say(hello);
        }

        if self.clock >= self.next_slow_tick {
            self.next_slow_tick = self.clock + 1.0;
            self.slow_tick(env, &mut out);
        }
        if let Some(sample) = env.system {
            self.system_tick(sample, env.system_dt);
        }

        // Shake the mouse to startle the pet.
        if let Some((cx, _)) = env.cursor {
            if self.shake.push(self.clock, cx, env.scale_factor) && self.data.settings.shake_reaction {
                self.pet.react(Anim::Surprised, 1.2);
                let line = self.pick(&["Whoa! Earthquake?!", "Hey, I'm getting dizzy!", "What was that?!"]);
                self.say(line);
            }
        }

        self.drag_tick(env);

        let (sw, sh) = self.sprite_px();
        let bounds = self.bounds(env, sw, sh);
        let s = &self.data.settings;
        let gravity = s.roam_mode != RoamMode::WholeScreen;
        let follow = self.follow_target(env, bounds, sw, sh, gravity);
        let inputs = Inputs {
            dt,
            bounds,
            feeling: self.feeling(),
            gravity,
            user_away: env.idle_secs >= u64::from(s.idle_sleep_minutes) * 60,
            alert: self.alerting.is_some(),
            overheated: s.system_monitor_enabled && (self.system.condition.hot || self.system.condition.ram_full),
            speed: f64::from(s.walk_speed) * env.scale_factor,
            follow,
            sprite: (sw, sh),
            windows: if self.climbing() { &env.windows } else { &[] },
            climbs_per_hour: if self.climbing() { f64::from(s.climbs_per_hour) } else { 0.0 },
            fun_per_hour: if s.fun_enabled { f64::from(s.fun_per_hour) } else { 0.0 },
        };

        // Moved to another monitor (by the settings): hop over.
        let wa = env.work_area;
        let (cx, cy) = (self.pet.x + sw / 2.0, self.pet.y + sh / 2.0);
        let outside = cx < wa.left - sw || cx > wa.right + sw || cy < wa.top - sh || cy > wa.bottom + sh;
        if outside && !self.pet.is_dragged() {
            self.pet.teleport(wa.left + (wa.width() - sw) / 2.0, bounds.floor);
        }
        self.pet.tick(inputs, &mut self.rng);
        self.climbable = env.windows.len();
        if self.pet.take_climb_failed() {
            self.say("I can't find a way up any window right now.");
        }

        if let Some(b) = &self.bubble {
            if b.expires.is_some_and(|t| self.clock >= t) {
                self.clear_bubble();
            }
        }
        if self.bubble.is_none() {
            if let Some(next) = self.queue.pop_front() {
                self.show_bubble(next);
            }
        }

        if self.clock >= self.next_mood_save {
            self.next_mood_save = self.clock + MOOD_SAVE_SECS;
            if let Err(e) = self.save() {
                eprintln!("DesktopPet: could not save: {e}");
            }
        }

        self.layout(env, sw, sh, &mut out);
        if std::mem::take(&mut self.bubble_dirty) {
            out.bubble = Some(self.bubble_view());
        }
        out
    }

    fn drag_tick(&mut self, env: &Env) {
        let Some(p) = &mut self.pointer else { return };
        if let Some(c) = env.cursor {
            let moved = (c.0 - p.start.0).hypot(c.1 - p.start.1);
            if !p.dragging && moved > DRAG_THRESHOLD * env.scale_factor {
                p.dragging = true;
                self.pet.start_drag();
                self.data.mood.apply(MoodEvent::Dragged);
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

    /// The floor and walls for the current roaming mode.
    fn bounds(&mut self, env: &Env, sw: f64, sh: f64) -> Bounds {
        let wa = env.work_area;
        let bottom = Bounds { left: wa.left, right: wa.right - sw, top: wa.top, floor: wa.bottom - sh };
        match self.data.settings.roam_mode {
            RoamMode::BottomEdge | RoamMode::WholeScreen => bottom,
            RoamMode::ActiveWindow => {
                match env.foreground {
                    Foreground::Window(_, r) => self.perch = Some(r),
                    Foreground::None => self.perch = None,
                    Foreground::Ours => {}
                }
                // Sit on the window's top edge if it is on this monitor and
                // there is room above it; otherwise use the taskbar.
                match self.perch {
                    Some(r)
                        if r.top - sh >= wa.top
                            && r.width() >= sw * 1.5
                            && r.left < wa.right
                            && r.right > wa.left
                            && r.top < wa.bottom =>
                    {
                        let feet = 2.0 * env.scale_factor;
                        Bounds {
                            left: r.left.max(wa.left),
                            right: r.right.min(wa.right) - sw,
                            top: wa.top,
                            floor: r.top - sh + feet,
                        }
                    }
                    _ => bottom,
                }
            }
        }
    }

    /// Where to walk if the pet decides to visit the mouse pointer.
    fn follow_target(&self, env: &Env, b: Bounds, sw: f64, sh: f64, gravity: bool) -> Option<(f64, f64)> {
        if !self.data.settings.follow_cursor || self.alerting.is_some() || self.focus.is_focusing() {
            return None;
        }
        let (cx, cy) = env.cursor?;
        if !env.work_area.contains(cx, cy) {
            return None;
        }
        let tx = (cx - sw / 2.0).clamp(b.left, b.right.max(b.left));
        let ty = if gravity { b.floor } else { (cy - sh / 2.0).clamp(b.top, b.floor) };
        // Already close: no need to come over.
        let far = (tx - self.pet.x).hypot(ty - self.pet.y) > 150.0 * env.scale_factor;
        far.then_some((tx, ty))
    }

    fn layout(&mut self, env: &Env, sw: f64, sh: f64, out: &mut TickOutput) {
        // The window covers exactly the sprite, plus the speech bubble above
        // it and the ladder while they are shown. Keeping it tight means no
        // invisible area blocks clicks to the apps behind. (Click-through via
        // `set_ignore_cursor_events` is not an option: on Windows it makes the
        // window WS_EX_LAYERED, and the WebView2 content then never draws.)
        let sf = env.scale_factor;
        let wa = env.work_area;
        let sprite = Rect { left: self.pet.x, top: self.pet.y, right: self.pet.x + sw, bottom: self.pet.y + sh };
        let mut area = sprite;
        if self.bubble.is_some() {
            let (bw, bh) = ((BUBBLE_W * sf).round(), (BUBBLE_H * sf).round());
            let mut bx = self.pet.x + sw / 2.0 - bw / 2.0;
            if !self.pet.is_dragged() {
                bx = bx.clamp(wa.left, (wa.right - bw).max(wa.left));
            }
            area = area.union(&Rect { left: bx, top: self.pet.y - bh, right: bx + bw, bottom: self.pet.y });
        }
        let ladder = self.pet.ladder().map(|l| {
            let lw = (sw * 0.42).round();
            Rect { left: l.x - lw / 2.0, top: l.top, right: l.x + lw / 2.0, bottom: l.bottom }
        });
        if let Some(l) = ladder {
            area = area.union(&l);
        }
        let (x, y) = (area.left.floor(), area.top.floor());
        let plan = WindowPlan {
            x: x as i32,
            y: y as i32,
            width: (area.right.ceil() - x) as u32,
            height: (area.bottom.ceil() - y) as u32,
        };
        if self.last_window != Some(plan) {
            self.last_window = Some(plan);
            out.window = Some(plan);
        }

        let css = |v: f64| (v / sf * 10.0).round() / 10.0;
        let frame = FrameView {
            anim: self.pet.anim(),
            facing: self.pet.facing,
            window_width: css(f64::from(plan.width)),
            window_height: css(f64::from(plan.height)),
            sprite_left: css(sprite.left - x),
            sprite_top: css(sprite.top - y),
            sprite_width: css(sw),
            sprite_height: css(sh),
            ladder: ladder.map(|l| LadderView {
                left: css(l.left - x),
                top: css(l.top - y),
                width: css(l.width()),
                height: css(l.bottom - l.top),
            }),
        };
        if self.last_frame.as_ref() != Some(&frame) {
            self.last_frame = Some(frame.clone());
            out.frame = Some(frame);
        }
    }

    fn system_tick(&mut self, sample: Sample, dt: f32) {
        let s = &self.data.settings;
        if !s.system_monitor_enabled {
            self.system.condition = Default::default();
            return;
        }
        let limits = system::Limits {
            cpu: s.cpu_alert_percent as f32,
            ram: s.ram_alert_percent as f32,
            battery_low: s.battery_low_percent as f32,
            disk_low: s.disk_low_percent as f32,
        };
        let speech = s.system_speech;
        let alerts = self.system.update(sample, dt, self.clock, limits);
        if speech && !self.focus.is_focusing() {
            if let Some(a) = alerts.first() {
                self.pet.react(Anim::Surprised, 1.0);
                self.say_for(a.message(), 12.0);
            }
        }
    }

    /// Once a second: reminders, mood, breaks, focus, summary and chatter.
    fn slow_tick(&mut self, env: &Env, out: &mut TickOutput) {
        let now = env.now;
        let hidden = !env.pet_visible || env.fullscreen;
        let notify = self.data.settings.notify_when_hidden && hidden;

        // ---- reminders ----
        let first = self
            .data
            .due_reminders(now)
            .first()
            .map(|r| (r.id, r.title.clone(), r.details.clone(), (now - r.fire_at()).num_seconds().max(0) as u64));
        let mut ignored_secs = 0;
        match first {
            Some((id, title, details, waited)) => {
                ignored_secs = waited;
                let showing = matches!(&self.bubble, Some(b) if b.view.reminder_id == Some(id));
                if !showing {
                    // A suggestion that was up waits until the reminder is answered.
                    if let Some(prev) = self.bubble.take().filter(|b| b.view.kind == BubbleKind::Nudge) {
                        self.queue.push_front(prev);
                    }
                    let view = BubbleView {
                        kind: BubbleKind::Reminder,
                        reminder_id: Some(id),
                        title: title.clone(),
                        text: details.clone(),
                        actions: vec![action(format!("done:{id}"), "Done"), action(format!("snooze:{id}"), "Snooze")],
                    };
                    self.show_bubble(Bubble { view, ttl: None, expires: None });
                }
                if self.alerting != Some(id) {
                    out.show_pet = true;
                    if notify {
                        out.notify.push((format!("⏰ {title}"), details));
                    }
                }
                self.alerting = Some(id);
            }
            None => {
                self.alerting = None;
                if matches!(&self.bubble, Some(b) if b.view.kind == BubbleKind::Reminder) {
                    self.clear_bubble();
                }
            }
        }

        // ---- mood ----
        let s = self.data.settings.clone();
        if s.emotions_enabled {
            let ctx = MoodContext {
                hour: now.hour(),
                sleeping: self.pet.is_sleeping(),
                ignored_reminder_secs: ignored_secs,
                care: s.care_enabled,
            };
            self.data.mood.tick(1.0, ctx);
            if s.care_enabled && self.feeling() == Feeling::Hungry {
                let due = self.hungry_said_at.is_none_or(|t| self.clock - t > 30.0 * 60.0);
                // Let the startup greeting be read first.
                if due && self.clock > 15.0 && !self.focus.is_focusing() {
                    self.hungry_said_at = Some(self.clock);
                    self.say("I'm hungry... right-click me to feed me?");
                }
            }
        }

        // ---- breaks ----
        if s.break_reminders_enabled {
            match self.breaks.tick(1.0, env.idle_secs, s.break_every_minutes, s.break_away_minutes) {
                Some(BreakEvent::Nudge) if !self.focus.is_focusing() => {
                    self.break_nudged = true;
                    let mins = (self.breaks.active_secs / 60.0).round();
                    self.nudge(
                        "Time for a break?",
                        format!(
                            "You've been at the screen for {mins:.0} minutes. Stretch, look away, drink some water."
                        ),
                        vec![action("break:take", "Taking one"), action("break:later", "In 15 min")],
                        Some(90.0),
                    );
                    if notify {
                        out.notify.push(("Time for a break?".into(), format!("{mins:.0} minutes at the screen.")));
                    }
                }
                Some(BreakEvent::TookBreak) if self.break_nudged => {
                    self.break_nudged = false;
                    self.data.log(HistoryEntry {
                        at: now,
                        kind: HistoryKind::Break,
                        title: "Screen break".into(),
                        reminder_id: None,
                        waited_minutes: 0,
                    });
                    out.data_changed = true;
                }
                _ => {}
            }
        }
        // Welcome back after being away a while.
        let returned = self.was_away(env);
        if returned && s.greetings_enabled && !self.pet.is_dragged() {
            self.pet.react(Anim::Happy, 1.5);
            self.say("Welcome back!");
        }

        // ---- focus timer ----
        match self.focus.tick(self.clock) {
            Some(FocusEvent::FocusDone) => {
                self.data.log(HistoryEntry {
                    at: now,
                    kind: HistoryKind::Focus,
                    title: format!("{} min focus", s.focus_minutes),
                    reminder_id: None,
                    waited_minutes: 0,
                });
                out.data_changed = true;
                self.pet.react(Anim::Happy, 2.0);
                let long = self.focus.streak >= s.sessions_before_long_break;
                let mins = if long { s.long_break_minutes } else { s.short_break_minutes };
                self.nudge(
                    "Focus session done! 🎉",
                    format!("Nice work. Take a {mins}-minute break?"),
                    vec![action("focus:break", "Start break"), action("focus:skip", "Skip")],
                    Some(10.0 * 60.0),
                );
                if notify {
                    out.notify.push(("Focus session done!".into(), format!("Take a {mins}-minute break.")));
                }
                let _ = self.save();
            }
            Some(FocusEvent::BreakDone) => {
                self.nudge(
                    "Break's over",
                    "Ready for another focus session?".into(),
                    vec![action("focus:again", "Start focus"), action("focus:no", "Not now")],
                    Some(10.0 * 60.0),
                );
                if notify {
                    out.notify.push(("Break's over".into(), "Ready for another focus session?".into()));
                }
            }
            None => {}
        }
        let tooltip = match self.focus.view(self.clock) {
            v if v.phase == "focus" => format!("DesktopPet: focusing, {} min left", v.remaining_secs.div_ceil(60)),
            v if v.phase == "break" => format!("DesktopPet: break, {} min left", v.remaining_secs.div_ceil(60)),
            _ => "DesktopPet".to_owned(),
        };
        if tooltip != self.last_tooltip {
            self.last_tooltip = tooltip.clone();
            out.tooltip = Some(tooltip);
        }

        // ---- daily summary ----
        let today = now.date();
        let summary_time = (s.daily_summary_hour, s.daily_summary_minute);
        if s.daily_summary_enabled
            && (now.hour(), now.minute()) >= summary_time
            && self.data.last_summary != Some(today)
        {
            self.data.last_summary = Some(today);
            let text = self.summary_text(now);
            self.nudge("Today's plan", text.clone(), vec![action("ok", "OK")], Some(45.0));
            if notify {
                out.notify.push(("Today's plan".into(), text));
            }
            let _ = self.save();
        }

        // ---- chatter ----
        if self.clock >= self.next_chatter {
            let interval = f64::from(s.chatter_interval_minutes) * 60.0;
            self.next_chatter = self.clock + interval * (0.75 + self.rng.f64() * 0.5);
            if s.chatter_enabled
                && !s.is_quiet(now.hour())
                && self.bubble.is_none()
                && self.alerting.is_none()
                && !self.pet.is_sleeping()
                && !self.focus.is_focusing()
            {
                let line = self.random_line();
                self.say(line);
            }
        }
    }

    /// True once, right as the user comes back from being idle a while.
    fn was_away(&mut self, env: &Env) -> bool {
        let threshold = u64::from(self.data.settings.break_away_minutes) * 60;
        let returned = self.away_marker && env.idle_secs < threshold;
        self.away_marker = env.idle_secs >= threshold;
        returned
    }

    fn summary_text(&self, now: NaiveDateTime) -> String {
        let today = self.data.reminders_on(now.date());
        if today.is_empty() {
            return "Nothing scheduled today. Enjoy!".into();
        }
        let mut lines: Vec<String> = today
            .iter()
            .take(4)
            .map(|r| {
                let at = r.fire_at();
                let time = if at.date() == now.date() { at.time() } else { r.due.time() };
                format!("{} {}", time.format("%H:%M"), r.title)
            })
            .collect();
        if today.len() > 4 {
            lines.push(format!("…and {} more", today.len() - 4));
        }
        format!("{} today:\n{}", plural(today.len(), "reminder"), lines.join("\n"))
    }

    fn random_line(&mut self) -> String {
        let notes = &self.data.notes;
        if !notes.is_empty() && self.rng.u8(0..10) < 6 {
            return format!("Don't forget: {}", notes[self.rng.usize(..notes.len())].text);
        }
        let lines: &[&str] = match self.feeling() {
            Feeling::Happy => &["Today is a good day!", "*happy wiggle*", "You're doing great!"],
            Feeling::Sad => &["I feel a bit lonely...", "Could you poke me?"],
            Feeling::Bored => &["I'm bored... play with me? (right-click)", "*yawns*", "Anything fun to do?"],
            Feeling::Hungry => &["My tummy is rumbling...", "Is it snack time?"],
            Feeling::Neutral | Feeling::Sleepy => {
                &["*stretches*", "I'm right here if you need me.", "Right-click me for options."]
            }
        };
        lines[self.rng.usize(..lines.len())].to_owned()
    }
}

impl AppState {
    fn pick(&mut self, lines: &[&'static str]) -> &'static str {
        lines[self.rng.usize(..lines.len())]
    }
}

fn greeting(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning!",
        12..=16 => "Good afternoon!",
        17..=21 => "Good evening!",
        _ => "Hello, night owl!",
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn parse_id(s: &str) -> Result<u64, String> {
    s.parse().map_err(|_| format!("bad id {s}"))
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
    use pet_core::system::Sample;

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(h, m, 0).unwrap()
    }

    fn env(now: NaiveDateTime) -> Env {
        Env {
            now,
            cursor: None,
            left_down: None,
            idle_secs: 0,
            work_area: Rect { left: 0.0, top: 0.0, right: 1920.0, bottom: 1040.0 },
            scale_factor: 1.0,
            foreground: Foreground::None,
            pet_visible: true,
            fullscreen: false,
            system: None,
            system_dt: 0.0,
            windows: vec![],
        }
    }

    fn state() -> AppState {
        let packs = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../pets"));
        let dir = std::env::temp_dir().join(format!("desktoppet-test-{}", fastrand::u64(..)));
        let mut data = Data::default();
        // Keep tests about one thing at a time.
        data.settings.daily_summary_enabled = false;
        data.settings.follow_cursor = false;
        AppState::new(data, dir.join("data.json"), vec![packs])
    }

    fn run(s: &mut AppState, e: &Env, secs: f64) -> Vec<TickOutput> {
        (0..(secs / 0.03) as usize).map(|_| s.tick(0.03, e)).collect()
    }

    fn last_bubble(outs: &[TickOutput]) -> Option<Option<BubbleView>> {
        outs.iter().rev().find_map(|o| o.bubble.clone())
    }

    fn draft(title: &str, due: NaiveDateTime) -> ReminderDraft {
        ReminderDraft {
            title: title.into(),
            details: "".into(),
            due,
            repeat: Repeat::Once,
            enabled: true,
            category_id: None,
        }
    }

    #[test]
    fn built_in_packs_load() {
        let s = state();
        assert!(s.pack_errors.is_empty(), "{:?}", s.pack_errors);
        let ids: Vec<_> = s.packs.iter().map(|p| p.manifest.id.as_str()).collect();
        assert!(ids.contains(&"panda") && ids.contains(&"cat") && ids.contains(&"blue-cat"), "{ids:?}");
        assert_eq!(s.loaded_pack().unwrap().id, "panda");
        // The generated packs draw every animation themselves, no fallbacks.
        for id in ["panda", "cat"] {
            let pack = s.packs.iter().find(|p| p.manifest.id == id).unwrap();
            for anim in Anim::ALL {
                assert!(pack.manifest.animations.contains_key(anim.key()), "{id} lacks {}", anim.key());
            }
        }
    }

    #[test]
    fn greets_by_time_of_day() {
        let mut s = state();
        let b = last_bubble(&run(&mut s, &env(at(9, 0)), 0.1)).unwrap().unwrap();
        assert_eq!(b.text, "Good morning! I'm Bamboo.");
    }

    #[test]
    fn reminder_bubble_until_done_with_notification_when_hidden() {
        let mut s = state();
        let id = s.data.add_reminder(draft("Call mom", at(9, 5))).unwrap();
        run(&mut s, &env(at(9, 4)), 1.2);
        assert!(s.alerting.is_none());

        let mut e = env(at(9, 6));
        e.pet_visible = false;
        let outs = run(&mut s, &e, 1.2);
        let b = last_bubble(&outs).unwrap().unwrap();
        assert_eq!((b.kind, b.reminder_id, b.title.as_str()), (BubbleKind::Reminder, Some(id), "Call mom"));
        assert_eq!(
            b.actions.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            vec![format!("done:{id}"), format!("snooze:{id}")]
        );
        assert!(outs.iter().any(|o| o.show_pet));
        assert_eq!(outs.iter().flat_map(|o| o.notify.clone()).count(), 1, "notified once");
        assert_eq!(s.pet.anim(), Anim::Alert);

        s.bubble_action(&format!("done:{id}"), at(9, 6)).unwrap();
        assert!(s.bubble_view().is_none());
        run(&mut s, &env(at(9, 7)), 1.2);
        assert!(s.alerting.is_none());
        assert_eq!(s.data.history.len(), 1);
    }

    #[test]
    fn suggestions_wait_behind_reminders() {
        let mut s = state();
        run(&mut s, &env(at(12, 0)), 0.1);
        s.nudge("Nudge", "text".into(), vec![action("ok", "OK")], None);
        let id = s.data.add_reminder(draft("Due", at(11, 0))).unwrap();
        run(&mut s, &env(at(12, 0)), 1.2);
        assert_eq!(s.bubble_view().unwrap().kind, BubbleKind::Reminder);
        s.bubble_action(&format!("snooze:{id}"), at(12, 0)).unwrap();
        run(&mut s, &env(at(12, 0)), 0.1);
        assert_eq!(s.bubble_view().unwrap().title, "Nudge");
        s.bubble_action("ok", at(12, 0)).unwrap();
        assert!(s.bubble_view().is_none());
    }

    #[test]
    fn window_hugs_the_sprite_and_grows_for_bubbles() {
        let mut s = state();
        let outs = run(&mut s, &env(at(12, 0)), 3.0);
        let plan = outs.iter().rev().find_map(|o| o.window).unwrap();
        assert_eq!(plan.y + plan.height as i32, 1040);
        assert_eq!((plan.width, plan.height), (240, 128 + 150));

        let outs = run(&mut s, &env(at(12, 0)), 10.0);
        let plan = outs.iter().rev().find_map(|o| o.window).unwrap();
        assert!(s.bubble_view().is_none());
        assert_eq!((plan.width, plan.height), (128, 128));
    }

    #[test]
    fn press_without_move_pokes_and_with_move_drags() {
        let mut s = state();
        let mut e = env(at(12, 0));
        run(&mut s, &e, 3.0);
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
        run(&mut s, &e, 2.0);
        assert_eq!(s.pet.y, 1040.0 - 128.0);
    }

    #[test]
    fn perches_on_the_active_window() {
        let mut s = state();
        s.data.settings.roam_mode = RoamMode::ActiveWindow;
        let mut e = env(at(12, 0));
        e.foreground = Foreground::Window(7, Rect { left: 300.0, top: 400.0, right: 1200.0, bottom: 900.0 });
        run(&mut s, &e, 3.0);
        assert_eq!(s.pet.y, 400.0 - 128.0 + 2.0);
        assert!((300.0..=1200.0 - 128.0).contains(&s.pet.x));

        // Focusing one of our own windows keeps the perch...
        e.foreground = Foreground::Ours;
        run(&mut s, &e, 1.0);
        assert_eq!(s.pet.y, 400.0 - 128.0 + 2.0);
        // ...the desktop sends it back down to the taskbar.
        e.foreground = Foreground::None;
        run(&mut s, &e, 2.0);
        assert_eq!(s.pet.y, 1040.0 - 128.0);
    }

    #[test]
    fn climbs_recent_windows_only_in_taskbar_mode() {
        let mut s = state();
        s.data.settings.climbs_per_hour = 20;
        let mut e = env(at(12, 0));
        e.windows = vec![WindowRect {
            id: 42,
            left: 400.0,
            top: 500.0,
            right: 1400.0,
            bottom: 1040.0,
            walk_left: 408.0,
            walk_right: 1250.0,
        }];
        // Fast-forward until it has climbed (20 trips an hour = one every ~3 min).
        let mut climbed = false;
        for _ in 0..(15 * 60 * 30) {
            s.tick(1.0 / 30.0, &e);
            if s.standing_on() == Some(42) {
                climbed = true;
                break;
            }
        }
        assert!(climbed);

        // Other modes ignore the windows: it falls back down.
        s.data.settings.roam_mode = RoamMode::WholeScreen;
        assert!(!s.climbing());
        s.data.settings.roam_mode = RoamMode::BottomEdge;
        s.data.settings.climb_windows = false;
        for _ in 0..90 {
            s.tick(1.0 / 30.0, &e);
        }
        assert_eq!(s.standing_on(), None);
    }

    #[test]
    fn moves_to_the_chosen_monitor() {
        let mut s = state();
        run(&mut s, &env(at(12, 0)), 2.0);
        let mut e = env(at(12, 0));
        e.work_area = Rect { left: 1920.0, top: 0.0, right: 3840.0, bottom: 1040.0 };
        run(&mut s, &e, 2.0);
        assert!(s.pet.x >= 1920.0, "x = {}", s.pet.x);
    }

    #[test]
    fn break_nudge_after_long_screen_time() {
        let mut s = state();
        s.data.settings.break_every_minutes = 5;
        let e = env(at(12, 0));
        // Fast-forward: one-second slow ticks drive the break tracker.
        for _ in 0..(5 * 60 + 2) {
            s.tick(1.0, &e);
        }
        let b = s.bubble_view().unwrap();
        assert_eq!(b.title, "Time for a break?");
        s.bubble_action("break:take", at(12, 5)).unwrap();
        assert_eq!(s.data.history.last().unwrap().kind, HistoryKind::Break);
    }

    #[test]
    fn focus_session_flow() {
        let mut s = state();
        s.data.settings.focus_minutes = 1;
        let e = env(at(12, 0));
        run(&mut s, &e, 0.1);
        s.start_focus();
        for _ in 0..62 {
            s.tick(1.0, &e);
        }
        assert_eq!(s.bubble_view().unwrap().title, "Focus session done! 🎉");
        assert_eq!(s.data.history.last().unwrap().kind, HistoryKind::Focus);
        s.bubble_action("focus:break", at(12, 1)).unwrap();
        assert_eq!(s.focus.view(s.clock()).phase, "break");
    }

    #[test]
    fn daily_summary_once_a_day() {
        let mut s = state();
        s.data.settings.daily_summary_enabled = true;
        s.data.add_reminder(draft("Lunch", at(13, 0))).unwrap();
        run(&mut s, &env(at(8, 59)), 1.2);
        assert!(s.data.last_summary.is_none());
        run(&mut s, &env(at(9, 0)), 1.2);
        let b = s.bubble_view().unwrap();
        assert_eq!(b.title, "Today's plan");
        assert!(b.text.contains("13:00 Lunch"), "{}", b.text);
        s.bubble_action("ok", at(9, 0)).unwrap();
        run(&mut s, &env(at(10, 0)), 2.0);
        assert!(s.bubble_view().is_none_or(|b| b.title != "Today's plan"));
    }

    #[test]
    fn high_cpu_makes_it_sweat_and_speak() {
        let mut s = state();
        let mut e = env(at(12, 0));
        run(&mut s, &e, 10.0); // let the greeting expire
        e.system = Some(Sample { cpu_percent: Some(99.0), ..Default::default() });
        e.system_dt = 2.0;
        for _ in 0..60 {
            s.tick(0.03, &e);
        }
        assert!(s.system.condition.hot);
        assert!(s.bubble_view().unwrap().text.contains("CPU"));
    }

    #[test]
    fn feeding_and_hunger() {
        let mut s = state();
        s.data.mood.fullness = 5.0;
        assert_eq!(s.feeling(), Feeling::Hungry);
        s.feed();
        assert_ne!(s.feeling(), Feeling::Hungry);
        assert_eq!(s.pet.anim(), Anim::Eat);
    }

    #[test]
    fn welcome_back_after_a_break() {
        let mut s = state();
        let mut e = env(at(12, 0));
        run(&mut s, &e, 10.0);
        e.idle_secs = 10 * 60;
        run(&mut s, &e, 1.2);
        e.idle_secs = 0;
        run(&mut s, &e, 1.2);
        assert_eq!(s.bubble_view().unwrap().text, "Welcome back!");
    }
}
