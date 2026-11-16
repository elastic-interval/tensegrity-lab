//! Headless Hug and Open Claw taken apart strut by strut and put together
//! again (`build/assembly.rs`), with every step's worst cable and strut.
//! Results are in `docs/crane-lift.md`.

use crate::build::assembly::Assembly;
use crate::build::dsl::fabric_library::{self, FabricName};
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::connector::parts::placements;
use crate::crane_lift::{
    attitude, joint, buckling_load, forces_of, is_cable, mass_and_centre, member_name, report, standing, standing_forces, ticks, Forces,
    Rig, AVERAGED, CABLE_WORKING_LOAD,
};
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use slotmap::SecondaryMap;

/// A joint this low is on the ground; looked at every so many ticks.
const ON_THE_GROUND: f32 = 0.01;
const DRAG_SAMPLED: usize = 20;
/// In seconds.
const TICK: f32 = 50e-6;
/// In metres per second: a joint moving faster than this has been jolted.
const SHOCK: f32 = 2.0;

/// One line for a step: what went in or came out, and the members that
/// carry most.
fn line(step: &str, fabric: &Fabric, forces: &Forces) {
    let cable = fabric
        .intervals
        .iter()
        .filter(|(_, interval)| is_cable(interval))
        .map(|(key, interval)| (forces[key], member_name(fabric, interval)))
        .max_by(|a, b| a.0.total_cmp(&b.0));
    let strut = fabric
        .intervals
        .iter()
        .filter(|(_, interval)| interval.has_role(Role::Pushing))
        .map(|(key, interval)| (buckling_load(interval.ideal()).to_kilonewtons() / forces[key].max(1e-3), member_name(fabric, interval)))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let cables = fabric.intervals.values().filter(|interval| is_cable(interval)).count();
    let (mass, _) = mass_and_centre(fabric);
    let (cable_force, cable_name) = cable.unwrap_or((0.0, "none".to_string()));
    let (margin, strut_name) = strut.unwrap_or((f32::MAX, "none".to_string()));
    let warning = if cable_force > CABLE_WORKING_LOAD.to_kilonewtons() || margin < 1.5 { "  <<<" } else { "" };
    eprintln!(
        "  {step:<22} {cables:>3} cables {mass:>4.0} kg  cable max {cable_force:>4.1} kN {cable_name:<10} strut margin {:>5.1} {strut_name}{warning}",
        margin.min(99.9)
    );
}

