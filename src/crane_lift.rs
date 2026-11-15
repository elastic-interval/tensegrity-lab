//! Headless Hug on a crane hook: which joints to put a lifting eye on.
//!
//! Two lifts are tried, each from several sets of eyes, on equal sling legs
//! from one hook that the crane keeps straight above them:
//!
//! - the figure without its arms lying on its belly, its shoulders raised
//!   until it stands on its feet;
//! - the whole figure hanging free, to be carried and set down.
//!
//! The ground and the hook are the building site's (`fabric/site.rs`).
//! Nothing here runs in a build. Results are in `docs/crane-lift.md`.

use crate::build::dsl::fabric_library::{self, FabricName};
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::connector::strut::{tube_set, HEADLESS_HUG_TUBES, INNER_BORE_RADIUS, INNER_RADIUS, OUTER_BORE_RADIUS, OUTER_RADIUS};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::Physics;
use crate::fabric::site::{self, Hook, Site};
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Newtons, Seconds, Unit};
use crate::Age;
use glam::{Mat4, Vec3};
use slotmap::SecondaryMap;
use std::f32::consts::{FRAC_PI_2, PI};

pub(crate) const FLOOR: f32 = 0.0;
/// A 6 mm cable's 25.5 kN breaking load over the factor of 5 usual for wire rope.
pub(crate) const CABLE_WORKING_LOAD: Newtons = Newtons(5.1e3);
/// Aluminium's modulus, in pascals.
const TUBE_MODULUS: f32 = 69e9;
pub(crate) const HOOK_SPEED: f32 = 0.25;
/// Forces are averaged over this long, which removes the fast axial ringing
/// only the simulation has.
pub(crate) const AVERAGED: Seconds = Seconds(0.02);

pub(crate) fn ticks(seconds: Seconds) -> usize {
    (seconds / Age::iteration_duration()) as usize
}

/// Headless Hug as built, standing on the frozen surface with its guy lines.
pub(crate) fn standing() -> (Fabric, Physics) {
    let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::HeadlessHug));
    while !executor.is_complete() {
        let _ = executor.iterate();
    }
    // Heavier drag than viewing's: a hanging figure swings for a long time.
    let physics = Physics { surface: executor.physics.surface, drag: 3.0, ..VIEWING };
    (executor.fabric, physics)
}

pub(crate) fn joint(fabric: &Fabric, name: &str) -> JointKey {
    fabric.joint_key_by_label(name).unwrap_or_else(|| panic!("no joint {name}"))
}

pub(crate) fn member_name(fabric: &Fabric, interval: &Interval) -> String {
    format!("{}–{}", fabric.joint_label(interval.alpha_key), fabric.joint_label(interval.omega_key))
}

/// Take away the guy lines and their ground anchors, and stand the figure
/// on the floor.
pub(crate) fn onto_floor(fabric: &mut Fabric) {
    let anchors: Vec<JointKey> = fabric
        .intervals
        .values()
        .filter(|interval| interval.has_role(Role::GuyLine))
        .flat_map(|interval| [interval.alpha_key, interval.omega_key])
        .filter(|&end| fabric.push_at(end).is_none())
        .collect();
    for anchor in anchors {
        fabric.remove_joint(anchor);
    }
    fabric.apply_translation(Vec3::new(0.0, FLOOR, 0.0));
}

/// Take both arms off: every joint built beyond the shoulder hubs, and the
/// node plates hanging from those.
fn remove_arms(fabric: &mut Fabric) {
    let arm = |key: JointKey| {
        let path = fabric.joints[key].path.to_string();
        path.starts_with("CX1ACX") || path.starts_with("DX1ACX")
    };
    let mut gone: Vec<JointKey> = fabric.joints.keys().filter(|&key| arm(key)).collect();
    let plates: Vec<JointKey> = fabric
        .joints
        .keys()
        .filter(|node| fabric.bridle_nodes.get(node).is_some_and(|&link| arm(fabric.intervals[link].other_joint(*node))))
        .collect();
    gone.extend(plates);
    for key in gone {
        fabric.bridle_nodes.remove(&key);
        fabric.remove_joint(key);
    }
}

/// Lay the figure face down, just above the floor.
pub(crate) fn lay_on_belly(fabric: &mut Fabric) {
    fabric.apply_matrix4(Mat4::from_rotation_x(FRAC_PI_2));
    let lowest = fabric.altitude_range().0;
    fabric.apply_translation(Vec3::new(0.0, FLOOR - lowest + 0.01, 0.0));
}

