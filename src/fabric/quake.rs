//! The wobble table a Medium piece stands on: a hexagonal deck hung on three
//! short ties, at alternate corners, from arms rising off a fixed hexagonal
//! base. Hung so, the deck can't tilt; it sways on its ties like a pendulum
//! and twists about its middle. A stepper motor in the middle of the base
//! turns a crank, and a cable from the crank's pin to each of the deck's
//! hooks draws the deck after the pin. The piece's feet stand on the deck
//! and go with it, and pull on it in turn.
//!
//! The stepper plays cues (`Cue`), DMX loops of the crank's position, and
//! breeds them. It watches the piece's hands (or, with none, its highest
//! joints) swing against the deck while it tries each cue, scoring it for
//! the hands swinging expressively and for how little the crank works, and
//! keeps the best of each kind of swing, unlike one another. Every so often
//! it plays what it has kept, each in turn, then breeds on.

use crate::build::dsl::brick_dsl::Side;
use crate::fabric::cue::Cue;
use crate::fabric::dimensions::Hardware;
use crate::fabric::{Fabric, JointKey};
use crate::units::{Seconds, Unit, EARTH_GRAVITY};
use crate::Age;
use glam::{Quat, Vec3};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use slotmap::SecondaryMap;
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

/// The deck's top, where the piece stands, above the floor.
pub const DECK_HEIGHT: f32 = 0.45;
pub const DECK_THICKNESS: f32 = 0.04;
pub const BASE_THICKNESS: f32 = 0.05;
/// Between the deck's underside and an arm's head.
pub const HEAD_CLEARANCE: f32 = 0.03;
/// Each tie, from the arm's head down to the hook's foot: the deck sways
/// once a second on ties this long.
pub const TIE_LENGTH: f32 = 0.25;
/// The ties hang this far out from the middle, as part of the deck's radius.
pub const TIE_RADIUS: f32 = 0.75;
/// How far to each side of its tie a hook drops and an arm rises.
pub const HOOK_REACH: f32 = 0.1;
/// The hooks and the arms stand at these corners of the hexagon: one behind
/// the piece and two before it, either side, so the table has the piece's
/// own mirror symmetry and the piece faces between two of them.
pub const HANG_CORNERS: [usize; 3] = [0, 2, 4];
/// The pin's height above the base.
pub const CAM_HEIGHT: f32 = 0.05;
/// The crank on the stepper's shaft: its pin this far from the axis.
pub const CRANK: f32 = 0.008;
/// The crank's zero: pointing forward, the way the piece faces, along its
/// mirror plane.
pub const CRANK_ZERO: f32 = TAU / 4.0;
/// The stepper's top speed, radians a second.
const TOP_SPEED: f32 = 4.0 * TAU;
/// Deck and frame, per square metre.
const DECK_DENSITY: f32 = 15.0;
/// The deck reaches this far past the farthest foot or guy anchor.
pub const DECK_MARGIN: f32 = 0.1;
/// How much of its swing the deck loses each cycle, as a damping ratio.
const DAMPING: f32 = 0.05;
/// Each cable's tension with the deck still and the crank at its zero.
/// Enough that none goes slack drawing the deck round.
const CABLE_PRETENSION: f32 = 300.0;
/// How far the hands should swing against the deck: expressive, no more.
const EXPRESSIVE: f32 = 0.04;
/// A cue is tried this long, listened to after it has had time to build.
const TRYING: Seconds = Seconds(12.0);
const SETTLING_IN: Seconds = Seconds(4.0);
/// A cue kept is played this long.
const PLAYING: Seconds = Seconds(15.0);
/// After this many tries, it plays what it has kept.
const ROUND: u32 = 8;
/// It keeps at most this many cues.
const KEPT: usize = 6;
/// How much the crank's work counts against a cue, against its hands'
/// swing: a turn every half second costs this much.
const EFFORT: f32 = 0.3;
/// Two swings less alike than this (`Quake::alike`) are of different kinds.
pub const DISTINCT: f32 = 0.6;
/// A swing's shape is followed in this many steps through its cue's loop.
const BINS: usize = 12;
/// How long the hands' swing is remembered, as it dies away, and how long
/// where they stand is averaged over.
const RESPONSE_MEMORY: Seconds = Seconds(1.0);
const STANCE_MEMORY: Seconds = Seconds(3.0);

