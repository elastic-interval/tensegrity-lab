//! Phase 2 smoke test: parallelize a real `Fabric` and step it on the GPU.
//!
//! This test proves end-to-end that a CPU-built tensegrity-lab `Fabric`
//! can be uploaded via `GpuBatch::parallelize`, stepped forward, and
//! read back without producing NaNs or crashing. Numerical parity
//! against the CPU reference is phase 3's job, not this test's.

use glam::Vec3;

use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::interval::Role;
use crate::fabric::Fabric;
use crate::physics_gpu::{create_headless_device, GpuBatch};
use crate::units::Meters;

/// Build a tiny triangle of three pull cables. Each cable's ideal
/// length is 90% of its actual length, so all three pull the triangle
/// inward toward its centroid.
fn build_triangle_fabric() -> Fabric {
    let mut fabric = Fabric::new("smoke-triangle".to_string());

    let a = fabric.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let b = fabric.create_joint(Vec3::new(1.0, 0.0, 0.0));
    let c = fabric.create_joint(Vec3::new(0.5, 0.0, 0.866_025_4));

    let rest = Meters(0.9);
    fabric.create_fixed_interval(a, b, Role::Pulling, rest);
    fabric.create_fixed_interval(b, c, Role::Pulling, rest);
    fabric.create_fixed_interval(c, a, Role::Pulling, rest);

    fabric
}

#[test]
fn parallelize_runs_on_real_fabric() {
    let Some((device, queue)) = create_headless_device("physics_gpu smoke") else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let fabric = build_triangle_fabric();
    let physics = CONSTRUCTION.clone();

    let initial_centroid = centroid(&fabric);

    let batch = GpuBatch::parallelize(&device, &queue, &[&fabric], &physics);
    batch.step(&device, &queue, 200);

    let positions = batch.read_positions(&device, &queue);
    assert_eq!(positions.len(), 3);

    for (idx, p) in positions.iter().enumerate() {
        assert!(
            p.is_finite(),
            "joint {idx} position has non-finite component: {p:?}"
        );
    }

    let final_centroid = positions.iter().copied().sum::<Vec3>() / 3.0;
    let drift = (final_centroid - initial_centroid).length();
    assert!(
        drift < 0.01,
        "centroid drifted by {drift} m, expected symmetric contraction"
    );

    let mut max_distance_change = 0.0f32;
    for (idx, p) in positions.iter().enumerate() {
        let initial = initial_joint_position(idx);
        let change = (*p - initial).length();
        max_distance_change = max_distance_change.max(change);
    }
    assert!(
        max_distance_change > 1e-6,
        "joints did not move at all after 200 iterations (got {max_distance_change})"
    );
}

fn centroid(fabric: &Fabric) -> Vec3 {
    let n = fabric.joints.len() as f32;
    fabric
        .joints
        .values()
        .map(|j| j.location)
        .sum::<Vec3>()
        / n
}

fn initial_joint_position(index: usize) -> Vec3 {
    match index {
        0 => Vec3::new(0.0, 0.0, 0.0),
        1 => Vec3::new(1.0, 0.0, 0.0),
        2 => Vec3::new(0.5, 0.0, 0.866_025_4),
        _ => panic!("unexpected joint index {index}"),
    }
}
