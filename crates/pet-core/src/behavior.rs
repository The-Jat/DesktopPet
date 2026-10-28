//! What the pet is doing on screen: walking, sitting, sleeping, reacting...
//!
//! Coordinates are physical screen pixels of the sprite's top-left corner.

use crate::mood::Feeling;
use fastrand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Animation names. These are the keys a pet pack's manifest can define.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Anim {
    Idle,
    Walk,
    Sit,
    Sleep,
    Happy,
    Sad,
    Bored,
    Surprised,
    Drag,
    Alert,
    Eat,
    Play,
    /// Hot and bothered: the computer is working hard.
    Sweat,
    Hungry,
    /// Climbing up the side of a window.
    Climb,
    /// In the air, jumping between surfaces.
    Jump,
    /// Sitting on a window's top edge, legs dangling over it.
    Perch,
    /// Walking with a ladder held overhead.
    Carry,
    /// Drifting down under an umbrella.
    Float,
    /// A puff of smoke (things appearing or vanishing).
    Poof,
    RideBike,
    RideMoto,
    RideCar,
    RideSkate,
    Dance,
    Read,
}

impl Anim {
    pub const ALL: [Anim; 26] = [
        Anim::Idle,
        Anim::Walk,
        Anim::Sit,
        Anim::Sleep,
        Anim::Happy,
        Anim::Sad,
        Anim::Bored,
        Anim::Surprised,
        Anim::Drag,
        Anim::Alert,
        Anim::Eat,
        Anim::Play,
        Anim::Sweat,
        Anim::Hungry,
        Anim::Climb,
        Anim::Jump,
        Anim::Perch,
        Anim::Carry,
        Anim::Float,
        Anim::Poof,
        Anim::RideBike,
        Anim::RideMoto,
        Anim::RideCar,
        Anim::RideSkate,
        Anim::Dance,
        Anim::Read,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Anim::Idle => "idle",
            Anim::Walk => "walk",
            Anim::Sit => "sit",
            Anim::Sleep => "sleep",
            Anim::Happy => "happy",
            Anim::Sad => "sad",
            Anim::Bored => "bored",
            Anim::Surprised => "surprised",
            Anim::Drag => "drag",
            Anim::Alert => "alert",
            Anim::Eat => "eat",
            Anim::Play => "play",
            Anim::Sweat => "sweat",
            Anim::Hungry => "hungry",
            Anim::Climb => "climb",
            Anim::Jump => "jump",
            Anim::Perch => "perch",
            Anim::Carry => "carry",
            Anim::Float => "float",
            Anim::Poof => "poof",
            Anim::RideBike => "rideBike",
            Anim::RideMoto => "rideMoto",
            Anim::RideCar => "rideCar",
            Anim::RideSkate => "rideSkate",
            Anim::Dance => "dance",
            Anim::Read => "read",
        }
    }

    /// Which animation to use when a pack does not define this one.
    pub fn fallback(self) -> Option<Anim> {
        match self {
            Anim::Idle => None,
            Anim::Walk | Anim::Sit | Anim::Happy | Anim::Sad => Some(Anim::Idle),
            Anim::Sleep | Anim::Bored => Some(Anim::Sit),
            Anim::Surprised | Anim::Alert | Anim::Eat | Anim::Play => Some(Anim::Happy),
            Anim::Drag => Some(Anim::Surprised),
            Anim::Sweat | Anim::Hungry => Some(Anim::Sad),
            // Arms up and legs hanging: close enough to clinging on.
            Anim::Climb => Some(Anim::Drag),
            Anim::Jump => Some(Anim::Happy),
            Anim::Perch | Anim::Read => Some(Anim::Sit),
            Anim::Carry | Anim::RideBike | Anim::RideMoto | Anim::RideCar | Anim::RideSkate => Some(Anim::Walk),
            Anim::Float => Some(Anim::Drag),
            Anim::Poof => Some(Anim::Surprised),
            Anim::Dance => Some(Anim::Happy),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Facing {
    Left,
    Right,
}

/// The area the pet may use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Smallest allowed sprite x.
    pub left: f64,
    /// Largest allowed sprite x.
    pub right: f64,
    /// Smallest allowed sprite y (only used without gravity).
    pub top: f64,
    /// Sprite y when standing on the floor (taskbar or window top), and the
    /// largest allowed y without gravity.
    pub floor: f64,
}

/// A window the pet may climb, in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRect {
    /// Stable identity (the window handle).
    pub id: u64,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    /// The part of the top edge the pet may stand on: not covered by other
    /// windows and away from the title-bar buttons.
    pub walk_left: f64,
    pub walk_right: f64,
}

/// A ladder the pet has set up against a window, in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ladder {
    /// Centre line.
    pub x: f64,
    pub top: f64,
    pub bottom: f64,
    /// The window's position when the ladder went up; if it moves, the
    /// ladder falls over.
    anchor: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vehicle {
    Bike,
    Moto,
    Car,
    Skate,
}

impl Vehicle {
    const ALL: [Vehicle; 4] = [Vehicle::Bike, Vehicle::Moto, Vehicle::Car, Vehicle::Skate];

    /// Speed relative to walking.
    fn speed(self) -> f64 {
        match self {
            Vehicle::Bike => 2.4,
            Vehicle::Moto => 3.6,
            Vehicle::Car => 4.2,
            Vehicle::Skate => 2.0,
        }
    }

    fn anim(self) -> Anim {
        match self {
            Vehicle::Bike => Anim::RideBike,
            Vehicle::Moto => Anim::RideMoto,
            Vehicle::Car => Anim::RideCar,
            Vehicle::Skate => Anim::RideSkate,
        }
    }
}

/// Something asked for from outside (the menu), done at the next chance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Force {
    Climb,
    Fun,
}