/// Which way the hexagon's corner `index` lies, from the middle, measured
/// from +x toward +z, the way the piece faces: corners 0, 2 and 4 at 30°,
/// 150° and 270°.
pub fn corner_angle(index: usize) -> f32 {
    (index as f32 * 60.0 + 30.0).to_radians()
}

/// The y of a hook's foot, where its tie and its cable are tied.
pub fn hook_foot() -> f32 {
    -DECK_THICKNESS - HEAD_CLEARANCE - TIE_LENGTH
}

/// The stepper's axis.
pub fn cam_axis() -> Vec3 {
    Vec3::new(0.0, -DECK_HEIGHT + BASE_THICKNESS + CAM_HEIGHT, 0.0)
}

/// How far the pin is from the stepper's axis, the crank at `angle` from
/// its zero.
pub fn crank_pin(angle: f32) -> Vec3 {
    let angle = CRANK_ZERO + angle;
    Vec3::new(angle.cos(), 0.0, angle.sin()) * CRANK
}

/// Where each hook's cable is tied, at its foot's inner corner, on the deck
/// at rest.
pub fn cable_ends(radius: f32) -> [Vec3; 3] {
    HANG_CORNERS.map(|corner| {
        let angle = corner_angle(corner);
        Vec3::new(angle.cos(), 0.0, angle.sin()) * (TIE_RADIUS * radius - HOOK_REACH) + Vec3::Y * hook_foot()
    })
}

/// `angle` brought round to between a half turn back and a half turn on.
fn wrapped(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

/// A cue kept: how well it did, how far the hands swung, what they did,
/// and the shape of their swing (`Quake::shape`).
#[derive(Clone, Debug)]
pub struct Kept {
    pub cue: Cue,
    pub score: f32,
    pub swing: f32,
    pub motion: String,
    shape: Vec<f32>,
}

/// What the stepper does with a cue: try it, a child of a kept one or a
/// seed, or play one kept.
#[derive(Clone, Debug, PartialEq)]
enum Task {
    Try(Cue, Option<usize>),
    Play(usize),
}

/// A kept cue's name: A to F, by where it is kept.
fn letter(index: usize) -> char {
    (b'A' + index as u8) as char
}

/// A joint the motor watches: which hand it is on, and where it has stood
/// on the deck lately.
#[derive(Clone, Debug)]
struct Watched {
    key: JointKey,
    side: Option<Side>,
    stance: Vec3,
}

/// What it hears while a cue plays, once it has settled in: the hands'
/// swing, the crank's work, and the hands' way from where they stand
/// through the loop, `BINS` steps of it for each.
#[derive(Clone, Debug, Default)]
struct Hearing {
    ticks: u32,
    swing: f32,
    work: f32,
    shape: Vec<Vec3>,
    counts: Vec<u32>,
}

#[derive(Clone, Debug)]
struct Motor {
    since: Age,
    stopping: bool,
    /// The crank's angle from its zero, and how fast it turned last tick.
    angle: f32,
    speed: f32,
    hearing: Hearing,
}

/// Where the deck is: moved this far from where it hangs still and turned
/// this far about its middle, and how far the pin is from the stepper's
/// axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Deck {
    pub offset: Vec3,
    pub turn: f32,
    pub radius: f32,
    pub pin: Vec3,
}

impl Deck {
    /// Where a point resting on the deck at rest is now.
    fn carry(&self, point: Vec3) -> Vec3 {
        Quat::from_rotation_y(self.turn) * point + self.offset
    }

    /// Where a point is on the deck, as if it were at rest.
    fn on_deck(&self, point: Vec3) -> Vec3 {
        Quat::from_rotation_y(-self.turn) * (point - self.offset)
    }
}