/// The hook, and the eyes its sling legs go to.
pub(crate) struct Rig {
    pub(crate) hook: JointKey,
    pub(crate) eyes: Vec<JointKey>,
    pub(crate) height: f32,
    /// Where the crane holds the hook, when not straight above the eyes.
    pub(crate) place: Option<Vec3>,
}

impl Rig {
    /// Sling legs `leg` long from one hook to the joints named, the hook
    /// straight above them where the first leg comes taut.
    pub(crate) fn new(fabric: &mut Fabric, eyes: &[&str], leg: Meters) -> Rig {
        let eyes: Vec<JointKey> = eyes.iter().map(|name| joint(fabric, name)).collect();
        let above = Self::above(fabric, &eyes);
        let height = eyes
            .iter()
            .map(|&eye| {
                let at = fabric.location(eye);
                let aside = Vec3::new(at.x - above.x, 0.0, at.z - above.z).length();
                at.y + (leg.f32() * leg.f32() - aside * aside).max(0.0).sqrt()
            })
            .fold(f32::MAX, f32::min);
        Rig::at(fabric, eyes, leg, height, None)
    }

    /// The same, with the hook at `height`, held at `place` if given.
    pub(crate) fn at(fabric: &mut Fabric, eyes: Vec<JointKey>, leg: Meters, height: f32, place: Option<Vec3>) -> Rig {
        let above = place.unwrap_or_else(|| Self::above(fabric, &eyes));
        let hook = fabric.create_joint(Vec3::new(above.x, height, above.z));
        for &eye in &eyes {
            fabric.create_fixed_interval(eye, hook, Role::GuyLine, leg);
        }
        Rig { hook, eyes, height, place }
    }

    pub(crate) fn above(fabric: &Fabric, eyes: &[JointKey]) -> Vec3 {
        eyes.iter().map(|&eye| fabric.location(eye)).sum::<Vec3>() / eyes.len() as f32
    }

    /// How near a sling leg's centre line comes to the centre line of a
    /// member that doesn't end at its eye, and which member.
    fn clearance(&self, fabric: &Fabric) -> (Meters, String) {
        let mut nearest = (Meters(f32::MAX), String::new());
        for leg in fabric.intervals.values().filter(|interval| interval.touches(self.hook)) {
            let eye = leg.other_joint(self.hook);
            for member in fabric.intervals.values().filter(|member| !member.touches(eye) && !member.touches(self.hook)) {
                let apart = segment_distance(leg.locations(&fabric.joints), member.locations(&fabric.joints));
                if apart < nearest.0.f32() {
                    nearest = (Meters(apart), member_name(fabric, member));
                }
            }
        }
        nearest
    }

    pub(crate) fn eye_height(&self, fabric: &Fabric) -> f32 {
        Self::above(fabric, &self.eyes).y - FLOOR
    }
}

/// The shortest distance between two line segments.
fn segment_distance((p0, p1): (Vec3, Vec3), (q0, q1): (Vec3, Vec3)) -> f32 {
    let (d1, d2, r) = (p1 - p0, q1 - q0, p0 - q0);
    let (a, b, c, d, e) = (d1.dot(d1), d1.dot(d2), d2.dot(d2), d1.dot(r), d2.dot(r));
    let denominator = a * c - b * b;
    let mut s = if denominator > 1e-9 { ((b * e - c * d) / denominator).clamp(0.0, 1.0) } else { 0.0 };
    let t = ((b * s + e) / c).clamp(0.0, 1.0);
    s = ((b * t - d) / a).clamp(0.0, 1.0);
    (p0 + d1 * s).distance(q0 + d2 * t)
}

/// What the figure rests on.
#[derive(Clone, Copy, PartialEq)]
enum Ground {
    /// The builds' frozen surface, which never lets go.
    Surface,
    /// The floor holding what presses on it where it is, as chocked feet are.
    Gripping,
}

/// Every member's force in kilonewtons, tension in a cable and compression
/// in a strut, averaged over `AVERAGED`.
pub(crate) type Forces = SecondaryMap<IntervalKey, f32>;

/// The forces of the figure standing as built, on the frozen surface.
pub(crate) fn standing_forces(fabric: &mut Fabric, physics: &Physics) -> Forces {
    advance(fabric, physics, None, Seconds(0.1), 0.0, Ground::Surface)
}