/// What the pet is standing on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Ground,
    Window(u64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Activity {
    Idle,
    /// Walk to a point (y is ignored with gravity).
    WalkTo {
        x: f64,
        y: f64,
    },
    Sit,
    Sleep,
    React(Anim),
    Alert,
    Dragged,
    Falling {
        vy: f64,
    },
    /// Going up the side of a window, or a ladder against it.
    Climb {
        id: u64,
        via: Via,
    },
    /// Drifting down under an umbrella.
    Float {
        vx: f64,
    },
    Ride {
        vehicle: Vehicle,
        left: f64,
    },
    /// A ballistic hop from one place to another.
    Jump {
        from: (f64, f64),
        to: (f64, f64),
        t: f64,
        dur: f64,
        land: Landing,
    },
    /// Sitting on a window's edge with legs dangling.
    Perch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Via {
    Wall(Side),
    Ladder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Landing {
    On(Surface),
    /// Grab a window's side wall, ready to climb.
    Wall,
}

/// One step of a multi-step outing (walk over, jump, climb...).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    Walk {
        x: f64,
    },
    Jump {
        x: f64,
        y: f64,
        land: Landing,
    },
    Climb {
        id: u64,
        via: Via,
    },
    Perch,
    /// Pull a ladder out of thin air (at the screen edge).
    Fetch,
    PlaceLadder {
        id: u64,
    },
    Float {
        dir: f64,
    },
    Poof,
    Ride {
        vehicle: Vehicle,
        secs: f64,
    },
    Act {
        anim: Anim,
        secs: f64,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    pub dt: f64,
    pub bounds: Bounds,
    pub feeling: Feeling,
    /// Stand on a floor and fall when dropped. `false` lets the pet wander
    /// the whole area.
    pub gravity: bool,
    /// The user has been away from keyboard and mouse long enough to nap.
    pub user_away: bool,
    /// A reminder is waiting for the user.
    pub alert: bool,
    /// The computer is under heavy load.
    pub overheated: bool,
    /// Walking speed in physical pixels per second.
    pub speed: f64,
    /// A sprite position worth walking over to (the mouse pointer), if any.
    pub follow: Option<(f64, f64)>,
    /// Sprite size in physical pixels.
    pub sprite: (f64, f64),
    /// Windows the pet may climb and stand on (only with gravity).
    pub windows: &'a [WindowRect],
    /// How many climbing trips to start per hour; 0 = never.
    pub climbs_per_hour: f64,
    /// How many fun things (rides, dancing, reading) per hour; 0 = never.
    pub fun_per_hour: f64,
}

#[derive(Debug, Clone)]
pub struct Pet {
    pub x: f64,
    pub y: f64,
    pub facing: Facing,
    activity: Activity,
    /// Seconds left in the current activity before choosing another.
    timer: f64,
    feeling: Feeling,
    overheated: bool,
    surface: Surface,
    /// Where the window under the pet was last tick, to ride along with it.
    ride: Option<(f64, f64)>,
    plan: VecDeque<Step>,
    /// Seconds until the next climbing trip (counts on the ground only).
    climb_timer: Option<f64>,
    /// Seconds left on the current window before heading back down.
    visit_timer: f64,
    fun_timer: Option<f64>,
    /// Holding a ladder.
    carrying: bool,
    ladder: Option<Ladder>,
    climb_phase: f64,
    force: Option<Force>,
    /// A requested climb found nothing reachable.
    climb_failed: bool,
}

const GRAVITY: f64 = 2400.0;
/// Highest jump from the ground to grab a window's side, in sprite heights.
const MAX_JUMP_UP: f64 = 2.5;
/// Widest gap to jump between windows, in sprite widths.
const MAX_JUMP_ACROSS: f64 = 4.0;
/// How far the feet sink into a window's top edge, as a share of the height.
const FEET: f64 = 0.04;
/// How far a perching pet sits below the edge (its legs hang over it).
const PERCH_DROP: f64 = 0.12;
/// Hand-over-hand pulls per second while climbing.
const CLIMB_CADENCE: f64 = 1.15;
const TAU: f64 = std::f64::consts::TAU;

impl Pet {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            facing: Facing::Right,
            activity: Activity::Idle,
            timer: 2.0,
            feeling: Feeling::Neutral,
            overheated: false,
            surface: Surface::Ground,
            ride: None,
            plan: VecDeque::new(),
            climb_timer: None,
            visit_timer: 0.0,
            fun_timer: None,
            carrying: false,
            ladder: None,
            climb_phase: 0.0,
            force: None,
            climb_failed: false,
        }
    }

    pub fn anim(&self) -> Anim {
        match self.activity {
            Activity::Idle | Activity::WalkTo { .. } if self.carrying => Anim::Carry,
            Activity::Idle => match self.feeling {
                _ if self.overheated => Anim::Sweat,
                Feeling::Sad => Anim::Sad,
                Feeling::Bored => Anim::Bored,
                Feeling::Happy => Anim::Happy,
                Feeling::Hungry => Anim::Hungry,
                Feeling::Neutral | Feeling::Sleepy => Anim::Idle,
            },
            Activity::WalkTo { .. } => Anim::Walk,
            Activity::Sit => match self.feeling {
                Feeling::Bored => Anim::Bored,
                _ => Anim::Sit,
            },
            Activity::Sleep => Anim::Sleep,
            Activity::React(a) => a,
            Activity::Alert => Anim::Alert,
            Activity::Dragged | Activity::Falling { .. } => Anim::Drag,
            Activity::Climb { .. } => Anim::Climb,
            Activity::Float { .. } => Anim::Float,
            Activity::Ride { vehicle, .. } => vehicle.anim(),
            Activity::Jump { .. } => Anim::Jump,
            Activity::Perch => Anim::Perch,
        }
    }

    pub fn surface(&self) -> Surface {
        self.surface
    }

    pub fn is_sleeping(&self) -> bool {
        self.activity == Activity::Sleep
    }

    pub fn is_dragged(&self) -> bool {
        self.activity == Activity::Dragged
    }

    pub fn is_reacting(&self) -> bool {
        matches!(self.activity, Activity::React(_))
    }

    /// Climbing or mid-jump: not to be interrupted.
    fn is_busy_moving(&self) -> bool {
        matches!(
            self.activity,
            Activity::Dragged
                | Activity::Falling { .. }
                | Activity::Climb { .. }
                | Activity::Jump { .. }
                | Activity::Float { .. }
        )
    }

    /// The user clicked the pet.
    pub fn poke(&mut self) {
        let anim = if self.is_sleeping() { Anim::Surprised } else { Anim::Happy };
        self.react(anim, 1.5);
    }

    /// The ladder standing against a window, if any.
    pub fn ladder(&self) -> Option<Ladder> {
        self.ladder
    }

    /// Climb a window at the next chance (from the menu).
    pub fn climb_now(&mut self) {
        self.force = Some(Force::Climb);
        self.climb_failed = false;
        self.wake_for_request();
    }

    /// Do something fun at the next chance (from the menu).
    pub fn fun_now(&mut self) {
        self.force = Some(Force::Fun);
        self.wake_for_request();
    }

    fn wake_for_request(&mut self) {
        if !self.is_busy_moving() && self.surface == Surface::Ground {
            self.abort_plan();
            self.activity = Activity::Idle;
            self.timer = 0.0;
        }
    }

    /// True once after a requested climb found no window to use.
    pub fn take_climb_failed(&mut self) -> bool {
        std::mem::take(&mut self.climb_failed)
    }

    fn abort_plan(&mut self) {
        self.plan.clear();
        self.carrying = false;
    }

    pub fn react(&mut self, anim: Anim, secs: f64) {
        if !self.is_busy_moving() {
            self.abort_plan();
            self.activity = Activity::React(anim);
            self.timer = secs;
        }
    }

    pub fn start_drag(&mut self) {
        self.activity = Activity::Dragged;
        self.abort_plan();
        self.ladder = None;
        self.surface = Surface::Ground;
        self.ride = None;
    }

    pub fn drag_to(&mut self, x: f64, y: f64) {
        if self.is_dragged() {
            self.x = x;
            self.y = y;
        }
    }

    pub fn end_drag(&mut self) {
        if self.is_dragged() {
            self.activity = Activity::Falling { vy: 0.0 };
        }
    }

    /// Jump straight to a place, e.g. onto another monitor.
    pub fn teleport(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
        self.surface = Surface::Ground;
        self.ride = None;
        self.abort_plan();
        self.ladder = None;
        if !self.is_dragged() {
            self.activity = Activity::Idle;
            self.timer = 1.0;
        }
    }

    pub fn tick(&mut self, inp: Inputs, rng: &mut Rng) {
        self.feeling = inp.feeling;
        self.overheated = inp.overheated;
        let (sw, sh) = inp.sprite;

        // ---- things in motion ----
        match self.activity {
            Activity::Dragged => return,
            Activity::Falling { vy } if inp.gravity => {
                let vy = vy + GRAVITY * inp.dt;
                let next = self.y + vy * inp.dt;
                match self.landing_between(self.y, next, &inp) {
                    Some((surface, y)) => {
                        self.y = y;
                        self.arrive(surface, &inp, rng);
                        self.activity = Activity::React(Anim::Surprised);
                        self.timer = 0.6;
                    }
                    None => {
                        self.y = next;
                        self.activity = Activity::Falling { vy };
                    }
                }
                return;
            }
            Activity::Falling { .. } => {
                // No gravity: stay where dropped.
                self.activity = Activity::React(Anim::Surprised);
                self.timer = 0.6;
            }
            Activity::Jump { from, to, t, dur, land } => {
                let t = (t + inp.dt / dur).min(1.0);
                let rise = (from.1 - to.1).max(0.0);
                let height = sh * 0.5 + rise * 0.3;
                self.x = from.0 + (to.0 - from.0) * t;
                self.y = from.1 + (to.1 - from.1) * t - height * 4.0 * t * (1.0 - t);
                if to.0 != from.0 {
                    self.facing = if to.0 > from.0 { Facing::Right } else { Facing::Left };
                }
                if t >= 1.0 {
                    match land {
                        Landing::On(surface) => {
                            // The target window may have gone while in the air.
                            let surface = match surface {
                                Surface::Window(id) if find(inp.windows, id).is_none() => Surface::Ground,
                                s => s,
                            };
                            self.arrive(surface, &inp, rng);
                        }
                        Landing::Wall => {}
                    }
                    self.activity = Activity::Idle;
                    self.timer = 0.4;
                    self.next_step(&inp, rng);
                } else {
                    self.activity = Activity::Jump { from, to, t, dur, land };
                }
                return;
            }
            Activity::Climb { id, via } => {
                let Some(w) = find(inp.windows, id).copied() else {
                    self.ladder = None;
                    self.fall();
                    return;
                };
                match via {
                    Via::Wall(side) => {
                        self.x = wall_x(&w, side, sw);
                        self.facing = if side == Side::Left { Facing::Right } else { Facing::Left };
                    }
                    Via::Ladder => match self.ladder {
                        Some(l) if (l.anchor.0 - w.left).abs() < 2.0 && (l.anchor.1 - w.top).abs() < 2.0 => {
                            self.x = l.x - sw / 2.0;
                        }
                        // The window moved: the ladder topples.
                        _ => {
                            self.ladder = None;
                            self.fall();
                            return;
                        }
                    },
                }
                // Hand over hand: quick pulls, short pauses.
                self.climb_phase += inp.dt * CLIMB_CADENCE;
                let pull = (self.climb_phase * TAU).sin().max(0.0);
                self.y -= (inp.speed * 1.5).max(45.0) * (0.12 + 1.7 * pull) * inp.dt;
                // Pull up once the head is over the top.
                if self.y <= w.top - sh * 0.8 {
                    self.y = standing_y(&w, sh);
                    let x = match via {
                        Via::Wall(Side::Left) => w.walk_left,
                        Via::Wall(Side::Right) => w.walk_right - sw,
                        Via::Ladder => self.x,
                    };
                    self.x = x.clamp(w.walk_left, (w.walk_right - sw).max(w.walk_left));
                    // Pull the ladder up after itself.
                    self.ladder = None;
                    self.arrive(Surface::Window(id), &inp, rng);
                    self.activity = Activity::React(Anim::Happy);
                    self.timer = 1.0;
                }
                return;
            }
            Activity::Float { vx } => {
                let next = self.y + (sh * 0.9).max(50.0) * inp.dt;
                let g = inp.bounds;
                self.x = (self.x + vx * inp.dt).clamp(g.left, g.right.max(g.left));
                // Only things below where it started: not the window it stepped off.
                match self.landing_between(self.y + 1.0, next, &inp) {
                    Some((surface, y)) => {
                        self.y = y;
                        self.arrive(surface, &inp, rng);
                        self.activity = Activity::React(Anim::Happy);
                        self.timer = 0.8;
                    }
                    None => self.y = next,
                }
                return;
            }
            _ => {}
        }

        // ---- standing on something ----
        let b = match self.surface_bounds(&inp) {
            Some(b) => b,
            None => {
                // The window under the pet closed, minimised or got covered.
                self.fall();
                return;
            }
        };

        if inp.gravity {
            let rest = if self.activity == Activity::Perch { b.floor + sh * PERCH_DROP } else { b.floor };
            // Dropped above the floor, or the floor moved down: fall.
            if self.y < rest - 1.0 && self.surface == Surface::Ground {
                self.activity = Activity::Falling { vy: 0.0 };
                return;
            }
            self.y = rest;
        } else {
            self.y = self.y.clamp(b.top.min(b.floor), b.floor);
        }
        self.x = self.x.clamp(b.left, b.right.max(b.left));

        if inp.alert {
            if !self.is_reacting() {
                self.abort_plan();
                self.activity = Activity::Alert;
            }
        } else if self.activity == Activity::Alert {
            self.activity = Activity::Idle;
            self.timer = 1.0;
        }

        if let Activity::WalkTo { x, y } = self.activity {
            let speed = inp.speed * if self.overheated { 0.6 } else { 1.0 };
            let ty = if inp.gravity { self.y } else { y };
            let (dx, dy) = (x.clamp(b.left, b.right.max(b.left)) - self.x, ty - self.y);
            let dist = dx.hypot(dy);
            let step = speed * inp.dt;
            if dist <= step.max(0.5) {
                self.x += dx;
                self.y += dy;
                self.activity = Activity::Idle;
                self.timer = 1.0 + rng.f64() * 3.0;
                if !self.plan.is_empty() {
                    self.next_step(&inp, rng);
                }
            } else {
                self.x += dx / dist * step;
                self.y += dy / dist * step;
            }
            if dx.abs() > 0.5 {
                self.facing = if dx > 0.0 { Facing::Right } else { Facing::Left };
            }
        }

        if let Activity::Ride { vehicle, left } = self.activity {
            let dir = if self.facing == Facing::Right { 1.0 } else { -1.0 };
            self.x += dir * inp.speed.max(20.0) * vehicle.speed() * inp.dt;
            if self.x <= b.left {
                self.x = b.left;
                self.facing = Facing::Right;
            } else if self.x >= b.right {
                self.x = b.right;
                self.facing = Facing::Left;
            }
            let left = left - inp.dt;
            if left <= 0.0 {
                self.activity = Activity::Idle;
                self.timer = 0.0;
            } else {
                self.activity = Activity::Ride { vehicle, left };
            }
        }

        // Wake up as soon as the user is back, if not actually tired.
        if self.is_sleeping() && !inp.user_away && inp.feeling != Feeling::Sleepy {
            self.timer = self.timer.min(0.0);
        }

        // Climbing trips are counted down while on the ground.
        if inp.gravity && self.surface == Surface::Ground {
            // The first one comes within a couple of minutes, then at the set rate.
            if inp.climbs_per_hour > 0.0 {
                let first = (3600.0 / inp.climbs_per_hour * (0.2 + rng.f64() * 0.3)).min(60.0 + rng.f64() * 60.0);
                *self.climb_timer.get_or_insert(first) -= inp.dt;
            }
            if inp.fun_per_hour > 0.0 {
                let first = (3600.0 / inp.fun_per_hour * (0.3 + rng.f64() * 0.5)).min(150.0 + rng.f64() * 120.0);
                *self.fun_timer.get_or_insert(first) -= inp.dt;
            }
        }
        if self.surface != Surface::Ground {
            self.visit_timer -= inp.dt;
        }

        self.timer -= inp.dt;
        if self.timer <= 0.0 && self.activity != Activity::Alert {
            if self.plan.is_empty() {
                self.choose_next(&inp, b, rng);
            } else {
                self.next_step(&inp, rng);
            }
        }
    }

    /// Bounds of what the pet stands on, or `None` if that window is gone.
    fn surface_bounds(&mut self, inp: &Inputs) -> Option<Bounds> {
        let (sw, sh) = inp.sprite;
        match self.surface {
            Surface::Ground => {
                self.ride = None;
                Some(inp.bounds)
            }
            Surface::Window(id) => {
                let w = find(inp.windows, id)?;
                if w.walk_right - w.walk_left < sw {
                    return None;
                }
                // Ride along when the window is dragged around.
                if let Some((left, _)) = self.ride {
                    self.x += w.left - left;
                }
                self.ride = Some((w.left, w.top));
                Some(Bounds {
                    left: w.walk_left,
                    right: w.walk_right - sw,
                    top: inp.bounds.top,
                    floor: standing_y(w, sh),
                })
            }
        }
    }

    fn fall(&mut self) {
        self.surface = Surface::Ground;
        self.ride = None;
        self.abort_plan();
        self.activity = Activity::Falling { vy: 0.0 };
    }

    fn arrive(&mut self, surface: Surface, inp: &Inputs, rng: &mut Rng) {
        if surface != self.surface {
            if let Surface::Window(_) = surface {
                self.visit_timer = 60.0 + rng.f64() * 120.0;
            }
        }
        self.surface = surface;
        self.ride = match surface {
            Surface::Window(id) => find(inp.windows, id).map(|w| (w.left, w.top)),
            Surface::Ground => None,
        };
    }

    /// Where a fall from `y0` to `y1` hits something: the highest window top
    /// under the pet's middle in that range, or the ground.
    fn landing_between(&self, y0: f64, y1: f64, inp: &Inputs) -> Option<(Surface, f64)> {
        let (sw, sh) = inp.sprite;
        let mid = self.x + sw / 2.0;
        let window = inp
            .windows
            .iter()
            .filter(|w| w.walk_right - w.walk_left >= sw && mid >= w.walk_left && mid <= w.walk_right)
            .map(|w| (Surface::Window(w.id), standing_y(w, sh)))
            .filter(|(_, y)| *y >= y0 - 0.5 && *y <= y1)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let ground = inp.bounds.floor;
        match window {
            Some(hit) if hit.1 <= ground => Some(hit),
            _ if y1 >= ground => Some((Surface::Ground, ground)),
            _ => None,
        }
    }

    /// Start the next queued step of an outing.
    fn next_step(&mut self, inp: &Inputs, rng: &mut Rng) {
        let Some(step) = self.plan.pop_front() else { return };
        let (_, sh) = inp.sprite;
        match step {
            Step::Walk { x } => {
                self.activity = Activity::WalkTo { x, y: self.y };
                self.timer = 30.0;
            }
            Step::Jump { x, y, land } => {
                let dist = (x - self.x).hypot(y - self.y);
                let dur = (0.45 + dist / (sh * 8.0)).clamp(0.45, 1.3);
                self.surface = Surface::Ground;
                self.ride = None;
                self.activity = Activity::Jump { from: (self.x, self.y), to: (x, y), t: 0.0, dur, land };
            }
            Step::Climb { id, via } => {
                let ladder_ok = via != Via::Ladder || self.ladder.is_some();
                if find(inp.windows, id).is_some() && ladder_ok {
                    self.climb_phase = 0.0;
                    self.activity = Activity::Climb { id, via };
                } else {
                    self.abort_plan();
                    self.ladder = None;
                    self.activity = Activity::Idle;
                    self.timer = 1.0;
                }
            }
            Step::Fetch => {
                // Rummage at the screen edge... and out comes a ladder.
                self.carrying = true;
                self.activity = Activity::React(Anim::Poof);
                self.timer = 0.6;
            }
            Step::PlaceLadder { id } => {
                self.carrying = false;
                let (sw, _) = inp.sprite;
                match find(inp.windows, id) {
                    Some(w) => {
                        self.ladder = Some(Ladder {
                            x: self.x + sw / 2.0,
                            top: w.top - sh * 0.12,
                            bottom: inp.bounds.floor + sh,
                            anchor: (w.left, w.top),
                        });
                        self.activity = Activity::Idle;
                        self.timer = 0.5;
                    }
                    None => {
                        self.abort_plan();
                        self.activity = Activity::Idle;
                        self.timer = 1.0;
                    }
                }
            }
            Step::Float { dir } => {
                self.surface = Surface::Ground;
                self.ride = None;
                self.activity = Activity::Float { vx: dir * (inp.speed * 0.5).max(15.0) };
            }
            Step::Poof => {
                self.activity = Activity::React(Anim::Poof);
                self.timer = 0.55;
            }
            Step::Ride { vehicle, secs } => {
                self.activity = Activity::Ride { vehicle, left: secs };
                self.timer = secs + 1.0;
            }
            Step::Act { anim, secs } => {
                self.activity = Activity::React(anim);
                self.timer = secs;
            }

            Step::Perch => {
                self.activity = Activity::Perch;
                self.timer = 10.0 + rng.f64() * 25.0;
            }
        }
    }

    fn choose_next(&mut self, inp: &Inputs, b: Bounds, rng: &mut Rng) {
        if inp.user_away || inp.feeling == Feeling::Sleepy {
            self.activity = Activity::Sleep;
            self.timer = 20.0 + rng.f64() * 40.0;
            return;
        }

        if self.surface == Surface::Ground {
            let forced = self.force.take();
            // Time for a climb?
            let timed_climb = inp.climbs_per_hour > 0.0 && self.climb_timer.is_some_and(|t| t <= 0.0);
            if forced == Some(Force::Climb) || timed_climb {
                if timed_climb {
                    self.climb_timer = Some(3600.0 / inp.climbs_per_hour * (0.5 + rng.f64()));
                }
                if self.plan_climb(inp, rng) {
                    self.next_step(inp, rng);
                    return;
                }
                self.climb_failed = forced == Some(Force::Climb);
            }
            // Time for something fun?
            let timed_fun = inp.fun_per_hour > 0.0 && self.fun_timer.is_some_and(|t| t <= 0.0);
            if (forced == Some(Force::Fun) || timed_fun) && inp.gravity {
                if timed_fun {
                    self.fun_timer = Some(3600.0 / inp.fun_per_hour * (0.5 + rng.f64()));
                }
                self.plan_fun(rng);
                self.next_step(inp, rng);
                return;
            }
            // Sometimes wander over to the mouse pointer.
            if let Some((fx, fy)) = inp.follow {
                if rng.u8(0..100) < 35 {
                    self.activity = Activity::WalkTo { x: fx, y: fy };
                    self.timer = 12.0;
                    return;
                }
            }
        } else if let Surface::Window(id) = self.surface {
            if let Some(w) = find(inp.windows, id).copied() {
                let roll = rng.u8(0..100);
                if self.visit_timer <= 0.0 {
                    self.plan_jump_down(&w, inp);
                    self.next_step(inp, rng);
                    return;
                }
                if roll < 15 && self.plan_jump_across(&w, inp, rng) {
                    self.next_step(inp, rng);
                    return;
                }
                if roll >= 85 {
                    // Sit on one end with the legs over the edge.
                    let x = if rng.bool() { b.left } else { b.right };
                    self.plan.extend([Step::Walk { x }, Step::Perch]);
                    self.next_step(inp, rng);
                    return;
                }
            }
        }

        // Weights for idle, walk, sit, hop.
        let (idle, walk, sit, hop) = match inp.feeling {
            _ if inp.overheated => (60, 15, 25, 0),
            Feeling::Happy => (25, 50, 10, 15),
            Feeling::Neutral => (40, 45, 15, 0),
            Feeling::Bored => (30, 25, 45, 0),
            Feeling::Sad | Feeling::Hungry => (60, 15, 25, 0),
            Feeling::Sleepy => unreachable!(),
        };
        let mut roll = rng.u32(0..idle + walk + sit + hop);
        let pick = |w: u32, roll: &mut u32| {
            if *roll < w {
                true
            } else {
                *roll -= w;
                false
            }
        };

        if pick(idle, &mut roll) {
            self.activity = Activity::Idle;
            self.timer = 2.0 + rng.f64() * 4.0;
        } else if pick(walk, &mut roll) {
            let span = (b.right - b.left).max(0.0);
            let target_x = if inp.gravity {
                // A stroll of up to a third of the floor, turning at the edges.
                let dist = (60.0 + rng.f64() * span / 3.0) * if rng.bool() { 1.0 } else { -1.0 };
                (self.x + dist).clamp(b.left, b.right.max(b.left))
            } else {
                b.left + rng.f64() * span
            };
            let target_y = if inp.gravity { self.y } else { b.top + rng.f64() * (b.floor - b.top).max(0.0) };
            self.activity = Activity::WalkTo { x: target_x, y: target_y };
            self.timer = 15.0;
        } else if pick(sit, &mut roll) {
            self.activity = Activity::Sit;
            self.timer = 4.0 + rng.f64() * 8.0;
        } else {
            self.activity = Activity::React(Anim::Happy);
            self.timer = 1.5;
        }
    }

    /// From the ground: walk to a window's side, jump to grab it if needed,
    /// and climb. Out of reach (or now and then, for fun): fetch a ladder from
    /// the screen edge and climb that. False if no window can be used at all.
    fn plan_climb(&mut self, inp: &Inputs, rng: &mut Rng) -> bool {
        let (sw, sh) = inp.sprite;
        let ground = inp.bounds;
        let mid = self.x + sw / 2.0;
        let mut walls: Vec<(u64, Side, f64, f64)> = Vec::new();
        let mut ladders: Vec<(u64, f64)> = Vec::new();
        for w in inp.windows {
            if w.walk_right - w.walk_left < sw * 1.2 || standing_y(w, sh) >= ground.floor - sh * 0.5 {
                continue;
            }
            // Grab the side where the window's bottom is, or as low as possible.
            let grab_y = (w.bottom - sh).min(ground.floor);
            if ground.floor - grab_y <= sh * MAX_JUMP_UP {
                for side in [Side::Left, Side::Right] {
                    let x = wall_x(w, side, sw);
                    if x >= ground.left && x <= ground.right {
                        walls.push((w.id, side, x, grab_y));
                    }
                }
            }
            // A ladder leaning on the window's front, near where the pet is.
            let lx = mid.clamp(w.walk_left + sw * 0.6, (w.walk_right - sw * 0.6).max(w.walk_left + sw * 0.6));
            if lx - sw / 2.0 >= ground.left && lx - sw / 2.0 <= ground.right {
                ladders.push((w.id, lx));
            }
        }
        let here = self.x;
        let by_distance = |x: f64| (x - here).abs();
        let use_ladder = !ladders.is_empty() && (walls.is_empty() || rng.u8(0..100) < 25);
        self.abort_plan();
        if use_ladder {
            ladders.sort_by(|a, b| by_distance(a.1).total_cmp(&by_distance(b.1)));
            let (id, lx) = ladders[rng.usize(..ladders.len().min(2))];
            let edge = if self.x - ground.left < ground.right - self.x { ground.left } else { ground.right };
            self.plan.extend([
                Step::Walk { x: edge },
                Step::Fetch,
                Step::Walk { x: lx - sw / 2.0 },
                Step::PlaceLadder { id },
                Step::Climb { id, via: Via::Ladder },
            ]);
            return true;
        }
        if walls.is_empty() {
            return false;
        }
        // Prefer a nearby wall, with some randomness.
        walls.sort_by(|a, b| by_distance(a.2).total_cmp(&by_distance(b.2)));
        let (id, side, x, grab_y) = walls[rng.usize(..walls.len().min(2))];
        self.plan.push_back(Step::Walk { x });
        if grab_y < ground.floor - 1.0 {
            self.plan.push_back(Step::Jump { x, y: grab_y, land: Landing::Wall });
        }
        self.plan.push_back(Step::Climb { id, via: Via::Wall(side) });
        true
    }

    /// A ride on something, a dance or a read.
    fn plan_fun(&mut self, rng: &mut Rng) {
        self.abort_plan();
        match rng.u8(0..100) {
            0..=19 => self.plan.push_back(Step::Act { anim: Anim::Dance, secs: 6.0 + rng.f64() * 5.0 }),
            20..=34 => self.plan.push_back(Step::Act { anim: Anim::Read, secs: 15.0 + rng.f64() * 20.0 }),
            _ => {
                let vehicle = Vehicle::ALL[rng.usize(..Vehicle::ALL.len())];
                self.facing = if rng.bool() { Facing::Left } else { Facing::Right };
                self.plan.extend([Step::Poof, Step::Ride { vehicle, secs: 8.0 + rng.f64() * 8.0 }, Step::Poof]);
            }
        }
    }

    /// Hop to another window whose top is within reach.
    fn plan_jump_across(&mut self, from: &WindowRect, inp: &Inputs, rng: &mut Rng) -> bool {
        let (sw, sh) = inp.sprite;
        let here = standing_y(from, sh);
        let targets: Vec<&WindowRect> = inp
            .windows
            .iter()
            .filter(|w| w.id != from.id && w.walk_right - w.walk_left >= sw * 1.2)
            .filter(|w| standing_y(w, sh) >= here - sh * 2.0)
            .filter(|w| gap(from, w) <= sw * MAX_JUMP_ACROSS)
            .collect();
        if targets.is_empty() {
            return false;
        }
        let to = targets[rng.usize(..targets.len())];
        // Take off from the end nearer the target, land on its near end.
        let (take_off, land_x) = if to.walk_left >= from.walk_right - sw {
            (from.walk_right - sw, to.walk_left)
        } else if to.walk_right <= from.walk_left + sw {
            (from.walk_left, to.walk_right - sw)
        } else {
            // Overlapping (one above the other): jump straight across.
            let x = self.x.clamp(to.walk_left, to.walk_right - sw);
            (self.x, x)
        };
        self.abort_plan();
        self.plan.extend([
            Step::Walk { x: take_off },
            Step::Jump { x: land_x, y: standing_y(to, sh), land: Landing::On(Surface::Window(to.id)) },
        ]);
        true
    }

    /// Back down to the ground from the nearer end of the window.
    fn plan_jump_down(&mut self, w: &WindowRect, inp: &Inputs) {
        let (sw, _) = inp.sprite;
        let g = inp.bounds;
        let go_left = self.x - w.walk_left < w.walk_right - sw - self.x;
        let (edge, out) = if go_left { (w.walk_left, w.walk_left - sw) } else { (w.walk_right - sw, w.walk_right) };
        let out = if out < g.left || out > g.right {
            if go_left {
                w.walk_right
            } else {
                w.walk_left - sw
            }
        } else {
            out
        };
        self.abort_plan();
        if g.floor - self.y > inp.sprite.1 * MAX_JUMP_UP {
            // Too high to jump: open an umbrella and drift down.
            self.plan.extend([Step::Walk { x: edge }, Step::Float { dir: if go_left { -1.0 } else { 1.0 } }]);
            return;
        }
        self.plan.extend([
            Step::Walk { x: edge },
            Step::Jump { x: out.clamp(g.left, g.right.max(g.left)), y: g.floor, land: Landing::On(Surface::Ground) },
        ]);
    }
}

