//! The wobble table a Medium piece stands on: a hexagonal deck hung on three
//! short ties, at alternate corners, from arms rising off a fixed hexagonal
//! base. Hung so, the deck can't tilt; it sways on its ties like a pendulum
//! and twists about its middle. A stepper motor in the middle of the base
//! turns a cam whose throw can be set as it runs, and a cable from the
//! cam's pin to each of the deck's hooks draws the deck round after the pin.
//! The piece's feet stand on the deck and go with it, and pull on it in turn.
//!
//! The motor listens to the piece. It watches its hands (or, with none, its
//! highest joints) move against the deck, first sweeping slowly through its
//! speeds to find where they answer most, its resonances, then holding the
//! strongest, keeping on its peak, with the cam thrown only just far enough
//! for the hands to move expressively.

use crate::fabric::dimensions::Hardware;
use crate::fabric::{Fabric, JointKey};
use crate::units::{Seconds, Unit, EARTH_GRAVITY};
use crate::Age;
use glam::{Quat, Vec3};
use slotmap::SecondaryMap;
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
/// The hooks and the arms stand at these corners of the hexagon.
pub const HANG_CORNERS: [usize; 3] = [0, 2, 4];
/// The pin's height above the base.
pub const CAM_HEIGHT: f32 = 0.05;
/// Deck and frame, per square metre.
const DECK_DENSITY: f32 = 15.0;
/// The deck reaches this far past the farthest foot or guy anchor.
const DECK_MARGIN: f32 = 0.1;
/// How much of its swing the deck loses each cycle, as a damping ratio.
const DAMPING: f32 = 0.05;
/// Each cam cable's tension with the deck still and the pin home.
/// Enough that none goes slack drawing the deck round at the fastest.
const CABLE_PRETENSION: f32 = 300.0;
/// How far the hands should swing against the deck: expressive, no more.
const EXPRESSIVE: f32 = 0.04;
/// The cam's throw while it searches, and the most and least it is thrown.
const SEARCH_THROW: f32 = 0.005;
const THROW_RANGE: (f32, f32) = (0.001, 0.04);
/// The speeds searched, turns a second, slowly up from the one to the
/// other in this time.
const SEARCH: (f32, f32) = (0.3, 3.0);
const SEARCH_TIME: Seconds = Seconds(40.0);
/// How quickly the throw follows what the hands do, per second.
const THROW_ADAPT: f32 = 0.3;
/// Holding, how long it watches a speed before trying the next, and how
/// far it first steps, as a part of the speed.
const DWELL: Seconds = Seconds(4.0);
const FIRST_STEP: f32 = 0.02;
/// How long the hands' swing is remembered, as it dies away, and how long
/// where they stand is averaged over.
const RESPONSE_MEMORY: Seconds = Seconds(1.0);
const STANCE_MEMORY: Seconds = Seconds(3.0);
/// The motor starting and stopping.
const SWELL: Seconds = Seconds(2.0);

/// The y of a hook's foot, where its tie and its cam cable are tied.
pub fn hook_foot() -> f32 {
    -DECK_THICKNESS - HEAD_CLEARANCE - TIE_LENGTH
}

/// Where the cam's pin is, turned to `angle`, thrown `throw` from its axis.
pub fn cam_pin(angle: f32, throw: f32) -> Vec3 {
    Vec3::new(throw * angle.cos(), -DECK_HEIGHT + BASE_THICKNESS + CAM_HEIGHT, throw * angle.sin())
}

/// Where each hook's cam cable is tied, at its foot's inner corner, on the
/// deck at rest.
pub fn cable_ends(radius: f32) -> [Vec3; 3] {
    HANG_CORNERS.map(|corner| {
        let angle = corner as f32 * PI / 3.0;
        Vec3::new(angle.cos(), 0.0, angle.sin()) * (TIE_RADIUS * radius - HOOK_REACH) + Vec3::Y * hook_foot()
    })
}

/// A speed at which the watched joints answer strongly: their swing per
/// metre of the cam's throw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resonance {
    pub hertz: f32,
    pub gain: f32,
}

#[derive(Clone, Debug)]
enum Tuning {
    /// Sweeping up through the speeds, noting each one's gain.
    Searching { samples: Vec<(f32, f32)> },
    /// On a resonance, stepping the speed toward where the gain is
    /// highest, a dwell at a time.
    Holding { step: f32, dwell_since: f32, gains: f32, count: u32, last: Option<f32> },
}

#[derive(Clone, Debug)]
struct Motor {
    since: Age,
    stopping: Option<(Age, f32)>,
    hertz: f32,
    throw: f32,
    tuning: Tuning,
}