#[derive(Clone, Debug)]
pub struct Quake {
    deck: Deck,
    velocity: Vec3,
    spin: f32,
    /// The deck's own mass, kg.
    mass: f32,
    /// What hangs on the ties: the deck and the piece, newtons.
    weight: f32,
    /// Each cable's rest length, and its stiffness, newtons a metre.
    cables: [f32; 3],
    cable_stiffness: f32,
    motor: Option<Motor>,
    /// What the piece's feet pulled on the deck with this tick, and the
    /// twist it gave.
    pull: Vec3,
    pull_twist: f32,
    /// The joints it watches.
    watched: Vec<Watched>,
    /// How far they have swung against the deck lately, metres.
    response: f32,
    /// Each joint standing on the deck: where on the deck, and where it was
    /// put.
    standing: SecondaryMap<JointKey, (Vec3, Vec3)>,
    /// What the stepper plays, what comes once it is back at its zero, the
    /// seeds still to try, and how many tries since it last played.
    task: Option<Task>,
    next: Option<Task>,
    seeds: VecDeque<Cue>,
    tries: u32,
    since_played: u32,
    /// The cues kept, unlike one another, each named by where it is kept.
    kept: Vec<Kept>,
    random: ChaCha8Rng,
}

impl Quake {
    /// A table `radius` across its corners, still, its crank at its zero,
    /// nothing on it yet.
    pub fn of_radius(radius: f32) -> Quake {
        let area = 1.5 * 3.0f32.sqrt() * radius * radius;
        let mass = DECK_DENSITY * area;
        let home = cam_axis() + crank_pin(0.0);
        let spans = cable_ends(radius).map(|end| end.distance(home));
        let ea = Hardware::MEDIUM.cable_stiffness.0;
        let cable_stiffness = ea / (spans.iter().sum::<f32>() / 3.0);
        Quake {
            deck: Deck { radius, pin: crank_pin(0.0), ..Deck::default() },
            velocity: Vec3::ZERO,
            spin: 0.0,
            mass,
            weight: mass * EARTH_GRAVITY.f32(),
            cables: spans.map(|span| span - CABLE_PRETENSION / cable_stiffness),
            cable_stiffness,
            motor: None,
            pull: Vec3::ZERO,
            pull_twist: 0.0,
            watched: Vec::new(),
            response: 0.0,
            standing: SecondaryMap::new(),
            task: None,
            next: None,
            seeds: Cue::seeds().into(),
            tries: 0,
            since_played: 0,
            kept: Vec::new(),
            random: ChaCha8Rng::seed_from_u64(1),
        }
    }

    /// The piece `fabric` landed on the table: its weight on the ties, and
    /// `hands` watched, or with none its three highest joints.
    pub fn mount(&mut self, fabric: &Fabric, hands: &[(JointKey, Side)]) {
        let piece = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum::<f32>() / 1000.0;
        self.weight = (self.mass + piece) * EARTH_GRAVITY.f32();
        let mut watched: Vec<(JointKey, Option<Side>)> =
            hands.iter().filter(|(key, _)| fabric.joints.contains_key(*key)).map(|&(key, side)| (key, Some(side))).collect();
        if watched.is_empty() {
            let mut highest: Vec<(JointKey, f32)> = fabric.joints.iter().map(|(key, joint)| (key, joint.location.y)).collect();
            highest.sort_by(|a, b| b.1.total_cmp(&a.1));
            watched = highest.into_iter().take(3).map(|(key, _)| (key, None)).collect();
        }
        self.watched =
            watched.into_iter().map(|(key, side)| Watched { key, side, stance: fabric.joints[key].location }).collect();
    }

    /// Whether a piece has landed on it.
    pub fn mounted(&self) -> bool {
        !self.watched.is_empty()
    }

    pub fn deck(&self) -> Deck {
        self.deck
    }

    /// How far the watched joints have swung against the deck lately.
    pub fn response(&self) -> f32 {
        self.response
    }