/// The widest part of `span` not covered by any of `covers` (all `(left,
/// right)` ranges), e.g. the visible stretch of a window's top edge.
pub fn largest_uncovered(span: (f64, f64), covers: &[(f64, f64)]) -> Option<(f64, f64)> {
    let mut free = vec![span];
    for &(cl, cr) in covers {
        free = free
            .into_iter()
            .flat_map(|(l, r)| {
                if cr <= l || cl >= r {
                    vec![(l, r)]
                } else {
                    [(l, cl.max(l)), (cr.min(r), r)].into_iter().filter(|(a, b)| b > a).collect()
                }
            })
            .collect();
    }
    free.into_iter().filter(|(l, r)| r > l).max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
}

fn find(windows: &[WindowRect], id: u64) -> Option<&WindowRect> {
    windows.iter().find(|w| w.id == id)
}

fn standing_y(w: &WindowRect, sh: f64) -> f64 {
    w.top - sh + sh * FEET
}

/// Sprite x when clinging to a window's side (mostly outside the window).
fn wall_x(w: &WindowRect, side: Side, sw: f64) -> f64 {
    match side {
        Side::Left => w.left - sw * 0.6,
        Side::Right => w.right - sw * 0.4,
    }
}

/// Horizontal gap between two windows' walkable tops (0 if they overlap).
fn gap(a: &WindowRect, b: &WindowRect) -> f64 {
    (b.walk_left - a.walk_right).max(a.walk_left - b.walk_right).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Bounds = Bounds { left: 0.0, right: 1872.0, top: 0.0, floor: 912.0 };
    const SPRITE: (f64, f64) = (128.0, 128.0);
    const NONE: &[WindowRect] = &[];

    fn inputs(feeling: Feeling) -> Inputs<'static> {
        Inputs {
            dt: 0.05,
            bounds: B,
            feeling,
            gravity: true,
            user_away: false,
            alert: false,
            overheated: false,
            speed: 100.0,
            follow: None,
            sprite: SPRITE,
            windows: NONE,
            climbs_per_hour: 0.0,
            fun_per_hour: 0.0,
        }
    }

    fn win(id: u64, left: f64, top: f64, right: f64, bottom: f64) -> WindowRect {
        WindowRect { id, left, top, right, bottom, walk_left: left + 8.0, walk_right: right - 150.0 }
    }

    fn run(pet: &mut Pet, inp: Inputs, secs: f64, rng: &mut Rng) {
        for _ in 0..(secs / inp.dt) as usize {
            pet.tick(inp, rng);
        }
    }

    /// Run until `done` is true; returns whether it happened in time.
    fn run_until(pet: &mut Pet, inp: Inputs, secs: f64, rng: &mut Rng, mut done: impl FnMut(&Pet) -> bool) -> bool {
        for _ in 0..(secs / inp.dt) as usize {
            pet.tick(inp, rng);
            if done(pet) {
                return true;
            }
        }
        false
    }

    #[test]
    fn stays_inside_bounds() {
        let mut rng = Rng::with_seed(7);
        let mut pet = Pet::new(500.0, B.floor);
        let mut moved = false;
        for _ in 0..20_000 {
            let x = pet.x;
            pet.tick(inputs(Feeling::Happy), &mut rng);
            moved |= pet.x != x;
            assert!((B.left..=B.right).contains(&pet.x), "x = {}", pet.x);
            assert_eq!(pet.y, B.floor);
        }
        assert!(moved);
    }

    #[test]
    fn wanders_the_whole_area_without_gravity() {
        let mut rng = Rng::with_seed(9);
        let mut pet = Pet::new(500.0, 250.0);
        let inp = Inputs { gravity: false, ..inputs(Feeling::Happy) };
        let (mut min_y, mut max_y) = (f64::MAX, f64::MIN);
        for _ in 0..40_000 {
            pet.tick(inp, &mut rng);
            assert!((B.top..=B.floor).contains(&pet.y) && (B.left..=B.right).contains(&pet.x));
            min_y = min_y.min(pet.y);
            max_y = max_y.max(pet.y);
        }
        assert!(max_y - min_y > 200.0, "moved vertically: {min_y}..{max_y}");

        // Dropped mid-air: stays there.
        pet.start_drag();
        pet.drag_to(100.0, 120.0);
        pet.end_drag();
        run(&mut pet, inp, 0.2, &mut rng);
        assert_eq!(pet.y, 120.0);
    }

    #[test]
    fn falls_to_floor_after_drop() {
        let mut rng = Rng::with_seed(1);
        let mut pet = Pet::new(100.0, B.floor);
        pet.start_drag();
        pet.drag_to(300.0, 100.0);
        assert_eq!(pet.anim(), Anim::Drag);
        pet.end_drag();
        run(&mut pet, inputs(Feeling::Neutral), 1.0, &mut rng);
        assert_eq!(pet.y, B.floor);
        assert_eq!(pet.x, 300.0);
    }

    #[test]
    fn follows_the_pointer_sometimes() {
        let mut rng = Rng::with_seed(5);
        let mut pet = Pet::new(100.0, B.floor);
        let inp = Inputs { follow: Some((800.0, B.floor)), ..inputs(Feeling::Neutral) };
        let mut closest = f64::MAX;
        for _ in 0..1200 {
            pet.tick(inp, &mut rng);
            closest = closest.min((pet.x - 800.0).abs());
        }
        assert!(closest < 1.0, "closest = {closest}");
    }

    #[test]
    fn sleeps_when_user_away_and_wakes_on_return() {
        let mut rng = Rng::with_seed(2);
        let mut pet = Pet::new(100.0, B.floor);
        let away = Inputs { user_away: true, ..inputs(Feeling::Neutral) };
        run(&mut pet, away, 10.0, &mut rng);
        assert_eq!(pet.anim(), Anim::Sleep);
        pet.tick(inputs(Feeling::Neutral), &mut rng);
        assert!(!pet.is_sleeping());
    }

    #[test]
    fn alert_stops_the_pet_until_resolved() {
        let mut rng = Rng::with_seed(3);
        let mut pet = Pet::new(100.0, B.floor);
        let alert = Inputs { alert: true, ..inputs(Feeling::Neutral) };
        run(&mut pet, alert, 0.1, &mut rng);
        let x = pet.x;
        run(&mut pet, alert, 30.0, &mut rng);
        assert_eq!(pet.anim(), Anim::Alert);
        assert_eq!(pet.x, x);
        run(&mut pet, inputs(Feeling::Neutral), 0.1, &mut rng);
        assert_ne!(pet.anim(), Anim::Alert);
    }

    #[test]
    fn poke_reacts_then_returns() {
        let mut rng = Rng::with_seed(4);
        let mut pet = Pet::new(100.0, B.floor);
        pet.poke();
        assert_eq!(pet.anim(), Anim::Happy);
        run(&mut pet, inputs(Feeling::Neutral), 2.0, &mut rng);
        assert_ne!(pet.anim(), Anim::Happy);
    }

    #[test]
    fn mood_and_load_pick_idle_animation() {
        let mut rng = Rng::with_seed(6);
        let mut pet = Pet::new(100.0, B.floor);
        pet.tick(inputs(Feeling::Hungry), &mut rng);
        assert_eq!(pet.anim(), Anim::Hungry);
        pet.tick(Inputs { overheated: true, ..inputs(Feeling::Neutral) }, &mut rng);
        assert_eq!(pet.anim(), Anim::Sweat);
    }

    #[test]
    fn every_anim_falls_back_to_idle() {
        for anim in Anim::ALL {
            let mut a = anim;
            for _ in 0..5 {
                match a.fallback() {
                    Some(next) => a = next,
                    None => break,
                }
            }
            assert_eq!(a, Anim::Idle, "{anim:?}");
        }
    }

    // ---- climbing ----

    #[test]
    fn climbs_onto_a_window_and_walks_on_its_top() {
        let mut rng = Rng::with_seed(11);
        let windows = [win(1, 400.0, 300.0, 1400.0, 900.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(100.0, B.floor);
        let mut saw_climb = false;
        let on_top = run_until(&mut pet, inp, 120.0, &mut rng, |p| {
            saw_climb |= p.anim() == Anim::Climb;
            p.surface() == Surface::Window(1)
        });
        assert!(on_top && saw_climb);
        let w = windows[0];
        // Stays on the top edge, clear of the title-bar buttons.
        let mut walked = false;
        for _ in 0..600 {
            let x = pet.x;
            pet.tick(inp, &mut rng);
            if pet.surface() != Surface::Window(1) {
                break;
            }
            walked |= pet.x != x;
            let expected = standing_y(&w, 128.0);
            assert!(pet.y == expected || pet.y == expected + 128.0 * PERCH_DROP, "y = {}", pet.y);
            assert!(pet.x >= w.walk_left && pet.x + 128.0 <= w.walk_right, "x = {}", pet.x);
        }
        assert!(walked);
    }

    #[test]
    fn jumps_to_grab_a_window_that_floats_above_the_ground() {
        // Bottom 200 px above the floor: a jump reaches (within 2.5 sprite
        // heights). Now and then it takes a ladder anyway, so try a few runs.
        let windows = [win(1, 600.0, 250.0, 1500.0, B.floor + 128.0 - 200.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
        let mut jumped = 0;
        for seed in 0..8 {
            let mut rng = Rng::with_seed(100 + seed);
            let mut pet = Pet::new(100.0, B.floor);
            let mut saw_jump = false;
            assert!(run_until(&mut pet, inp, 300.0, &mut rng, |p| {
                saw_jump |= p.anim() == Anim::Jump;
                p.surface() == Surface::Window(1)
            }));
            jumped += usize::from(saw_jump);
        }
        assert!(jumped >= 4, "jumped in {jumped} of 8 runs");
    }

    #[test]
    fn out_of_reach_windows_are_climbed_with_a_ladder() {
        let mut rng = Rng::with_seed(13);
        // Bottom far above the taskbar: no jump can reach it.
        let windows = [win(1, 600.0, 100.0, 1500.0, 400.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(900.0, B.floor);
        let (mut carried, mut ladder_seen) = (false, None);
        assert!(run_until(&mut pet, inp, 300.0, &mut rng, |p| {
            carried |= p.anim() == Anim::Carry;
            if p.anim() == Anim::Climb {
                ladder_seen = p.ladder().or(ladder_seen);
            }
            p.surface() == Surface::Window(1)
        }));
        assert!(carried, "fetched and carried a ladder");
        let l = ladder_seen.expect("climbed a ladder");
        assert!(l.top < 100.0 && l.bottom == B.floor + 128.0, "{l:?}");
        assert!(pet.ladder().is_none(), "pulled the ladder up");
    }

    #[test]
    fn climbing_goes_in_pulls() {
        let mut rng = Rng::with_seed(19);
        let windows = [win(1, 400.0, 200.0, 1400.0, 1040.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(100.0, B.floor);
        assert!(run_until(&mut pet, inp, 120.0, &mut rng, |p| p.anim() == Anim::Climb));
        let mut steps = vec![];
        while pet.anim() == Anim::Climb {
            let y = pet.y;
            pet.tick(inp, &mut rng);
            steps.push(y - pet.y);
        }
        let fast = steps.iter().cloned().fold(0.0, f64::max);
        let slow = steps.iter().cloned().fold(f64::MAX, f64::min);
        assert!(fast > slow * 5.0, "pulls {fast} vs pauses {slow}");
    }

    #[test]
    fn ladder_topples_if_the_window_moves() {
        let mut rng = Rng::with_seed(20);
        let mut windows = vec![win(1, 600.0, 100.0, 1500.0, 400.0)];
        let mut pet = Pet::new(900.0, B.floor);
        {
            let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
            assert!(run_until(&mut pet, inp, 300.0, &mut rng, |p| p.anim() == Anim::Climb));
        }
        windows[0] = win(1, 700.0, 100.0, 1600.0, 400.0);
        let inp = Inputs { windows: &windows, ..inputs(Feeling::Neutral) };
        pet.tick(inp, &mut rng);
        assert!(pet.ladder().is_none());
        assert!(run_until(&mut pet, inp, 3.0, &mut rng, |p| p.y == B.floor));
    }

    #[test]
    fn floats_down_from_high_windows_with_an_umbrella() {
        let mut rng = Rng::with_seed(21);
        let windows = [win(1, 600.0, 100.0, 1500.0, 400.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 1.0, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(900.0, B.floor);
        pet.climb_timer = Some(0.0);
        assert!(run_until(&mut pet, inp, 300.0, &mut rng, |p| p.surface() == Surface::Window(1)));
        let mut floated = false;
        assert!(run_until(&mut pet, inp, 400.0, &mut rng, |p| {
            floated |= p.anim() == Anim::Float;
            p.surface() == Surface::Ground && p.y == B.floor
        }));
        assert!(floated);
    }

    #[test]
    fn rides_something_when_asked() {
        let mut rng = Rng::with_seed(22);
        let mut pet = Pet::new(900.0, B.floor);
        let inp = Inputs { fun_per_hour: 1.0, ..inputs(Feeling::Neutral) };
        // Fun is random; ask until a ride comes up.
        let mut rode = None;
        for _ in 0..20 {
            pet.fun_now();
            run_until(&mut pet, inp, 3.0, &mut rng, |p| {
                if matches!(p.anim(), Anim::RideBike | Anim::RideMoto | Anim::RideCar | Anim::RideSkate) {
                    rode = Some(p.anim());
                }
                rode.is_some()
            });
            if rode.is_some() {
                break;
            }
            run(&mut pet, inp, 40.0, &mut rng);
        }
        assert!(rode.is_some(), "rode something");
        // Faster than walking.
        let x = pet.x;
        pet.tick(inp, &mut rng);
        assert!((pet.x - x).abs() > 100.0 * 0.05 * 1.5);
        // And it ends with a poof.
        assert!(run_until(&mut pet, inp, 30.0, &mut rng, |p| p.anim() == Anim::Poof));
    }

    #[test]
    fn asking_to_climb_with_nothing_to_climb_says_so() {
        let mut rng = Rng::with_seed(23);
        let mut pet = Pet::new(900.0, B.floor);
        pet.climb_now();
        run(&mut pet, inputs(Feeling::Neutral), 0.2, &mut rng);
        assert!(pet.take_climb_failed());
        assert!(!pet.take_climb_failed());
    }

    #[test]
    fn rides_a_moving_window_and_falls_when_it_closes() {
        let mut rng = Rng::with_seed(14);
        let mut windows = vec![win(1, 400.0, 300.0, 1400.0, 900.0)];
        let mut pet = Pet::new(100.0, B.floor);
        {
            let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
            assert!(run_until(&mut pet, inp, 120.0, &mut rng, |p| p.surface() == Surface::Window(1)));
        }
        // Drag the window 100 px right and 50 px up.
        let before = pet.x;
        windows[0] = win(1, 500.0, 250.0, 1500.0, 850.0);
        {
            let inp = Inputs { windows: &windows, climbs_per_hour: 0.0, ..inputs(Feeling::Neutral) };
            pet.tick(Inputs { dt: 0.0001, ..inp }, &mut rng);
            assert!((pet.x - (before + 100.0)).abs() < 1.0, "rode along: {} -> {}", before, pet.x);
            let expected = standing_y(&windows[0], 128.0);
            assert!(pet.y == expected || pet.y == expected + 128.0 * PERCH_DROP, "y = {}", pet.y);
        }
        // Close it: falls back to the ground.
        let inp = inputs(Feeling::Neutral);
        assert!(run_until(&mut pet, inp, 3.0, &mut rng, |p| p.surface() == Surface::Ground && p.y == B.floor));
    }

    #[test]
    fn jumps_between_windows() {
        let mut rng = Rng::with_seed(15);
        let windows = [win(1, 300.0, 400.0, 900.0, 900.0), win(2, 1000.0, 350.0, 1700.0, 900.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 3600.0, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(100.0, B.floor);
        let mut visited = [false; 2];
        run_until(&mut pet, inp, 900.0, &mut rng, |p| {
            match p.surface() {
                Surface::Window(1) => visited[0] = true,
                Surface::Window(2) => visited[1] = true,
                _ => {}
            }
            visited == [true, true]
        });
        assert_eq!(visited, [true, true]);
    }

    #[test]
    fn comes_back_down_after_a_while() {
        let mut rng = Rng::with_seed(16);
        let windows = [win(1, 400.0, 300.0, 1400.0, 900.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 1.0, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(100.0, B.floor);
        pet.climb_timer = Some(0.0);
        assert!(run_until(&mut pet, inp, 120.0, &mut rng, |p| p.surface() == Surface::Window(1)));
        assert!(run_until(&mut pet, inp, 400.0, &mut rng, |p| p.surface() == Surface::Ground && p.y == B.floor));
    }

    #[test]
    fn dropped_onto_a_window_lands_on_it() {
        let mut rng = Rng::with_seed(17);
        let windows = [win(1, 400.0, 500.0, 1400.0, 900.0)];
        let inp = Inputs { windows: &windows, ..inputs(Feeling::Neutral) };
        let mut pet = Pet::new(100.0, B.floor);
        pet.start_drag();
        pet.drag_to(700.0, 100.0);
        pet.end_drag();
        run(&mut pet, inp, 1.5, &mut rng);
        assert_eq!(pet.surface(), Surface::Window(1));
        assert_eq!(pet.y, standing_y(&windows[0], 128.0));
    }

    #[test]
    fn uncovered_stretch() {
        assert_eq!(largest_uncovered((0.0, 100.0), &[]), Some((0.0, 100.0)));
        assert_eq!(largest_uncovered((0.0, 100.0), &[(20.0, 40.0), (90.0, 200.0)]), Some((40.0, 90.0)));
        assert_eq!(largest_uncovered((0.0, 100.0), &[(-10.0, 30.0)]), Some((30.0, 100.0)));
        assert_eq!(largest_uncovered((0.0, 100.0), &[(-10.0, 300.0)]), None);
    }

    #[test]
    fn never_climbs_when_turned_off() {
        let mut rng = Rng::with_seed(18);
        let windows = [win(1, 400.0, 300.0, 1400.0, 900.0)];
        let inp = Inputs { windows: &windows, climbs_per_hour: 0.0, ..inputs(Feeling::Happy) };
        let mut pet = Pet::new(100.0, B.floor);
        assert!(!run_until(&mut pet, inp, 600.0, &mut rng, |p| p.surface() != Surface::Ground));
    }
}