/// Where the deck is: moved this far from where it hangs still and turned
/// this far about its middle, and where the cam is turned and how far
/// thrown.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Deck {
    pub offset: Vec3,
    pub turn: f32,
    pub radius: f32,
    pub cam: f32,
    pub throw: f32,
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
    /// Each cam cable's rest length, and its stiffness, newtons a metre.
    cables: [f32; 3],
    cable_stiffness: f32,
    motor: Option<Motor>,
    /// What the piece's feet pulled on the deck with this tick, and the
    /// twist it gave.
    pull: Vec3,
    pull_twist: f32,
    /// The joints it watches, each where it has stood on the deck lately,
    /// on average: slow drift is not swinging.
    watched: Vec<(JointKey, Vec3)>,
    /// How far they have swung against the deck lately, metres.
    response: f32,
    /// Each joint standing on the deck: where on the deck, and where it was
    /// put.
    standing: SecondaryMap<JointKey, (Vec3, Vec3)>,
    /// What the last search found, strongest first.
    resonances: Vec<Resonance>,
}

impl Quake {
    /// The table under `fabric`, just big enough for its feet and guy
    /// anchors, still, its pin home, watching `hands`, or with none the
    /// fabric's three highest joints.
    pub fn under(fabric: &Fabric, hands: &[JointKey]) -> Quake {
        let reach = fabric
            .joints
            .values()
            .filter(|joint| joint.location.y < 0.01)
            .map(|joint| Vec3::new(joint.location.x, 0.0, joint.location.z).length())
            .fold(0.0, f32::max);
        let radius = reach + DECK_MARGIN;
        let area = 1.5 * 3.0f32.sqrt() * radius * radius;
        let mass = DECK_DENSITY * area;
        let piece = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum::<f32>() / 1000.0;
        let home = cam_pin(0.0, 0.0);
        let spans = cable_ends(radius).map(|end| end.distance(home));
        let ea = Hardware::MEDIUM.cable_stiffness.0;
        let cable_stiffness = ea / (spans.iter().sum::<f32>() / 3.0);
        let mut watched: Vec<JointKey> = hands.iter().copied().filter(|&key| fabric.joints.contains_key(key)).collect();
        if watched.is_empty() {
            let mut highest: Vec<(JointKey, f32)> = fabric.joints.iter().map(|(key, joint)| (key, joint.location.y)).collect();
            highest.sort_by(|a, b| b.1.total_cmp(&a.1));
            watched = highest.into_iter().take(3).map(|(key, _)| key).collect();
        }
        Quake {
            deck: Deck { radius, ..Deck::default() },
            velocity: Vec3::ZERO,
            spin: 0.0,
            mass,
            weight: (mass + piece) * EARTH_GRAVITY.f32(),
            cables: spans.map(|span| span - CABLE_PRETENSION / cable_stiffness),
            cable_stiffness,
            motor: None,
            pull: Vec3::ZERO,
            pull_twist: 0.0,
            watched: watched.into_iter().map(|key| (key, fabric.joints[key].location)).collect(),
            response: 0.0,
            standing: SecondaryMap::new(),
            resonances: Vec::new(),
        }
    }

    pub fn deck(&self) -> Deck {
        self.deck
    }

    /// How far the watched joints have swung against the deck lately.
    pub fn response(&self) -> f32 {
        self.response
    }

    /// What the last search found, strongest first.
    pub fn resonances(&self) -> &[Resonance] {
        &self.resonances
    }

    /// The speed the motor turns at, turns a second, if it runs.
    pub fn hertz(&self) -> Option<f32> {
        self.motor.as_ref().map(|motor| motor.hertz)
    }

    /// Whether it is holding a resonance it found.
    pub fn holding(&self) -> bool {
        self.motor.as_ref().is_some_and(|motor| matches!(motor.tuning, Tuning::Holding { .. }))
    }

    /// What the motor is doing, for the title.
    pub fn describe(&self) -> String {
        let hands = self.response * 1000.0;
        match &self.motor {
            None => "Quake stopped".to_string(),
            Some(Motor { stopping: Some(_), .. }) => "Quake stopping".to_string(),
            Some(Motor { hertz, throw, tuning: Tuning::Searching { .. }, .. }) => {
                format!("Searching at {hertz:.2} Hz, cam {:.0} mm, hands {hands:.0} mm", throw * 1000.0)
            }
            Some(Motor { hertz, throw, tuning: Tuning::Holding { .. }, .. }) => {
                let others: Vec<String> =
                    self.resonances.iter().skip(1).map(|resonance| format!("{:.2}", resonance.hertz)).collect();
                let also = if others.is_empty() { String::new() } else { format!(" (also {} Hz)", others.join(", ")) };
                format!("Resonance at {hertz:.2} Hz{also}, cam {:.0} mm, hands {hands:.0} mm", throw * 1000.0)
            }
        }
    }