    /// The cues kept so far, A first.
    pub fn kept(&self) -> &[Kept] {
        &self.kept
    }

    /// How many cues it has tried.
    pub fn tries(&self) -> u32 {
        self.tries
    }

    /// The kept cue it plays, if it plays one.
    pub fn playing(&self) -> Option<usize> {
        match (&self.task, self.running()) {
            (Some(Task::Play(index)), true) => Some(*index),
            _ => None,
        }
    }

    /// What the stepper is doing, for the subtitle, briefly: the kept cue
    /// it plays, by its letter, or the one it tries, a child of a kept one
    /// (`B′`) or a seed (`?`), what the stepper is told (`Cue::describe`),
    /// and for one kept, what the hands do.
    pub fn describe(&self) -> String {
        let (Some(motor), Some(task)) = (&self.motor, &self.task) else {
            return "stopped".to_string();
        };
        let name = |task: &Task| match task {
            Task::Play(index) => letter(*index).to_string(),
            Task::Try(_, Some(parent)) => format!("{}′", letter(*parent)),
            Task::Try(_, None) => "?".to_string(),
        };
        if motor.stopping {
            return self.next.as_ref().map_or("stopping".to_string(), |next| format!("→ {}", name(next)));
        }
        match task {
            Task::Try(cue, _) => format!("{} {}", name(task), cue.describe()),
            Task::Play(index) => {
                let kept = &self.kept[*index];
                format!("{} {} · {}", name(task), kept.cue.describe(), kept.motion)
            }
        }
    }

    /// The Quake button: it breeds cues, trying each, every so often
    /// playing those it has kept, until pressed again.
    pub fn advance(&mut self, age: Age) {
        if self.running() {
            self.then(age, None);
        } else {
            self.go_on(age);
        }
    }

    /// Bring the crank back to its zero and stop.
    pub fn stop(&mut self, age: Age) {
        self.then(age, None);
    }

    /// On to what is next: the next kept cue while it plays them, or when
    /// it has tried enough, or else a seed, or a child of those kept.
    fn go_on(&mut self, age: Age) {
        let next = match &self.task {
            Some(Task::Play(index)) if index + 1 < self.kept.len() => Some(Task::Play(index + 1)),
            _ if self.seeds.is_empty() && self.since_played >= ROUND && !self.kept.is_empty() => {
                self.since_played = 0;
                Some(Task::Play(0))
            }
            _ => Some(match self.seeds.pop_front() {
                Some(seed) => Task::Try(seed, None),
                None => {
                    let (cue, parent) = self.child();
                    Task::Try(cue, parent)
                }
            }),
        };
        self.then(age, next);
    }

    /// A child of the cues kept, and which: of the better of a pair drawn,
    /// sometimes crossed with another; with none kept, of a seed.
    fn child(&mut self) -> (Cue, Option<usize>) {
        if self.kept.is_empty() {
            let seeds = Cue::seeds();
            let seed = &seeds[self.random.gen_range(0..seeds.len())];
            return (seed.mutated(None, &mut self.random), None);
        }
        let draw = |random: &mut ChaCha8Rng| {
            let (a, b) = (random.gen_range(0..self.kept.len()), random.gen_range(0..self.kept.len()));
            if self.kept[a].score >= self.kept[b].score { a } else { b }
        };
        let parent = draw(&mut self.random);
        let other = draw(&mut self.random);
        let other = (other != parent && self.random.gen_bool(0.3)).then(|| self.kept[other].cue.clone());
        (self.kept[parent].cue.mutated(other.as_ref(), &mut self.random), Some(parent))
    }

    /// Bring the crank back to its zero, then do `next`.
    fn then(&mut self, age: Age, next: Option<Task>) {
        self.next = next;
        match &mut self.motor {
            Some(motor) => motor.stopping = true,
            None => self.begin(age),
        }
    }