/// Run what the assembly has to do, telling each step's forces if `told`:
/// with how the figure is pitched if `standing_pitch` is given, which only
/// Headless Hug has. Gives the forces at the end of the last step.
fn follow(
    assembly: &mut Assembly,
    fabric: &mut Fabric,
    physics: &Physics,
    before: &Forces,
    standing_pitch: Option<f32>,
    told: bool,
) -> Forces {
    let mut strains: SecondaryMap<IntervalKey, f32> = SecondaryMap::new();
    let mut samples = 0;
    let mut forces = Forces::new();
    // Where each joint was, and how far it has slid over the ground this step.
    let mut dragged: SecondaryMap<JointKey, (glam::Vec3, f32)> = SecondaryMap::new();
    let mut tick = 0usize;
    // The fastest any joint moved this step, and which.
    let mut fastest = (0.0f32, String::new(), 0usize);
    let mut step_tick = 0usize;
    let mut was: SecondaryMap<JointKey, glam::Vec3> = SecondaryMap::new();
    while !assembly.is_idle() {
        let ending = assembly.remaining_ticks().is_some_and(|left| left <= ticks(AVERAGED));
        let ended = assembly.iterate(fabric, physics);
        tick += 1;
        step_tick += 1;
        // How fast each joint is seen to move: how far it went this tick.
        let seen = fabric
            .joints
            .iter()
            .filter_map(|(key, joint)| was.get(key).map(|&before| (key, joint.location.distance(before) / TICK)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        was = fabric.joints.iter().map(|(key, joint)| (key, joint.location)).collect();
        if let Some((key, speed)) = seen {
            if speed > fastest.0 {
                let label = fabric.joint_label(key);
                let chain = fabric.intervals.values().any(|interval| interval.chain && interval.touches(key));
                fastest = (speed, if chain && label == "Z0" { "a chain link".into() } else { label }, step_tick);
            }
        }
        if tick % DRAG_SAMPLED == 0 {
            for (key, joint) in fabric.joints.iter() {
                let (at, was) = (joint.location, dragged.get(key).copied());
                let slid = match was {
                    Some((before, _)) if at.y < ON_THE_GROUND && before.y < ON_THE_GROUND => {
                        glam::Vec3::new(at.x - before.x, 0.0, at.z - before.z).length()
                    }
                    _ => 0.0,
                };
                dragged.insert(key, (at, was.map_or(0.0, |(_, sum)| sum) + slid));
            }
        }
        if let Some(label) = ended.as_ref() {
            let (worst, total) = dragged.values().fold((0.0f32, 0.0f32), |(worst, total), &(_, sum)| (worst.max(sum), total + sum));
            let mean = total / dragged.len().max(1) as f32;
            if told && mean > 0.03 {
                eprintln!("  dragged over the ground in \"{label}\": {mean:.2} m a joint, {worst:.2} m at worst");
            }
            if told && fastest.0 > SHOCK {
                eprintln!("  a shock in \"{label}\": {} at {:.1} m/s, {:.2} s in", fastest.1, fastest.0, fastest.2 as f32 / 20000.0);
            }
            (dragged, fastest, step_tick) = (SecondaryMap::new(), (0.0, String::new(), 0), 0);
        }
        if ending {
            for (key, interval) in fabric.intervals.iter() {
                *strains.entry(key).unwrap().or_insert(0.0) += interval.strain;
            }
            samples += 1;
        }
        let Some(label) = ended else {
            continue;
        };
        // What the app does between frames must hold for every stage.
        fabric.refresh_bridle_nodes();
        placements(fabric);
        if samples == 0 {
            if told {
                eprintln!("{label}");
            }
            continue;
        }
        forces = forces_of(fabric, physics, &strains, samples);
        (strains, samples) = (SecondaryMap::new(), 0);
        if !told {
            continue;
        }
        let Some(standing_pitch) = standing_pitch.filter(|_| !label.starts_with(['+', '−'])) else {
            line(&label, fabric, &forces);
            continue;
        };
        {
            let rig = fabric.site.as_ref().and_then(|site| site.hook.as_ref()).map(|hook| Rig {
                hook: hook.joint,
                eyes: hook.eyes.clone(),
                height: hook.height,
                place: hook.place,
            });
            report(&label, fabric, rig.as_ref(), &forces, before, standing_pitch);
        }
    }
    forces
}

fn figure() -> (Fabric, Physics, Forces, Option<f32>) {
    let (mut fabric, physics) = standing();
    fabric.update_all_attachment_connections();
    let standing_pitch = attitude(&fabric, 0.0).0;
    let before = standing_forces(&mut fabric, &physics);
    (fabric, physics, before, Some(standing_pitch))
}

fn open_claw() -> (Fabric, Physics, Forces) {
    let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(FabricName::OpenClaw));
    while !executor.is_complete() {
        let _ = executor.iterate();
    }
    let physics = Assembly::physics(&Physics { surface: executor.physics.surface, ..VIEWING });
    let mut fabric = executor.fabric;
    fabric.update_all_attachment_connections();
    let before = standing_forces(&mut fabric, &physics);
    (fabric, physics, before)
}

/// How far the finished figure's forces are from the one the plan builds.
fn compare(fabric: &Fabric, forces: &Forces, before: &Forces) {
    let furthest = before
        .iter()
        .map(|(key, was)| ((forces[key] - was).abs(), key))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .expect("the figure has members");
    eprintln!(
        "  against the figure as the plan builds it: {} differs most, {:.1} kN now and {:.1} kN then",
        member_name(fabric, &fabric.intervals[furthest.1]),
        forces[furthest.1],
        before[furthest.1]
    );
}

/// Taken apart from standing: hook on, guy lines off, leant forward, the
/// arms off strut by strut, the shoulders let down, then the rest back to
/// the pelvis.
///
/// ```
/// cargo test --release --lib assembly::take_headless_hug_apart -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn take_headless_hug_apart() {
    let (mut fabric, physics, before, standing_pitch) = figure();
    let mut assembly = Assembly::of(&fabric, FabricName::HeadlessHug);
    assembly.take_apart();
    follow(&mut assembly, &mut fabric, &physics, &before, standing_pitch, true);
    assert!(assembly.is_apart());
}

/// Taken apart, then built strut by strut from the pelvis lying on the
/// ground to standing on its guy lines: every step's worst cable and strut,
/// and how near the finished figure comes to the one the plan builds.
///
/// ```
/// cargo test --release --lib assembly::put_headless_hug_together -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn put_headless_hug_together() {
    let (mut fabric, physics, before, standing_pitch) = figure();
    let mut assembly = Assembly::of(&fabric, FabricName::HeadlessHug);
    assembly.take_apart();
    follow(&mut assembly, &mut fabric, &physics, &before, standing_pitch, false);
    assembly.put_together();
    let forces = follow(&mut assembly, &mut fabric, &physics, &before, standing_pitch, true);
    assert!(fabric.site.is_none(), "it stands on the surface again");
    compare(&fabric, &forces, &before);
}

/// Open Claw lifted by its top, taken apart from its feet upwards, the net
/// lifted clear, then let down and built again from the top: every step's
/// worst cable and strut, and where it stands at the end.
///
/// ```
/// cargo test --release --lib assembly::open_claw_apart_and_together -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn open_claw_apart_and_together() {
    let (mut fabric, physics, before) = open_claw();
    let stood: Vec<(String, glam::Vec3)> =
        ["Z2", "A30", "B30", "C30"].iter().map(|name| (name.to_string(), fabric.location(joint(&fabric, name)))).collect();
    let mut assembly = Assembly::of(&fabric, FabricName::OpenClaw);
    assembly.take_apart();
    follow(&mut assembly, &mut fabric, &physics, &before, None, true);
    assert!(assembly.is_apart());
    assembly.put_together();
    let forces = follow(&mut assembly, &mut fabric, &physics, &before, None, true);
    assert!(fabric.site.is_none(), "it stands on the surface again");
    compare(&fabric, &forces, &before);
    for (name, was) in stood {
        let now = fabric.location(joint(&fabric, &name));
        eprintln!("  {name} is {:.0} mm from where it stood", now.distance(was) * 1000.0);
        assert!(now.distance(was) < 0.05, "{name} stands where it stood");
    }
}
