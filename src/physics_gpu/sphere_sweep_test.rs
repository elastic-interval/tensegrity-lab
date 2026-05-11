//! Headless GPU sweep for algorithmic Sphere fabrics.
//!
//! Generates sphere fabrics at a range of frequencies, uploads each to the
//! GPU, drives a slow pretension approach by advancing `fabric.age` between
//! `update_ideals` calls, then settles and reads per-slot facts. No CPU
//! iteration — pure generator → GPU.

use glam::Vec3;

use crate::Age;
use crate::build::algo::tensegrity_sphere::generate_sphere;
use crate::fabric::interval::{Role, Span};
use crate::fabric::physics::presets;
use crate::fabric::{Fabric, IntervalKey};
use crate::physics_gpu::batch::{GpuBatch, SlotFacts};
use crate::physics_gpu::create_headless_device;
use crate::units::{Grams, GramsPerMeter, Meters, Seconds, Unit};

/// Configure a sphere fabric for GPU-only iteration: sets joint/push densities
/// and installs `Span::Approaching` pretensions with the candidate scaling law.
///
/// `strain_scale` is multiplied by the base strains, so passing 1.0 gives the
/// canonical `1 + 0.10/√f` / `1 − 0.05/√f` endpoints.
fn build_sphere_with_pretension(
    frequency: usize,
    radius: f32,
    approach_duration: Seconds,
    strain_scale: f32,
) -> Fabric {
    let mut fabric = generate_sphere(frequency, radius);
    fabric.dimensions = fabric
        .dimensions
        .with_joint_mass(Grams(2.0))
        .with_push_density(GramsPerMeter(3.0));

    let f = frequency as f32;
    let push_strain = strain_scale * 0.10 / f.sqrt();
    let pull_strain = strain_scale * 0.05 / f.sqrt();
    let push_pretension = 1.0 + push_strain;
    let pull_pretension = 1.0 - pull_strain;

    let cable_actuals: Vec<(IntervalKey, f32)> = fabric
        .intervals
        .iter()
        .filter(|(_, i)| i.role != Role::Pushing)
        .map(|(k, i)| {
            let a = fabric.joints[i.alpha_key].location;
            let o = fabric.joints[i.omega_key].location;
            (k, (o - a).length())
        })
        .collect();

    let age = fabric.age;
    for interval in fabric.intervals.values_mut() {
        if interval.role == Role::Pushing {
            if let Span::Fixed { length } = interval.span {
                interval.span = Span::Approaching {
                    start_length: length,
                    target_length: Meters(length.f32() * push_pretension),
                    start_age: age,
                    duration: approach_duration,
                };
            }
        }
    }
    for (key, actual) in cable_actuals {
        if let Some(interval) = fabric.intervals.get_mut(key) {
            if let Span::Fixed { length } = interval.span {
                interval.span = Span::Approaching {
                    start_length: length,
                    target_length: Meters(actual * pull_pretension),
                    start_age: age,
                    duration: approach_duration,
                };
            }
        }
    }

    fabric
}

/// One-frequency GPU run: upload, drive approach, settle, return final facts.
/// Positions are read back after settling so we can detect explosion via
/// bounding_radius blowup or NaN.
fn run_single_sphere_on_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frequency: usize,
    radius: f32,
    approach_duration: Seconds,
    settle_duration: Seconds,
    strain_scale: f32,
    rounds_per_phase: u32,
) -> (SlotFacts, bool) {
    let mut fabric = build_sphere_with_pretension(
        frequency,
        radius,
        approach_duration,
        strain_scale,
    );

    let physics = presets::CONSTRUCTION;
    let batch = GpuBatch::parallelize(device, queue, &[&fabric], &physics);

    let dt = Age::iteration_duration();
    let approach_iters = (approach_duration.0 / dt) as u32;
    let settle_iters = (settle_duration.0 / dt) as u32;

    // Drive the approach in chunks so Span::Approaching interpolates forward
    // on every `update_ideals` call. rounds_per_phase chunks across approach.
    let approach_chunk = (approach_iters / rounds_per_phase).max(1);
    let mut total_ticked = 0u32;
    for _ in 0..rounds_per_phase {
        fabric.age = fabric.age.advanced(approach_chunk as usize);
        batch.update_ideals(queue, &[&fabric], &physics);
        batch.step(device, queue, approach_chunk);
        total_ticked += approach_chunk;
    }

    // Settle: spans are now clamped at target (completion == 1.0), so no more
    // updates needed. One big step.
    if settle_iters > 0 {
        batch.step(device, queue, settle_iters);
    }

    let all_positions = batch.read_all_positions(device, queue);
    let positions = &all_positions[0];
    let exploded = positions.iter().any(|p| !p.is_finite())
        || positions
            .iter()
            .map(|p| p.length())
            .fold(0.0f32, f32::max)
            > radius * 20.0;

    let _ = total_ticked;
    (SlotFacts::from_positions(positions), exploded)
}

