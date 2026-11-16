//! Step 0 of docs/brick-evolution.md: is each current brick better than
//! every one-step change to it, and do its numbers matter to its shape?

use super::measure::{measure, Measures};
use super::neighbours::{apply, changes, orbit, turn, Change, Shape};
use super::bake;
use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::{BrickName, BrickRole, OmniParams, SingleParams, TorqueParams};
use crate::build::dsl::brick_library::{get_prototype, get_scale, omni, single_left, torque};
use glam::Vec3;

const TARGETS: [BrickName; 3] = [
    BrickName::SingleTwistLeft,
    BrickName::OmniSymmetrical,
    BrickName::TorqueSymmetrical,
];

struct Candidate {
    name: &'static str,
    value: fn(&Measures) -> f32,
    higher_is_better: bool,
}

const CANDIDATES: [Candidate; 5] = [
    Candidate { name: "stiffness", value: |m| m.stiffness, higher_is_better: true },
    Candidate { name: "clearance", value: |m| m.clearance, higher_is_better: true },
    Candidate { name: "strain spread", value: |m| m.strain_spread, higher_is_better: false },
    Candidate { name: "face irregularity", value: |m| m.face_irregularity, higher_is_better: false },
    Candidate { name: "sphere spread", value: |m| m.sphere_spread, higher_is_better: false },
];

/// Better by more than rounding.
fn beats(candidate: &Candidate, challenger: &Measures, current: &Measures) -> bool {
    let (c, o) = ((candidate.value)(challenger), (candidate.value)(current));
    let margin = 1.0e-3 * o.abs().max(1.0e-3);
    if candidate.higher_is_better { c > o + margin } else { c < o - margin }
}

fn describe(m: &Measures) -> String {
    format!(
        "face strain {:.3} slack {} mech {} stiff {:.4} clear {:.3} strain± {:.4} face± {:.4} sphere± {:.4}",
        m.face_strain, m.slack, m.mechanisms, m.stiffness, m.clearance, m.strain_spread, m.face_irregularity, m.sphere_spread
    )
}

/// The brick's symmetry turn, if it declares one and the shape has it.
fn symmetry_turn(brick: BrickName, shape: &Shape) -> Option<Vec<(crate::build::dsl::brick_dsl::JointName, crate::build::dsl::brick_dsl::JointName)>> {
    let symmetry = get_prototype(brick).symmetry(BrickRole::Seed(1))?;
    turn(shape, symmetry.axis, symmetry.order)
}

fn neighbours(brick: BrickName, orbits: bool) -> (Measures, Vec<(Vec<Change>, Measures)>) {
    let proto = get_prototype(brick);
    let scale = get_scale(brick);
    let (parent_bake, parent) = bake(&proto, scale);
    let current = measure(&parent_bake);
    let turn = if orbits { symmetry_turn(brick, &parent) } else { None };
    if orbits && turn.is_none() {
        return (current, vec![]);
    }
    let mut seen: Vec<Vec<String>> = vec![];
    let mut result = vec![];
    for change in changes(&proto) {
        let step = match &turn {
            Some(turn) => orbit(&change, turn, 3),
            None => vec![change],
        };
        let mut key: Vec<String> = step.iter().map(|c| format!("{c:?}")).collect();
        key.sort();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let child = apply(&proto, &step, &parent);
        if child.pushes.is_empty() || child.faces.is_empty() {
            continue;
        }
        let (child_bake, _) = bake(&child, scale);
        result.push((step, measure(&child_bake)));
    }
    (current, result)
}

fn report(brick: BrickName, orbits: bool) {
    let (current, neighbours) = neighbours(brick, orbits);
    let viable: Vec<_> = neighbours.iter().filter(|(_, m)| m.viable()).collect();
    eprintln!(
        "\n=== {brick:?} {} — {} neighbours, {} viable\n  current: {}",
        if orbits { "symmetric changes" } else { "single changes" },
        neighbours.len(),
        viable.len(),
        describe(&current)
    );
    for kind in ["RemoveStrut", "RemovePull", "RemoveFace", "AddPull", "AddFace", "Rewire"] {
        let of_kind: Vec<_> = neighbours.iter().filter(|(step, _)| format!("{:?}", step[0]).starts_with(kind)).collect();
        if !of_kind.is_empty() {
            let failing = |rule: fn(&Measures) -> bool| of_kind.iter().filter(|(_, m)| rule(m)).count();
            eprintln!(
                "  {kind:<12} {:>4} tried, {:>4} viable; off face strain {}, slack {}, mechanism {}, struts too close {}",
                of_kind.len(),
                of_kind.iter().filter(|(_, m)| m.viable()).count(),
                failing(|m| (m.face_strain - 0.1).abs() >= 0.005),
                failing(|m| m.slack > 0),
                failing(|m| m.mechanisms > 0),
                failing(|m| m.clearance <= 0.1),
            );
        }
    }
    for candidate in &CANDIDATES {
        let mut better: Vec<_> = viable.iter().filter(|(_, m)| beats(candidate, m, &current)).collect();
        better.sort_by(|a, b| {
            let (x, y) = ((candidate.value)(&a.1), (candidate.value)(&b.1));
            if candidate.higher_is_better { y.partial_cmp(&x).unwrap() } else { x.partial_cmp(&y).unwrap() }
        });
        eprintln!("  {:<18} beaten by {:>3}", candidate.name, better.len());
        for (step, m) in better.iter().take(3) {
            eprintln!("      {:?}\n        {}", step, describe(m));
        }
    }
}