    fn begin(&mut self, age: Age) {
        self.task = self.next.take();
        if self.task.is_some() {
            let shape = vec![Vec3::ZERO; self.watched.len() * BINS];
            let counts = vec![0; self.watched.len() * BINS];
            let hearing = Hearing { shape, counts, ..Hearing::default() };
            self.motor = Some(Motor { since: age, stopping: false, angle: 0.0, speed: 0.0, hearing });
        }
    }

    /// The cue it plays now.
    fn cue(&self) -> Option<&Cue> {
        match self.task.as_ref()? {
            Task::Try(cue, _) => Some(cue),
            Task::Play(index) => Some(&self.kept[*index].cue),
        }
    }

    /// Done with the cue in hand: a cue tried is scored, and kept if the
    /// hands swung and no kept cue's swing is like it, or in place of one
    /// whose is but did less well, or of the worst when there are enough.
    fn done(&mut self, age: Age) {
        let (Some(Task::Try(cue, _)), Some(motor)) = (self.task.clone(), &self.motor) else {
            self.go_on(age);
            return;
        };
        self.tries += 1;
        self.since_played += 1;
        let hearing = &motor.hearing;
        let ticks = hearing.ticks.max(1) as f32;
        let swing = hearing.swing / ticks;
        let work = hearing.work / ticks / (2.0 * TAU);
        let expressive = swing / EXPRESSIVE;
        let expressive = if expressive <= 1.0 { expressive } else { (2.0 - expressive).max(0.0) };
        let score = expressive - EFFORT * work;
        let shape: Vec<f32> = hearing
            .shape
            .iter()
            .zip(&hearing.counts)
            .flat_map(|(sum, &count)| (*sum / count.max(1) as f32).to_array())
            .collect();
        let found = Kept { motion: self.motion(&shape), cue, score, swing, shape };
        if swing > EXPRESSIVE / 2.0 {
            match self.kept.iter().position(|kept| Self::alike(&kept.shape, &found.shape) >= DISTINCT) {
                Some(index) if found.score > self.kept[index].score => self.kept[index] = found,
                Some(_) => {}
                None if self.kept.len() < KEPT => self.kept.push(found),
                None => {
                    let worst = (0..self.kept.len()).min_by(|&a, &b| self.kept[a].score.total_cmp(&self.kept[b].score));
                    if let Some(worst) = worst.filter(|&worst| found.score > self.kept[worst].score) {
                        self.kept[worst] = found;
                    }
                }
            }
        }
        self.go_on(age);
    }

    /// How alike two swings are, whatever their size and wherever in the
    /// loop they start: one for the same, zero for nothing alike.
    pub fn alike(a: &[f32], b: &[f32]) -> f32 {
        let joints = a.len() / (3 * BINS);
        let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
        let (na, nb) = (norm(a), norm(b));
        (0..BINS)
            .map(|shift| {
                let mut dot = 0.0;
                for joint in 0..joints {
                    for bin in 0..BINS {
                        let (i, j) = (joint * BINS + bin, joint * BINS + (bin + shift) % BINS);
                        for axis in 0..3 {
                            dot += a[i * 3 + axis] * b[j * 3 + axis];
                        }
                    }
                }
                dot.abs() / (na * nb)
            })
            .fold(0.0, f32::max)
    }

    /// What the hands do in a swing's `shape`, in words: which way they go
    /// most, and whether together or against each other.
    fn motion(&self, shape: &[f32]) -> String {
        let way = |joint: usize, bin: usize| Vec3::from_slice(&shape[(joint * BINS + bin) * 3..][..3]);
        let mut spread = Vec3::ZERO;
        for joint in 0..self.watched.len() {
            for bin in 0..BINS {
                spread += way(joint, bin).abs();
            }
        }
        let axis = if spread.x >= spread.y && spread.x >= spread.z { 0 } else if spread.z >= spread.y { 2 } else { 1 };
        let hand = |side: Side, bin: usize| {
            let joints: Vec<usize> = (0..self.watched.len()).filter(|&j| self.watched[j].side == Some(side)).collect();
            joints.iter().map(|&joint| way(joint, bin)[axis]).sum::<f32>() / joints.len().max(1) as f32
        };
        let together: f32 = (0..BINS).map(|bin| hand(Side::Left, bin) * hand(Side::Right, bin)).sum();
        let handed = self.watched.iter().any(|watched| watched.side.is_some());
        match (axis, !handed || together >= 0.0) {
            (0, true) => "swaying side to side",
            (0, false) => "opening and closing",
            (2, true) => "rocking back and forth",
            (2, false) => "swinging back and forth in turn",
            (_, true) => "bobbing up and down",
            (_, false) => "waving up and down in turn",
        }
        .to_string()
    }