/// Run on the floor for `seconds`, the hook rising at `climb` metres per
/// second, and give the forces at the end.
pub(crate) fn run(fabric: &mut Fabric, physics: &Physics, rig: Option<&mut Rig>, seconds: Seconds, climb: f32) -> Forces {
    advance(fabric, physics, rig, seconds, climb, Ground::Gripping)
}

fn advance(fabric: &mut Fabric, physics: &Physics, mut rig: Option<&mut Rig>, seconds: Seconds, climb: f32, ground: Ground) -> Forces {
    fabric.unfreeze();
    fabric.site = (ground != Ground::Surface).then(|| Site {
        ground: site::Ground::Gripping,
        hook: rig.as_deref().map(|rig| Hook { joint: rig.hook, eyes: rig.eyes.clone(), height: rig.height, place: rig.place, climb }),
        anchors: Vec::new(),
    });
    let (total, averaged) = (ticks(seconds), ticks(AVERAGED));
    let mut strains: SecondaryMap<IntervalKey, f32> = fabric.intervals.keys().map(|key| (key, 0.0)).collect();
    for tick in 0..total {
        fabric.iterate(physics);
        if tick + averaged >= total {
            for (key, interval) in fabric.intervals.iter() {
                strains[key] += interval.strain;
            }
        }
    }
    if let (Some(rig), Some(hook)) = (rig.as_deref_mut(), fabric.site.take().and_then(|site| site.hook)) {
        rig.height = hook.height;
    }
    assert!(!fabric.frozen || fabric.max_velocity() < 0.1, "the figure flew apart");
    forces_of(fabric, physics, &strains, averaged)
}

/// The forces of members whose strains were summed over `samples` ticks.
pub(crate) fn forces_of(fabric: &Fabric, physics: &Physics, strains: &SecondaryMap<IntervalKey, f32>, samples: usize) -> Forces {
    fabric
        .intervals
        .iter()
        .map(|(key, interval)| {
            let ideal = interval.ideal();
            let stiffness = fabric.dimensions.spring_constant(interval.material, ideal, physics) * interval.stiffness.as_factor();
            let force = Newtons::from_millinewtons(stiffness * (strains[key] / samples as f32) * ideal.f32());
            (key, force.to_kilonewtons().abs())
        })
        .collect()
}

/// Raise the hook until the eyes are `height` above the floor (or the hook
/// has risen 12 m), then hold it for the figure to come to rest.
pub(crate) fn raise(fabric: &mut Fabric, physics: &Physics, rig: &mut Rig, height: Meters) -> Forces {
    let mut risen = 0.0;
    while rig.eye_height(fabric) < height.f32() && risen < 12.0 {
        run(fabric, physics, Some(rig), Seconds(0.2), HOOK_SPEED);
        risen += 0.2 * HOOK_SPEED;
    }
    run(fabric, physics, Some(rig), Seconds(10.0), 0.0)
}

/// A strut's ideal (Euler) buckling load as the telescoped tube it is,
/// pinned at both ends: the inner tube alone at its ends and the outer
/// between, the overlap counted as the outer alone.
pub(crate) fn buckling_load(length: Meters) -> Newtons {
    let second_moment = |outer: Meters, bore: Meters| PI / 4.0 * (outer.f32().powi(4) - bore.f32().powi(4));
    let (inner, outer) = (
        TUBE_MODULUS * second_moment(INNER_RADIUS, INNER_BORE_RADIUS),
        TUBE_MODULUS * second_moment(OUTER_RADIUS, OUTER_BORE_RADIUS),
    );
    let (set, _, _) = tube_set(HEADLESS_HUG_TUBES, length);
    let half_outer = set.outer.f32().min(length.f32()) / 2.0;
    let end = (length.f32() / 2.0 - half_outer).max(1e-4);
    // Symmetric bowing: tan(k₁·end) · tan(k₂·half_outer) = k₁/k₂, k = √(P/EI).
    let lacking = |load: f32| {
        let (k1, k2) = ((load / inner).sqrt(), (load / outer).sqrt());
        (k1 * end).tan() * (k2 * half_outer).tan() - k1 / k2
    };
    let quarter_wave = |stiffness: f32, span: f32| stiffness * (FRAC_PI_2 / span).powi(2);
    let (mut low, mut high) = (0.0, quarter_wave(inner, end).min(quarter_wave(outer, half_outer)) * 0.999_99);
    for _ in 0..60 {
        let middle = (low + high) / 2.0;
        if lacking(middle) < 0.0 {
            low = middle;
        } else {
            high = middle;
        }
    }
    Newtons(low)
}