#[test]
#[ignore]
fn current_bricks_against_their_neighbours() {
    for brick in TARGETS {
        report(brick, false);
        report(brick, true);
    }
}

/// Mean change in every distance between two joints, in face radial lengths.
fn shape_change(a: &Shape, b: &Shape, radial: f32) -> f32 {
    let distances = |s: &Shape| -> Vec<f32> {
        let mut d = vec![];
        for (i, (_, p)) in s.location.iter().enumerate() {
            for (_, q) in &s.location[i + 1..] {
                d.push((*p - *q).length());
            }
        }
        d
    };
    let (da, db) = (distances(a), distances(b));
    let sum: f32 = da.iter().zip(&db).map(|(x, y)| (x - y) * (x - y)).sum();
    (sum / da.len() as f32).sqrt() / radial
}

fn sweep(brick: BrickName, variant: impl Fn(f32) -> Vec<(&'static str, BrickPrototype)>) {
    let scale = get_scale(brick);
    let (_, reference) = bake(&get_prototype(brick), scale);
    eprintln!("\n=== {brick:?} numbers changed");
    for factor in [0.8, 0.9, 1.0, 1.1, 1.25] {
        for (number, proto) in variant(factor) {
            let (b, shape) = bake(&proto, scale);
            let radial = b.faces.iter().flat_map(|f| f.vertices.map(|v| (b.positions[v] - b.positions[f.midpoint]).length())).sum::<f32>()
                / (3 * b.faces.len()) as f32;
            eprintln!(
                "  {number:<10} ×{factor:<4}  shape change {:.4}  {}",
                shape_change(&reference, &shape, radial),
                describe(&measure(&b))
            );
        }
    }
}

#[test]
#[ignore]
fn current_bricks_with_their_numbers_changed() {
    sweep(BrickName::SingleTwistLeft, |f| {
        let (push, pull) = (Vec3::splat(3.204), 2.0);
        vec![
            ("push", single_left(&SingleParams { push_lengths: push * f, pull_length: pull })),
            ("pull", single_left(&SingleParams { push_lengths: push, pull_length: pull * f })),
        ]
    });
    sweep(BrickName::OmniSymmetrical, |f| {
        vec![("push", omni(&OmniParams { push_lengths: Vec3::splat(3.271 * f) }))]
    });
    sweep(BrickName::TorqueSymmetrical, |f| {
        let (push, pull) = (Vec3::new(3.0, 3.0, 6.0), 1.86);
        vec![
            ("push x", torque(&TorqueParams { push_lengths: push * Vec3::new(f, 1.0, 1.0), pull_length: pull })),
            ("push y", torque(&TorqueParams { push_lengths: push * Vec3::new(1.0, f, 1.0), pull_length: pull })),
            ("push z", torque(&TorqueParams { push_lengths: push * Vec3::new(1.0, 1.0, f), pull_length: pull })),
            ("pull", torque(&TorqueParams { push_lengths: push, pull_length: pull * f })),
        ]
    });
}

/// A current brick moves freely only as a rigid body.
#[test]
fn current_bricks_have_only_rigid_soft_modes() {
    use super::measure::{stiffness_modes, MECHANISM_FRACTION};
    for brick in TARGETS {
        let (b, _) = bake(&get_prototype(brick), get_scale(brick));
        let modes = stiffness_modes(&b);
        let stiffest = modes.iter().cloned().fold(0.0, f64::max);
        let soft = modes.iter().filter(|&&m| m < stiffest * MECHANISM_FRACTION).count();
        assert_eq!(soft, 6, "{brick:?}");
    }
}

/// The rules hold for every current brick.
#[test]
fn current_bricks_are_viable() {
    for brick in TARGETS {
        let (b, _) = bake(&get_prototype(brick), get_scale(brick));
        let m = measure(&b);
        assert!(m.viable(), "{brick:?}: {}", describe(&m));
    }
}