    /// Whether the stepper is playing, not stopping.
    pub fn running(&self) -> bool {
        self.motor.as_ref().is_some_and(|motor| !motor.stopping)
    }

    /// Whether the stepper is playing, or bringing its crank back to zero.
    pub fn busy(&self) -> bool {
        self.motor.is_some()
    }

    /// Whether the deck is still going, with the motor or after it.
    pub fn moving(&self) -> bool {
        self.motor.is_some() || self.velocity.length() > 1e-3 || self.spin.abs() > 1e-3
    }

    /// The stepper this tick: after its cue's position, or back to its zero,
    /// as fast as it can go; and done with the cue once it has had its time.
    fn turn_crank(&mut self, age: Age, dt: f32) {
        let Some(motor) = &self.motor else {
            return;
        };
        let elapsed = age.elapsed_since(motor.since).0;
        let target = match (motor.stopping, self.cue()) {
            (false, Some(cue)) => Cue::angle(cue.value(elapsed)),
            _ => 0.0,
        };
        let motor = self.motor.as_mut().unwrap();
        let toward = wrapped(target - motor.angle);
        let turned = toward.clamp(-TOP_SPEED * dt, TOP_SPEED * dt);
        motor.angle += turned;
        motor.speed = turned / dt;
        self.deck.pin = crank_pin(motor.angle);
        if motor.stopping {
            if wrapped(motor.angle).abs() < 1e-4 {
                self.motor = None;
                self.deck.pin = crank_pin(0.0);
                self.begin(age);
            }
            return;
        }
        let lasts = match self.task {
            Some(Task::Try(..)) => TRYING.0,
            _ => PLAYING.0,
        };
        if elapsed >= lasts {
            self.done(age);
        }
    }

    /// One tick of the deck: the stepper, the ties pulling it back, the
    /// cables, damping, and what the piece's feet pulled with.
    fn step(&mut self, age: Age, dt: f32) {
        self.turn_crank(age, dt);
        let stiffness = self.weight / TIE_LENGTH;
        let tie_radius = TIE_RADIUS * self.deck.radius;
        let twist_stiffness = stiffness * tie_radius * tie_radius;
        let inertia = self.mass * 5.0 / 12.0 * self.deck.radius * self.deck.radius;
        let held = stiffness + 1.5 * self.cable_stiffness;
        let sway_damping = 2.0 * DAMPING * (held * self.mass).sqrt();
        let twist_damping = 2.0 * DAMPING * (twist_stiffness * inertia).sqrt();

        let mut force = self.pull - self.deck.offset * stiffness - self.velocity * sway_damping;
        let mut torque = self.pull_twist - self.deck.turn * twist_stiffness - self.spin * twist_damping;
        let pin = cam_axis() + self.deck.pin;
        for (end, rest) in cable_ends(self.deck.radius).into_iter().zip(self.cables) {
            let at = self.deck.carry(end);
            let toward = pin - at;
            let tension = (self.cable_stiffness * (toward.length() - rest)).max(0.0);
            let pull = Vec3::new(toward.x, 0.0, toward.z) / toward.length() * tension;
            let arm = at - self.deck.offset;
            force += pull;
            torque += arm.z * pull.x - arm.x * pull.z;
        }
        self.velocity += force / self.mass * dt;
        self.velocity.y = 0.0;
        self.spin += torque / inertia * dt;
        self.deck.offset += self.velocity * dt;
        self.deck.turn += self.spin * dt;
    }