/// How far the figure is pitched forward from the way it stands, in
/// degrees (90 is face down), and rolled to its right.
pub(crate) fn attitude(fabric: &Fabric, standing_pitch: f32) -> (f32, f32) {
    let middle = |a: &str, b: &str| (fabric.location(joint(fabric, a)) + fabric.location(joint(fabric, b))) / 2.0;
    let side = (fabric.location(joint(fabric, "R54")) - fabric.location(joint(fabric, "L54"))).normalize();
    let axis = middle("L25", "R25") - middle("L54", "R54");
    let up = (axis - side * axis.dot(side)).normalize();
    let front = side.cross(up);
    ((-front.y).atan2(up.y).to_degrees() - standing_pitch, (-side.y).asin().to_degrees())
}

pub(crate) fn is_cable(interval: &Interval) -> bool {
    interval.role.is_pull_like() && !interval.has_role(Role::GuyLine)
}

/// One line of numbers for a station of a lift, then the members that
/// stand out, against the forces `before`.
pub(crate) fn report(station: &str, fabric: &Fabric, rig: Option<&Rig>, forces: &Forces, before: &Forces, standing_pitch: f32) {
    let (pitch, roll) = attitude(fabric, standing_pitch);
    let mut cables: Vec<(f32, IntervalKey)> =
        fabric.intervals.iter().filter(|(_, interval)| is_cable(interval)).map(|(key, _)| (forces[key], key)).collect();
    cables.sort_by(|a, b| b.0.total_cmp(&a.0));
    let over = cables.iter().filter(|(force, _)| *force > CABLE_WORKING_LOAD.to_kilonewtons()).count();
    let slack = cables.iter().filter(|(force, _)| *force < 0.001).count();
    let mut struts: Vec<(f32, IntervalKey)> = fabric
        .intervals
        .iter()
        .filter(|(_, interval)| interval.has_role(Role::Pushing))
        .map(|(key, interval)| (buckling_load(interval.ideal()).to_kilonewtons() / forces[key].max(1e-3), key))
        .collect();
    struts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let grounded: Vec<String> = fabric
        .joints
        .iter()
        .filter(|(_, joint)| (joint.location.y - FLOOR).abs() <= 0.002)
        .map(|(key, _)| fabric.joint_label(key))
        .collect();
    let legs: Vec<String> = rig
        .iter()
        .flat_map(|rig| fabric.intervals.iter().filter(|(_, interval)| interval.touches(rig.hook)))
        .map(|(key, interval)| {
            let eye = interval.other_joint(rig.unwrap().hook);
            let strut = &fabric.intervals[fabric.push_at(eye).expect("an eye is on a strut end")];
            let outward = fabric.location(eye) - fabric.location(strut.other_joint(eye));
            let along = fabric.location(rig.unwrap().hook) - fabric.location(eye);
            format!("{} {:.2} at {:.0}° to its strut", fabric.joint_label(eye), forces[key], outward.angle_between(along).to_degrees())
        })
        .collect();
    eprintln!(
        "  {station:<18} pitch {pitch:>5.1}° roll {roll:>4.1}°  cable max {:.1} kN, {over} over {:.1}, {slack} slack  strut margin {:.2}",
        cables[0].0,
        CABLE_WORKING_LOAD.to_kilonewtons(),
        struts[0].0
    );
    if let Some(rig) = rig {
        let (clearance, nearest) = rig.clearance(fabric);
        eprintln!("      legs (kN): {}; nearest member {nearest}, {:.0} mm from a leg", legs.join(", "), clearance.to_mm());
    }
    eprintln!("      on the floor: {}", if grounded.is_empty() { "nothing".to_string() } else { grounded.join(" ") });
    let was = |key: IntervalKey| before.get(key).copied().unwrap_or(0.0);
    let cable_list: Vec<String> = cables
        .iter()
        .take(4)
        .map(|&(force, key)| format!("{} {force:.1} (was {:.1})", member_name(fabric, &fabric.intervals[key]), was(key)))
        .collect();
    eprintln!("      cables: {}", cable_list.join(", "));
    let strut_list: Vec<String> = struts
        .iter()
        .take(3)
        .map(|&(margin, key)| {
            let interval = &fabric.intervals[key];
            format!("{} {:.1} kN of {:.1} = {margin:.2} (was {:.1} kN)", member_name(fabric, interval), forces[key], buckling_load(interval.ideal()).to_kilonewtons(), was(key))
        })
        .collect();
    eprintln!("      struts: {}", strut_list.join(", "));
}

