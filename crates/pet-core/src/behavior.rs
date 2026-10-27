//! What the pet is doing on screen: walking, sitting, sleeping, reacting...
//!
//! Coordinates are physical screen pixels of the sprite's top-left corner.

use crate::mood::Feeling;
use fastrand::Rng;
use serde::{Deserialize, Serialize};

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
}

impl Anim {
    pub const ALL: [Anim; 10] = [
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
        }
    }

    /// Which animation to use when a pack does not define this one.
    pub fn fallback(self) -> Option<Anim> {
        match self {
            Anim::Idle => None,
            Anim::Walk | Anim::Sit | Anim::Happy | Anim::Sad => Some(Anim::Idle),
            Anim::Sleep | Anim::Bored => Some(Anim::Sit),
            Anim::Surprised | Anim::Alert => Some(Anim::Happy),
            Anim::Drag => Some(Anim::Surprised),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Facing {
    Left,
    Right,
}

/// The strip of screen the pet may stand on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Smallest allowed sprite x.
    pub left: f64,
    /// Largest allowed sprite x.
    pub right: f64,
    /// Sprite y when standing on the floor (usually just above the taskbar).
    pub floor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Activity {
    Idle,
    Walk,
    Sit,
    Sleep,
    React(Anim),
    Alert,
    Dragged,
    Falling { vy: f64 },
}

#[derive(Debug, Clone, Copy)]
pub struct Inputs {
    pub dt: f64,
    pub bounds: Bounds,
    pub feeling: Feeling,
    /// The user has been away from keyboard and mouse long enough to nap.
    pub user_away: bool,
    /// A reminder is waiting for the user.
    pub alert: bool,
    /// Walking speed in physical pixels per second.
    pub speed: f64,
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
}

const GRAVITY: f64 = 2400.0;

impl Pet {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, facing: Facing::Right, activity: Activity::Idle, timer: 2.0, feeling: Feeling::Neutral }
    }

    pub fn anim(&self) -> Anim {
        match self.activity {
            Activity::Idle => match self.feeling {
                Feeling::Sad => Anim::Sad,
                Feeling::Bored => Anim::Bored,
                Feeling::Happy => Anim::Happy,
                Feeling::Neutral | Feeling::Sleepy => Anim::Idle,
            },
            Activity::Walk => Anim::Walk,
            Activity::Sit => match self.feeling {
                Feeling::Bored => Anim::Bored,
                _ => Anim::Sit,
            },
            Activity::Sleep => Anim::Sleep,
            Activity::React(a) => a,
            Activity::Alert => Anim::Alert,
            Activity::Dragged | Activity::Falling { .. } => Anim::Drag,
        }
    }

    pub fn is_sleeping(&self) -> bool {
        self.activity == Activity::Sleep
    }

    pub fn is_dragged(&self) -> bool {
        self.activity == Activity::Dragged
    }

    /// The user clicked the pet.
    pub fn poke(&mut self) {
        let anim = if self.is_sleeping() { Anim::Surprised } else { Anim::Happy };
        self.react(anim, 1.5);
    }

    pub fn react(&mut self, anim: Anim, secs: f64) {
        if !matches!(self.activity, Activity::Dragged | Activity::Falling { .. }) {
            self.activity = Activity::React(anim);
            self.timer = secs;
        }
    }

    pub fn start_drag(&mut self) {
        self.activity = Activity::Dragged;
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

    pub fn tick(&mut self, inp: Inputs, rng: &mut Rng) {
        self.feeling = inp.feeling;
        let b = inp.bounds;

        match self.activity {
            Activity::Dragged => return,
            Activity::Falling { vy } => {
                let vy = vy + GRAVITY * inp.dt;
                self.y += vy * inp.dt;
                if self.y >= b.floor {
                    self.y = b.floor;
                    self.activity = Activity::React(Anim::Surprised);
                    self.timer = 0.6;
                } else {
                    self.activity = Activity::Falling { vy };
                }
                return;
            }
            _ => {}
        }

        // Dropped somewhere above the floor (or the taskbar moved): fall.
        if self.y < b.floor - 1.0 {
            self.activity = Activity::Falling { vy: 0.0 };
            return;
        }
        self.y = b.floor;
        self.x = self.x.clamp(b.left, b.right.max(b.left));

        if inp.alert {
            if !matches!(self.activity, Activity::React(_)) {
                self.activity = Activity::Alert;
            }
        } else if self.activity == Activity::Alert {
            self.activity = Activity::Idle;
            self.timer = 1.0;
        }

        if self.activity == Activity::Walk {
            let dir = if self.facing == Facing::Right { 1.0 } else { -1.0 };
            self.x += dir * inp.speed * inp.dt;
            if self.x <= b.left {
                self.x = b.left;
                self.facing = Facing::Right;
            } else if self.x >= b.right {
                self.x = b.right;
                self.facing = Facing::Left;
            }
        }

        // Wake up as soon as the user is back, if not actually tired.
        if self.is_sleeping() && !inp.user_away && inp.feeling != Feeling::Sleepy {
            self.timer = self.timer.min(0.0);
        }

        self.timer -= inp.dt;
        if self.timer <= 0.0 && self.activity != Activity::Alert {
            self.choose_next(inp, rng);
        }
    }

    fn choose_next(&mut self, inp: Inputs, rng: &mut Rng) {
        if inp.user_away || inp.feeling == Feeling::Sleepy {
            self.activity = Activity::Sleep;
            self.timer = 20.0 + rng.f64() * 40.0;
            return;
        }

        // Weights for idle, walk, sit, hop.
        let (idle, walk, sit, hop) = match inp.feeling {
            Feeling::Happy => (25, 50, 10, 15),
            Feeling::Neutral => (40, 45, 15, 0),
            Feeling::Bored => (30, 25, 45, 0),
            Feeling::Sad => (60, 15, 25, 0),
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
            self.activity = Activity::Walk;
            if rng.bool() {
                self.facing = if self.facing == Facing::Left { Facing::Right } else { Facing::Left };
            }
            self.timer = 2.0 + rng.f64() * 6.0;
        } else if pick(sit, &mut roll) {
            self.activity = Activity::Sit;
            self.timer = 4.0 + rng.f64() * 8.0;
        } else {
            self.activity = Activity::React(Anim::Happy);
            self.timer = 1.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Bounds = Bounds { left: 0.0, right: 1000.0, floor: 500.0 };

    fn inputs(feeling: Feeling) -> Inputs {
        Inputs { dt: 0.05, bounds: B, feeling, user_away: false, alert: false, speed: 100.0 }
    }

    fn run(pet: &mut Pet, inp: Inputs, secs: f64, rng: &mut Rng) {
        for _ in 0..(secs / inp.dt) as usize {
            pet.tick(inp, rng);
        }
    }

    #[test]
    fn stays_inside_bounds() {
        let mut rng = Rng::with_seed(7);
        let mut pet = Pet::new(500.0, 500.0);
        for _ in 0..20_000 {
            pet.tick(inputs(Feeling::Happy), &mut rng);
            assert!((B.left..=B.right).contains(&pet.x), "x = {}", pet.x);
            assert_eq!(pet.y, B.floor);
        }
    }

    #[test]
    fn falls_to_floor_after_drop() {
        let mut rng = Rng::with_seed(1);
        let mut pet = Pet::new(100.0, 500.0);
        pet.start_drag();
        pet.drag_to(300.0, 100.0);
        assert_eq!(pet.anim(), Anim::Drag);
        pet.end_drag();
        run(&mut pet, inputs(Feeling::Neutral), 1.0, &mut rng);
        assert_eq!(pet.y, B.floor);
        assert_eq!(pet.x, 300.0);
    }

    #[test]
    fn sleeps_when_user_away_and_wakes_on_return() {
        let mut rng = Rng::with_seed(2);
        let mut pet = Pet::new(100.0, 500.0);
        let away = Inputs { user_away: true, ..inputs(Feeling::Neutral) };
        run(&mut pet, away, 10.0, &mut rng);
        assert_eq!(pet.anim(), Anim::Sleep);
        pet.tick(inputs(Feeling::Neutral), &mut rng);
        assert!(!pet.is_sleeping());
    }

    #[test]
    fn alert_stops_the_pet_until_resolved() {
        let mut rng = Rng::with_seed(3);
        let mut pet = Pet::new(100.0, 500.0);
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
        let mut pet = Pet::new(100.0, 500.0);
        pet.poke();
        assert_eq!(pet.anim(), Anim::Happy);
        run(&mut pet, inputs(Feeling::Neutral), 2.0, &mut rng);
        assert_ne!(pet.anim(), Anim::Happy);
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
}
