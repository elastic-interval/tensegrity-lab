//! Headless Hug taken apart strut by strut and put together again
//! (`build/assembly.rs`), with every step's worst cable and strut.
//! Results are in `docs/crane-lift.md`.

use crate::build::assembly::Assembly;
use crate::connector::parts::placements;
use crate::crane_lift::{
    attitude, buckling_load, forces_of, is_cable, mass_and_centre, member_name, report, standing, standing_forces, ticks, Forces,
    Rig, AVERAGED, CABLE_WORKING_LOAD,
};
use crate::fabric::interval::Role;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, IntervalKey};
use slotmap::SecondaryMap;

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

/// Run what the assembly has to do, telling each step's forces if `told`.
/// Gives the forces at the end of the last step.
fn follow(assembly: &mut Assembly, fabric: &mut Fabric, physics: &Physics, before: &Forces, standing_pitch: f32, told: bool) -> Forces {
    let mut strains: SecondaryMap<IntervalKey, f32> = SecondaryMap::new();
    let mut samples = 0;
    let mut forces = Forces::new();
    while !assembly.is_idle() {
        let ending = assembly.remaining_ticks().is_some_and(|left| left <= ticks(AVERAGED));
        let ended = assembly.iterate(fabric, physics);
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
        if label.starts_with(['+', '−']) {
            line(&label, fabric, &forces);
        } else {
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

fn figure() -> (Fabric, Physics, Forces, f32) {
    let (mut fabric, physics) = standing();
    fabric.update_all_attachment_connections();
    let standing_pitch = attitude(&fabric, 0.0).0;
    let before = standing_forces(&mut fabric, &physics);
    (fabric, physics, before, standing_pitch)
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
    let mut assembly = Assembly::of(&fabric);
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
    let mut assembly = Assembly::of(&fabric);
    assembly.take_apart();
    follow(&mut assembly, &mut fabric, &physics, &before, standing_pitch, false);
    assembly.put_together();
    let forces = follow(&mut assembly, &mut fabric, &physics, &before, standing_pitch, true);
    assert!(fabric.site.is_none(), "it stands on the surface again");
    let furthest = before
        .iter()
        .map(|(key, was)| ((forces[key] - was).abs(), key))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .expect("the figure has members");
    eprintln!(
        "  against the figure as the plan builds it: {} differs most, {:.1} kN now and {:.1} kN then",
        member_name(&fabric, &fabric.intervals[furthest.1]),
        forces[furthest.1],
        before[furthest.1]
    );
}