pub(crate) fn mass_and_centre(fabric: &Fabric) -> (f32, Vec3) {
    let mass: f32 = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum();
    let centre = fabric.joints.values().map(|joint| joint.location * joint.accumulated_mass.f32()).sum::<Vec3>() / mass;
    (mass / 1000.0, centre)
}

/// Pitch the standing figure forward about the line between its feet.
fn pitch_forward(fabric: &mut Fabric, degrees: f32) {
    let at = |name: &str| fabric.location(fabric.joint_key_by_label(name).expect("a foot"));
    let feet = (at("L54") + at("R54")) / 2.0;
    fabric.apply_matrix4(
        Mat4::from_translation(feet) * Mat4::from_rotation_x(degrees.to_radians()) * Mat4::from_translation(-feet),
    );
}

/// The telescoped strut's buckling load, against `docs/parts.md`: 6.6 kN
/// for Z1–Z2 at 2.40 m, 7.4 kN for L5–L6 at 2.28 m.
#[test]
fn buckling_loads_match_the_parts_document() {
    assert!((buckling_load(Meters(2.40)).to_kilonewtons() - 6.6).abs() < 0.1);
    assert!((buckling_load(Meters(2.28)).to_kilonewtons() - 7.4).abs() < 0.1);
}

/// The whole figure hanging free from each set of eyes: how it pitches, and
/// what its members carry against standing.
///
/// ```
/// cargo test --release --lib hang_headless_hug -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn hang_headless_hug() {
    let (mut stood, physics) = standing();
    let standing_pitch = attitude(&stood, 0.0).0;
    let before = standing_forces(&mut stood, &physics);
    let (mass, centre) = mass_and_centre(&stood);
    eprintln!("standing: {mass:.0} kg, centre of gravity x {:.2} y {:.2} z {:.2}", centre.x, centre.y, centre.z);
    report("standing", &stood, None, &before, &before, standing_pitch);
    onto_floor(&mut stood);
    for (eyes, leg) in HANGS {
        let mut fabric = stood.clone();
        let mut rig = Rig::new(&mut fabric, eyes, Meters(*leg));
        let height = rig.eye_height(&fabric) + 1.5;
        eprintln!("hanging from {} on {leg} m legs", eyes.join(" "));
        let forces = raise(&mut fabric, &physics, &mut rig, Meters(height));
        report("hanging", &fabric, Some(&rig), &forces, &before, standing_pitch);
    }
}

const HANGS: &[(&[&str], f32)] = &[
    (&["L25", "R25"], 4.0),
    (&["L27", "R27"], 4.0),
    (&["L17", "R17"], 4.0),
    (&["L24", "R24"], 4.0),
    (&["L20", "R20"], 4.0),
    (&["Z4"], 4.0),
    (&["L25", "R25", "L35", "R35"], 4.0),
    (&["L17", "R17", "L35", "R35"], 4.0),
];

/// The figure without its arms, face down on the floor, then raised by each
/// set of eyes at its shoulders until it stands on its feet.
///
/// ```
/// cargo test --release --lib raise_headless_hug_shoulders -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn raise_headless_hug_shoulders() {
    let (mut stood, physics) = standing();
    let standing_pitch = attitude(&stood, 0.0).0;
    let before = standing_forces(&mut stood, &physics);
    onto_floor(&mut stood);
    remove_arms(&mut stood);
    let mut upright = stood.clone();
    let armless = run(&mut upright, &physics, None, Seconds(1.0), 0.0);
    let (mass, _) = mass_and_centre(&upright);
    eprintln!("without arms: {mass:.0} kg");
    report("armless, upright", &upright, None, &armless, &before, standing_pitch);
    lay_on_belly(&mut stood);
    let lying = run(&mut stood, &physics, None, Seconds(15.0), 0.0);
    report("lying", &stood, None, &lying, &before, standing_pitch);
    for (eyes, leg) in RAISES {
        let mut fabric = stood.clone();
        let mut rig = Rig::new(&mut fabric, eyes, Meters(*leg));
        eprintln!("raised by {} on {leg} m legs", eyes.join(" "));
        for height in [1.5, 3.0, 4.5, 6.0, 7.5] {
            let forces = raise(&mut fabric, &physics, &mut rig, Meters(height));
            report(&format!("eyes at {:.1} m", rig.eye_height(&fabric)), &fabric, Some(&rig), &forces, &before, standing_pitch);
        }
    }
}