    /// How far the watched joints swing about where they have stood on the
    /// deck lately, the farthest, remembered as it dies away; and while a
    /// cue plays, once settled in, what it hears.
    fn watch(&mut self, fabric_joints: impl Fn(JointKey) -> Option<Vec3>, age: Age, dt: f32) {
        let follow = dt / STANCE_MEMORY.0;
        let mut swing = 0.0f32;
        let mut aways = Vec::with_capacity(self.watched.len());
        for watched in &mut self.watched {
            let away = fabric_joints(watched.key).map_or(Vec3::ZERO, |at| self.deck.on_deck(at) - watched.stance);
            swing = swing.max(away.length());
            watched.stance += away * follow;
            aways.push(away);
        }
        self.response = swing.max(self.response * (-dt / RESPONSE_MEMORY.0).exp());
        let loop_seconds = self.cue().map(Cue::seconds);
        let response = self.response;
        let Some(motor) = &mut self.motor else {
            return;
        };
        let elapsed = age.elapsed_since(motor.since).0;
        let (Some(loop_seconds), false, true) = (loop_seconds, motor.stopping, elapsed >= SETTLING_IN.0) else {
            return;
        };
        let hearing = &mut motor.hearing;
        hearing.ticks += 1;
        hearing.swing += response;
        hearing.work += motor.speed.abs();
        let bin = ((elapsed / loop_seconds).fract() * BINS as f32) as usize % BINS;
        for (joint, away) in aways.into_iter().enumerate() {
            hearing.shape[joint * BINS + bin] += away;
            hearing.counts[joint * BINS + bin] += 1;
        }
    }
}

impl Fabric {
    /// The deck under a Medium piece, where it is now.
    pub fn deck(&self) -> Option<Deck> {
        self.quake.as_ref().map(Quake::deck)
    }

    /// Move every joint standing on the deck, within `contact` of it, as
    /// the deck moves in this tick. Each is put where it stands on the deck,
    /// remembered while nothing else moves it, so no rounding builds up.
    pub(crate) fn apply_quake(&mut self, contact: f32) {
        let Some(quake) = &mut self.quake else {
            return;
        };
        let before = quake.deck;
        quake.step(self.age, Age::iteration_duration().f32());
        let after = quake.deck;
        for (key, joint) in self.joints.iter_mut() {
            if joint.location.y > contact {
                quake.standing.remove(key);
                continue;
            }
            let place = match quake.standing.get(key) {
                Some(&(place, left)) if left.x == joint.location.x && left.z == joint.location.z => place,
                _ => before.on_deck(joint.location),
            };
            let moved = after.carry(place);
            (joint.location.x, joint.location.z) = (moved.x, moved.z);
            quake.standing.insert(key, (place, joint.location));
        }
    }