    /// Whether the motor is running, or starting.
    pub fn running(&self) -> bool {
        self.motor.as_ref().is_some_and(|motor| motor.stopping.is_none())
    }

    /// Whether the deck is still going, with the motor or after it.
    pub fn moving(&self) -> bool {
        self.motor.is_some() || self.velocity.length() > 1e-3 || self.spin.abs() > 1e-3
    }

    /// Start the motor, searching anew.
    pub fn start(&mut self, age: Age) {
        if !self.running() {
            self.motor = Some(Motor {
                since: age,
                stopping: None,
                hertz: SEARCH.0,
                throw: 0.0,
                tuning: Tuning::Searching { samples: Vec::new() },
            });
        }
    }

    /// Bring the pin home and stop.
    pub fn stop(&mut self, age: Age) {
        if let Some(motor) = &mut self.motor {
            let throw = motor.throw;
            motor.stopping.get_or_insert((age, throw));
        }
    }

    /// The motor's speed and throw this tick: searching, holding, or
    /// stopping. Returns false once stopped.
    fn tune(motor: &mut Motor, resonances: &mut Vec<Resonance>, response: f32, age: Age, dt: f32) -> bool {
        let elapsed = age.elapsed_since(motor.since).0;
        if let Some((stopping, throw)) = motor.stopping {
            let left = 1.0 - age.elapsed_since(stopping).0 / SWELL.0;
            motor.throw = throw * left.max(0.0);
            return left > 0.0;
        }
        let gain = response / motor.throw.max(1e-4);
        match &mut motor.tuning {
            Tuning::Searching { samples } => {
                let along = elapsed / SEARCH_TIME.0;
                motor.hertz = SEARCH.0 * (SEARCH.1 / SEARCH.0).powf(along.min(1.0));
                motor.throw = SEARCH_THROW * (elapsed / SWELL.0).min(1.0);
                if elapsed > SWELL.0 && samples.last().is_none_or(|&(hertz, _)| motor.hertz > hertz * 1.01) {
                    samples.push((motor.hertz, gain));
                }
                if along >= 1.0 {
                    *resonances = peaks(samples);
                    motor.hertz = resonances.first().map_or(SEARCH.0, |resonance| resonance.hertz);
                    motor.tuning =
                        Tuning::Holding { step: FIRST_STEP, dwell_since: elapsed, gains: 0.0, count: 0, last: None };
                }
            }
            Tuning::Holding { step, dwell_since, gains, count, last } => {
                *gains += gain;
                *count += 1;
                if elapsed - *dwell_since >= DWELL.0 {
                    let mean = *gains / *count as f32;
                    if last.is_some_and(|last| mean < last) {
                        *step = -*step / 2.0;
                    }
                    *last = Some(mean);
                    motor.hertz *= 1.0 + *step;
                    (*dwell_since, *gains, *count) = (elapsed, 0.0, 0);
                }
                let wanted = (EXPRESSIVE / response.max(1e-4)).ln();
                let throw = motor.throw * (THROW_ADAPT * dt * wanted).exp();
                motor.throw = throw.clamp(THROW_RANGE.0, THROW_RANGE.1);
            }
        }
        true
    }