const RAISES: &[(&[&str], f32)] = &[
    (&["L25", "R25"], 4.0),
    (&["L17", "R17"], 4.0),
    (&["L27", "R27"], 4.0),
    (&["L19", "R19"], 4.0),
];

/// The whole figure held by each set of eyes, pitched forward on its feet
/// as it is when the arms have just been built, then raised until it
/// stands and on until it hangs clear of the floor.
///
/// ```
/// cargo test --release --lib stand_headless_hug_up -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn stand_headless_hug_up() {
    let (mut stood, physics) = standing();
    let standing_pitch = attitude(&stood, 0.0).0;
    let before = standing_forces(&mut stood, &physics);
    onto_floor(&mut stood);
    let upright = stood.clone();
    pitch_forward(&mut stood, 60.0);
    for (eyes, leg) in RAISES {
        let mut fabric = stood.clone();
        let mut rig = Rig::new(&mut fabric, eyes, Meters(*leg));
        eprintln!("stood up by {} on {leg} m legs", eyes.join(" "));
        for height in [0.0, 6.0, 7.0, 10.0] {
            let forces = raise(&mut fabric, &physics, &mut rig, Meters(height));
            report(&format!("eyes at {:.1} m", rig.eye_height(&fabric)), &fabric, Some(&rig), &forces, &before, standing_pitch);
            if height == 7.0 {
                bring_upright(fabric.clone(), &physics, &rig, &upright, eyes, Meters(*leg), &before, standing_pitch);
            }
        }
    }
}

/// On its feet and leaning on the hook, the figure can't be stood up by a
/// hook straight above its eyes: it would lift off first. Here the crane
/// takes the hook back, over 30 s, to where the legs just come taut on the
/// figure standing upright.
#[allow(clippy::too_many_arguments)]
fn bring_upright(mut fabric: Fabric, physics: &Physics, rig: &Rig, upright: &Fabric, eyes: &[&str], leg: Meters, before: &Forces, standing_pitch: f32) {
    let target = Rig::new(&mut upright.clone(), eyes, leg);
    let to = Vec3::new(Rig::above(upright, &target.eyes).x, target.height, Rig::above(upright, &target.eyes).z);
    let from = fabric.location(rig.hook);
    let mut rig = Rig { hook: rig.hook, eyes: rig.eyes.clone(), height: from.y, place: Some(from) };
    const STEPS: usize = 150;
    for step in 1..=STEPS {
        let at = from.lerp(to, step as f32 / STEPS as f32);
        (rig.height, rig.place) = (at.y, Some(at));
        run(&mut fabric, physics, Some(&mut rig), Seconds(0.2), 0.0);
    }
    let forces = run(&mut fabric, physics, Some(&mut rig), Seconds(10.0), 0.0);
    report("hook brought back", &fabric, Some(&rig), &forces, before, standing_pitch);
}

/// Standing joints from the top down, for choosing eyes.
#[test]
#[ignore]
fn list_headless_hug_joints() {
    let (fabric, _) = standing();
    let mut rows: Vec<(f32, String)> = Vec::new();
    for (key, joint) in fabric.joints.iter() {
        let struts: Vec<String> = fabric
            .intervals
            .values()
            .filter(|interval| interval.role == Role::Pushing && interval.touches(key))
            .map(|interval| fabric.joint_label(interval.other_joint(key)))
            .collect();
        let cables = fabric.intervals.values().filter(|interval| interval.role != Role::Pushing && interval.touches(key)).count();
        let at = joint.location;
        rows.push((
            at.y,
            format!(
                "{:<5} height {:>6.3}  side {:>6.3}  forward {:>6.3}  {:<12} strut to {:<8} {cables} cables",
                fabric.joint_label(key),
                at.y,
                at.x,
                at.z,
                joint.path.to_string(),
                struts.join(",")
            ),
        ));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, row) in rows {
        eprintln!("{row}");
    }
}