    /// What the joints standing on the deck pull on it with, as the
    /// intervals pull on them, for the deck's next tick.
    pub(crate) fn feel_quake(&mut self, contact: f32) {
        let Some(quake) = &mut self.quake else {
            return;
        };
        let middle = quake.deck.offset;
        let (mut pull, mut twist) = (Vec3::ZERO, 0.0);
        for joint in self.joints.values().filter(|joint| joint.location.y <= contact) {
            let force = Vec3::new(joint.force.x, 0.0, joint.force.z) / 1000.0;
            let arm = joint.location - middle;
            pull += force;
            twist += arm.z * force.x - arm.x * force.z;
        }
        (quake.pull, quake.pull_twist) = (pull, twist);
        let joints = &self.joints;
        quake.watch(|key| joints.get(key).map(|joint| joint.location), self.age, Age::iteration_duration().f32());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::fabric::dimensions::Size;
    use crate::fabric::physics::presets::VIEWING;
    use crate::fabric::physics::Physics;

    fn medium(name: FabricName) -> (Fabric, Physics) {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(name).at_size(Size::Medium));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..VIEWING };
        (executor.fabric, physics)
    }

    /// A Medium Headless Hug on its wobble table, Quake pressed once: it
    /// tries its seeds, keeps cues unlike one another that swing the hands,
    /// then plays them in turn. Pressed again the crank goes back to its
    /// zero, and the piece stands where it stood.
    #[test]
    fn medium_headless_hug_rides_the_wobble_table() {
        let (mut fabric, physics) = medium(FabricName::HeadlessHug);
        assert_eq!(fabric.quake.as_ref().map(|quake| quake.watched.len()), Some(6), "it watches both hands");
        fabric.take_motion();
        let before: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
        let per_second = Age::iterations_per_second() as usize;
        fabric.quake.as_mut().unwrap().advance(fabric.age);
        let mut ticks = 0;
        while fabric.quake.as_ref().unwrap().playing().is_none() {
            fabric.iterate(&physics);
            ticks += 1;
            assert!(ticks < 600 * per_second, "it played what it kept within ten minutes");
        }
        let quake = fabric.quake.as_ref().unwrap();
        for kept in quake.kept() {
            eprintln!("{} scored {:.2}, hands {:.0} mm, {}", kept.cue.describe(), kept.score, kept.swing * 1000.0, kept.motion);
        }
        assert!(quake.kept().len() > 1, "it kept {} cues", quake.kept().len());
        for (index, a) in quake.kept().iter().enumerate() {
            for b in &quake.kept()[index + 1..] {
                assert!(Quake::alike(&a.shape, &b.shape) < DISTINCT, "two kept cues swing alike");
            }
        }
        for _ in 0..20 * per_second {
            fabric.iterate(&physics);
        }
        fabric.quake.as_mut().unwrap().advance(fabric.age);
        assert!(!fabric.quake.as_ref().unwrap().running(), "pressed again, it stops");
        for _ in 0..30 * per_second {
            fabric.iterate(&physics);
        }
        assert!(fabric.joints.values().all(|joint| joint.location.is_finite()), "the figure diverged");
        let after = fabric.joints.values().zip(&before).map(|(joint, b)| joint.location.distance(*b));
        let settled = after.fold(0.0, f32::max);
        let motion = fabric.take_motion().expect("motion was measured");
        eprintln!(
            "{settled:.3} m off after, cable max {:.0} N, strut max {:.0} N, slack {}",
            motion.max_cable_force.0, motion.max_strut_force.0, motion.slack_cable_count
        );
        assert!(settled < 0.03, "it ended {:.0} mm from where it stood", settled * 1000.0);
    }

    /// Breed cues for a Medium piece (`PROBE` names the fabric, Headless
    /// Hug by default; `TRIES` how many, 100 by default) and print those
    /// kept as DMX loops, for the electronics.
    #[test]
    #[ignore]
    fn breed_wobble_cues() {
        let name: FabricName = std::env::var("PROBE").ok().and_then(|name| name.parse().ok()).unwrap_or(FabricName::HeadlessHug);
        let tries: u32 = std::env::var("TRIES").ok().and_then(|tries| tries.parse().ok()).unwrap_or(100);
        let (mut fabric, physics) = medium(name);
        fabric.quake.as_mut().unwrap().advance(fabric.age);
        let mut reported = 0;
        while fabric.quake.as_ref().unwrap().tries() < tries {
            fabric.iterate(&physics);
            let quake = fabric.quake.as_ref().unwrap();
            if quake.tries() != reported {
                reported = quake.tries();
                let best: Vec<String> = quake.kept().iter().map(|kept| format!("{:.2}", kept.score)).collect();
                eprintln!("try {reported}: kept {}", best.join(" "));
            }
        }
        for (index, kept) in fabric.quake.as_ref().unwrap().kept().iter().enumerate() {
            eprintln!(
                "// {} {}: scored {:.2}, hands {:.0} mm, {}\n{:?}",
                letter(index),
                kept.cue.describe(),
                kept.score,
                kept.swing * 1000.0,
                kept.motion,
                kept.cue.frames
            );
        }
    }
}