    /// One tick of the deck: the motor, the ties pulling it back, the cam's
    /// cables, damping, and what the piece's feet pulled with.
    fn step(&mut self, age: Age, dt: f32) {
        if let Some(motor) = &mut self.motor {
            if Self::tune(motor, &mut self.resonances, self.response, age, dt) {
                self.deck.cam = (self.deck.cam + TAU * motor.hertz * dt) % TAU;
                self.deck.throw = motor.throw;
            } else {
                self.deck.throw = 0.0;
                self.motor = None;
            }
        }
        let stiffness = self.weight / TIE_LENGTH;
        let tie_radius = TIE_RADIUS * self.deck.radius;
        let twist_stiffness = stiffness * tie_radius * tie_radius;
        let inertia = self.mass * 5.0 / 12.0 * self.deck.radius * self.deck.radius;
        let held = stiffness + 1.5 * self.cable_stiffness;
        let sway_damping = 2.0 * DAMPING * (held * self.mass).sqrt();
        let twist_damping = 2.0 * DAMPING * (twist_stiffness * inertia).sqrt();

        let mut force = self.pull - self.deck.offset * stiffness - self.velocity * sway_damping;
        let mut torque = self.pull_twist - self.deck.turn * twist_stiffness - self.spin * twist_damping;
        let pin = cam_pin(self.deck.cam, self.deck.throw);
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
    /// deck lately, the farthest, remembered as it dies away.
    fn watch(&mut self, fabric_joints: impl Fn(JointKey) -> Option<Vec3>, dt: f32) {
        let follow = dt / STANCE_MEMORY.0;
        let mut swing = 0.0f32;
        for (key, stance) in &mut self.watched {
            if let Some(at) = fabric_joints(*key) {
                let on_deck = self.deck.on_deck(at);
                swing = swing.max(on_deck.distance(*stance));
                *stance += (on_deck - *stance) * follow;
            }
        }
        self.response = swing.max(self.response * (-dt / RESPONSE_MEMORY.0).exp());
    }
}

/// The resonances in a search's samples: the speeds whose gain, smoothed,
/// is higher than its neighbours', strongest first, none within a tenth of
/// the speed of a stronger one, none weaker than a third of the strongest.
fn peaks(samples: &[(f32, f32)]) -> Vec<Resonance> {
    const SPREAD: usize = 4;
    let smooth: Vec<f32> = (0..samples.len())
        .map(|index| {
            let window = &samples[index.saturating_sub(SPREAD)..(index + SPREAD + 1).min(samples.len())];
            window.iter().map(|&(_, gain)| gain).sum::<f32>() / window.len() as f32
        })
        .collect();
    let mut found: Vec<Resonance> = (1..smooth.len().saturating_sub(1))
        .filter(|&index| smooth[index] > smooth[index - 1] && smooth[index] >= smooth[index + 1])
        .map(|index| Resonance { hertz: samples[index].0, gain: smooth[index] })
        .collect();
    found.sort_by(|a, b| b.gain.total_cmp(&a.gain));
    let strongest = found.first().map_or(0.0, |resonance| resonance.gain);
    let mut kept: Vec<Resonance> = Vec::new();
    for resonance in found {
        let near = kept.iter().any(|stronger| (resonance.hertz / stronger.hertz - 1.0).abs() < 0.1);
        if !near && resonance.gain >= strongest / 3.0 {
            kept.push(resonance);
        }
    }
    kept
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
        quake.watch(|key| joints.get(key).map(|joint| joint.location), Age::iteration_duration().f32());
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

    /// A Medium Headless Hug on its wobble table: the motor searches its
    /// speeds, finds a resonance at the hands, holds it with the cam thrown
    /// only just enough for the hands to swing expressively, and once it
    /// stops, the piece stands where it stood.
    #[test]
    fn medium_headless_hug_rides_the_wobble_table() {
        let plan = fabric_library::get_fabric_plan(FabricName::HeadlessHug).at_size(Size::Medium);
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..VIEWING };
        let mut fabric = executor.fabric;
        assert_eq!(fabric.quake.as_ref().map(|quake| quake.watched.len()), Some(6), "it watches both hands");
        fabric.take_motion();
        let before: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
        fabric.quake.as_mut().unwrap().start(fabric.age);
        let per_second = Age::iterations_per_second() as usize;
        let mut held = None;
        for tick in 0..120 * per_second {
            if tick == 90 * per_second {
                let quake = fabric.quake.as_ref().unwrap();
                held = Some((quake.holding(), quake.hertz().unwrap(), quake.deck().throw, quake.response()));
                eprintln!("{}", quake.describe());
                fabric.quake.as_mut().unwrap().stop(fabric.age);
            }
            if tick % (10 * per_second) == 0 {
                eprintln!("{:3} s: {}", tick / per_second, fabric.quake.as_ref().unwrap().describe());
            }
            fabric.iterate(&physics);
        }
        assert!(fabric.joints.values().all(|joint| joint.location.is_finite()), "the figure diverged");
        let after = fabric.joints.values().zip(&before).map(|(joint, b)| joint.location.distance(*b));
        let settled = after.fold(0.0, f32::max);
        let motion = fabric.take_motion().expect("motion was measured");
        let quake = fabric.quake.as_ref().unwrap();
        let (holding, hertz, throw, response) = held.unwrap();
        eprintln!(
            "resonances {:?}; held {hertz:.2} Hz, cam {:.1} mm, hands {:.0} mm; {:.0} mm off after, cable max {:.0} N, strut max {:.0} N, slack {}",
            quake.resonances(),
            throw * 1000.0,
            response * 1000.0,
            settled * 1000.0,
            motion.max_cable_force.0,
            motion.max_strut_force.0,
            motion.slack_cable_count
        );
        assert!(holding && !quake.resonances().is_empty(), "it found a resonance and held it");
        assert!((0.5..1.5).contains(&(response / EXPRESSIVE)), "the hands swung {:.0} mm", response * 1000.0);
        assert!(throw < THROW_RANGE.1, "the cam was thrown as far as it goes");
        assert!(settled < 0.03, "it ended {:.0} mm from where it stood", settled * 1000.0);
    }
}