#[test]
fn sphere_gpu_sweep_scaling_law() {
    let Some((device, queue)) = create_headless_device("physics_gpu sphere sweep") else {
        eprintln!("skipping: no GPU adapter");
        return;
    };

    let radius = 10.0f32;
    let approach_duration = Seconds(2.0);
    let settle_duration = Seconds(1.0);
    let strain_scale = 1.0; // canonical 1/√f scaling
    let rounds_per_phase = 40; // 40 update_ideals calls across approach

    let frequencies: &[usize] = &[3, 4, 6, 8, 12, 16, 24, 32];

    println!(
        "\n{:>5} | {:>7} | {:>9} | {:>10} | {:>12} | {:>10} | {:>8}",
        "freq", "joints", "intervals", "push_strain", "bound_radius", "centroid_y", "exploded"
    );
    println!("{}", "-".repeat(82));

    for &frequency in frequencies {
        let (facts, exploded) = run_single_sphere_on_gpu(
            &device,
            &queue,
            frequency,
            radius,
            approach_duration,
            settle_duration,
            strain_scale,
            rounds_per_phase,
        );
        let f = frequency as f32;
        let push_strain = 0.10 / f.sqrt();
        let temp_fabric = generate_sphere(frequency, radius);
        println!(
            "{:>5} | {:>7} | {:>9} | {:>10.4} | {:>12.4} | {:>10.4} | {:>8}",
            frequency,
            temp_fabric.joints.len(),
            temp_fabric.intervals.len(),
            push_strain,
            facts.bounding_radius,
            facts.centroid.y,
            exploded,
        );
    }
}

/// Test three candidate scaling exponents side-by-side at a single
/// high-frequency point. Prints which one best holds the bounding radius
/// near the original geometric radius.
#[test]
fn sphere_gpu_strain_exponent_probe() {
    let Some((device, queue)) = create_headless_device("physics_gpu sphere sweep") else {
        eprintln!("skipping: no GPU adapter");
        return;
    };

    let radius = 10.0f32;
    let approach_duration = Seconds(2.0);
    let settle_duration = Seconds(1.0);
    let rounds_per_phase = 40;
    let frequency = 12usize;

    let variants: &[(&str, f32)] = &[
        ("1/√f (default)", 1.0),
        ("1/√f × 2.0", 2.0),
        ("1/√f × 0.5", 0.5),
    ];

    println!("\nfrequency = {frequency}, radius = {radius}");
    println!("{:>20} | {:>12} | {:>10} | {:>8}", "variant", "bound_radius", "centroid_y", "exploded");
    println!("{}", "-".repeat(60));
    for &(label, strain_scale) in variants {
        let (facts, exploded) = run_single_sphere_on_gpu(
            &device,
            &queue,
            frequency,
            radius,
            approach_duration,
            settle_duration,
            strain_scale,
            rounds_per_phase,
        );
        println!(
            "{:>20} | {:>12.4} | {:>10.4} | {:>8}",
            label, facts.bounding_radius, facts.centroid.y, exploded,
        );
    }
}

#[allow(dead_code)]
fn summarise_positions(label: &str, positions: &[Vec3]) {
    let finite = positions.iter().all(|p| p.is_finite());
    let max_r = positions
        .iter()
        .map(|p| p.length())
        .fold(0.0f32, f32::max);
    println!("{label}: n={} finite={finite} max_r={max_r:.3}", positions.len());
}
