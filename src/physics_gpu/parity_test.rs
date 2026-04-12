//! Phase 3: CPU vs GPU numerical parity harness.
//!
//! Defines success for the whole port. A `Fabric` is cloned into two
//! copies: one stepped by `Fabric::iterate` on the CPU, the other
//! uploaded via `GpuBatch::parallelize` and stepped on the GPU. After
//! N iterations, the joint positions must match within a tolerance
//! that accounts for CPU/GPU f32 semantics and atomic-i32 force
//! quantization.
//!
//! Tolerance of 1e-3 is a starting point, not a physical requirement.
//! If the harness surfaces real integrator divergences, we fix them
//! on the GPU side; we only loosen the tolerance after verifying the
//! remaining delta is pure atomic-quantization noise.

use glam::Vec3;

use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::brick_library::get_prototype;
use crate::fabric::interval::{Role, Span};
use crate::fabric::physics::presets::{BAKING, CONSTRUCTION};
use crate::fabric::Fabric;
use crate::physics_gpu::GpuBatch;
use crate::units::Meters;

const PARITY_TOLERANCE: f32 = 1e-3;

fn create_headless_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = futures::executor::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        },
    ))
    .ok()?;
    let limits = adapter.limits();
    let (device, queue) = futures::executor::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("physics_gpu parity device"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        },
    ))
    .ok()?;
    Some((device, queue))
}

fn cpu_positions(fabric: &Fabric) -> Vec<Vec3> {
    fabric.joints.values().map(|j| j.location).collect()
}

fn compare(cpu: &[Vec3], gpu: &[Vec3], label: &str) -> f32 {
    assert_eq!(cpu.len(), gpu.len(), "{label}: joint count mismatch");
    let mut max_error: f32 = 0.0;
    for (idx, (c, g)) in cpu.iter().zip(gpu.iter()).enumerate() {
        let err = (*c - *g).length();
        eprintln!("{label}: joint {idx}: cpu={c:?} gpu={g:?} err={err:.3e}");
        if err > max_error {
            max_error = err;
        }
    }
    max_error
}

/// Smallest meaningful case: two joints, one pull cable whose ideal
/// length is shorter than its initial separation. Exercises one
/// elastic interval, one joint-pair mass accumulation, and one
/// force application. No pushes, no gravity, no surface.
#[test]
fn two_joint_pull_parity() {
    let Some((device, queue)) = create_headless_device() else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let mut fabric_cpu = Fabric::new("two-joint-pull".to_string());
    let a = fabric_cpu.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let b = fabric_cpu.create_joint(Vec3::new(1.0, 0.0, 0.0));
    fabric_cpu.create_fixed_interval(a, b, Role::Pulling, Meters(0.8));

    let fabric_gpu = fabric_cpu.clone();
    let physics = CONSTRUCTION.clone();

    let batch = GpuBatch::parallelize(&device, &queue, &[&fabric_gpu], &physics);

    let iterations = 500;
    for _ in 0..iterations {
        fabric_cpu.iterate(&physics);
    }
    batch.step(&device, &queue, iterations);

    let cpu = cpu_positions(&fabric_cpu);
    let gpu = batch.read_positions(&device, &queue);

    let error = compare(&cpu, &gpu, "two_joint_pull");
    assert!(
        error < PARITY_TOLERANCE,
        "max position error {error} exceeds tolerance {PARITY_TOLERANCE}"
    );
}

/// Three pull cables forming a triangle. Each joint is touched by two
/// intervals, so mass accumulation through atomic adds starts to
/// matter. Still no pushes or gravity.
#[test]
fn pull_triangle_parity() {
    let Some((device, queue)) = create_headless_device() else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let mut fabric_cpu = Fabric::new("pull-triangle".to_string());
    let a = fabric_cpu.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let b = fabric_cpu.create_joint(Vec3::new(1.0, 0.0, 0.0));
    let c = fabric_cpu.create_joint(Vec3::new(0.5, 0.0, 0.866_025_4));
    let rest = Meters(0.9);
    fabric_cpu.create_fixed_interval(a, b, Role::Pulling, rest);
    fabric_cpu.create_fixed_interval(b, c, Role::Pulling, rest);
    fabric_cpu.create_fixed_interval(c, a, Role::Pulling, rest);

    let fabric_gpu = fabric_cpu.clone();
    let physics = CONSTRUCTION.clone();

    let batch = GpuBatch::parallelize(&device, &queue, &[&fabric_gpu], &physics);

    let iterations = 500;
    for _ in 0..iterations {
        fabric_cpu.iterate(&physics);
    }
    batch.step(&device, &queue, iterations);

    let cpu = cpu_positions(&fabric_cpu);
    let gpu = batch.read_positions(&device, &queue);

    let error = compare(&cpu, &gpu, "pull_triangle");
    assert!(
        error < PARITY_TOLERANCE,
        "max position error {error} exceeds tolerance {PARITY_TOLERANCE}"
    );
}

/// Diagnostic: run the push-with-pulls setup for a small number of
/// iterations to see where divergence starts. Not an assertion — just
/// prints the state progression.
#[test]
fn push_with_pulls_progression() {
    let Some((device, queue)) = create_headless_device() else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let mut fabric_cpu = Fabric::new("push-progression".to_string());
    let anchor_a = fabric_cpu.create_joint(Vec3::new(-1.0, 0.0, 0.0));
    let push_a = fabric_cpu.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let push_b = fabric_cpu.create_joint(Vec3::new(1.0, 0.0, 0.0));
    let anchor_b = fabric_cpu.create_joint(Vec3::new(2.0, 0.0, 0.0));
    fabric_cpu.create_fixed_interval(push_a, push_b, Role::Pushing, Meters(1.2));
    fabric_cpu.create_fixed_interval(anchor_a, push_a, Role::Pulling, Meters(0.9));
    fabric_cpu.create_fixed_interval(push_b, anchor_b, Role::Pulling, Meters(0.9));

    let fabric_gpu = fabric_cpu.clone();
    let physics = CONSTRUCTION.clone();

    for checkpoint in [1usize, 2, 5, 10, 50] {
        let mut cpu = fabric_cpu.clone();
        for _ in 0..checkpoint {
            cpu.iterate(&physics);
        }
        let cpu_pos = cpu_positions(&cpu);

        // Fresh batch per checkpoint so dispatches are independent
        let fresh_batch = GpuBatch::parallelize(&device, &queue, &[&fabric_gpu], &physics);
        fresh_batch.step(&device, &queue, checkpoint as u32);
        let gpu_pos = fresh_batch.read_positions(&device, &queue);

        eprintln!("--- after {checkpoint} iterations ---");
        compare(&cpu_pos, &gpu_pos, &format!("push@{checkpoint}"));
    }
}

/// One push strut flanked by two pull cables anchoring it on each side.
/// Exercises spring-push force and slack logic. Layout:
/// joint 0 and joint 3 are fixed anchors (by drag — they start with
/// zero velocity and only connect through intervals), joint 1 and
/// joint 2 are the push strut endpoints, with pulls running
/// 0→1 and 2→3. No gravity, no surface.
#[test]
fn push_with_pulls_parity() {
    let Some((device, queue)) = create_headless_device() else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let mut fabric_cpu = Fabric::new("push-with-pulls".to_string());
    let anchor_a = fabric_cpu.create_joint(Vec3::new(-1.0, 0.0, 0.0));
    let push_a = fabric_cpu.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let push_b = fabric_cpu.create_joint(Vec3::new(1.0, 0.0, 0.0));
    let anchor_b = fabric_cpu.create_joint(Vec3::new(2.0, 0.0, 0.0));

    // Push strut between middle joints, slightly longer than current so
    // it pushes outward.
    fabric_cpu.create_fixed_interval(push_a, push_b, Role::Pushing, Meters(1.2));
    // Pull cables from each anchor to the adjacent push endpoint,
    // shorter than current so they pull inward.
    fabric_cpu.create_fixed_interval(anchor_a, push_a, Role::Pulling, Meters(0.9));
    fabric_cpu.create_fixed_interval(push_b, anchor_b, Role::Pulling, Meters(0.9));

    let fabric_gpu = fabric_cpu.clone();
    let physics = CONSTRUCTION.clone();

    let batch = GpuBatch::parallelize(&device, &queue, &[&fabric_gpu], &physics);

    let iterations = 500;
    for _ in 0..iterations {
        fabric_cpu.iterate(&physics);
    }
    batch.step(&device, &queue, iterations);

    let cpu = cpu_positions(&fabric_cpu);
    let gpu = batch.read_positions(&device, &queue);

    let error = compare(&cpu, &gpu, "push_with_pulls");
    assert!(
        error < PARITY_TOLERANCE,
        "max position error {error} exceeds tolerance {PARITY_TOLERANCE}"
    );
}

/// Parity on a SingleTwistLeft brick. The prototype produced by
/// `proto.to_fabric` uses `Span::Approaching` for all pull and face
/// radial intervals, with a 1-second approach duration. The GPU
/// backend only accepts finished fabrics, so we first bake the
/// prototype on the CPU long enough that every `Approaching` span
/// has resolved to `Fixed` — then clone and run the parity test
/// from that fully-built state.
#[test]
fn single_twist_left_parity() {
    let Some((device, queue)) = create_headless_device() else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let proto = get_prototype(BrickName::SingleTwistLeft);
    let mut fabric_cpu = proto.to_fabric(BrickName::SingleTwistLeft.face_scaling());

    // Drive the approaching intervals to completion under BAKING
    // physics. The approach duration is 1 second = 20000 ticks; we
    // give it 25000 to cover the approach plus a little settling.
    let baking = BAKING.clone();
    for _ in 0..25_000 {
        fabric_cpu.iterate(&baking);
    }
    assert!(
        fabric_cpu
            .intervals
            .values()
            .all(|i| matches!(i.span, Span::Fixed { .. })),
        "all interval spans should have resolved to Fixed after baking"
    );

    let fabric_gpu = fabric_cpu.clone();
    let physics = CONSTRUCTION.clone();

    let batch = GpuBatch::parallelize(&device, &queue, &[&fabric_gpu], &physics);

    let iterations = 500;
    for _ in 0..iterations {
        fabric_cpu.iterate(&physics);
    }
    batch.step(&device, &queue, iterations);

    let cpu = cpu_positions(&fabric_cpu);
    let gpu = batch.read_positions(&device, &queue);

    let error = compare(&cpu, &gpu, "single_twist_left");
    assert!(
        error < PARITY_TOLERANCE,
        "max position error {error} exceeds tolerance {PARITY_TOLERANCE}"
    );
}

/// Phase 4 acceptance test: N identical fabrics in one batch all
/// produce the same positions. This validates the slot-indexing
/// arithmetic without introducing per-slot variation.
#[test]
fn identical_batch_produces_identical_results() {
    let Some((device, queue)) = create_headless_device() else {
        eprintln!("No compute-capable adapter available; skipping.");
        return;
    };

    let mut fabric = Fabric::new("batch-triangle".to_string());
    let a = fabric.create_joint(Vec3::new(0.0, 0.0, 0.0));
    let b = fabric.create_joint(Vec3::new(1.0, 0.0, 0.0));
    let c = fabric.create_joint(Vec3::new(0.5, 0.0, 0.866_025_4));
    fabric.create_fixed_interval(a, b, Role::Pulling, Meters(0.9));
    fabric.create_fixed_interval(b, c, Role::Pulling, Meters(0.9));
    fabric.create_fixed_interval(c, a, Role::Pulling, Meters(0.9));

    let physics = CONSTRUCTION.clone();
    let n = 10;
    let refs: Vec<&Fabric> = (0..n).map(|_| &fabric).collect();
    let batch = GpuBatch::parallelize(&device, &queue, &refs, &physics);
    assert_eq!(batch.num_slots(), n as u32);

    batch.step(&device, &queue, 500);
    let all_positions = batch.read_all_positions(&device, &queue);
    assert_eq!(all_positions.len(), n);

    let reference = &all_positions[0];
    for (slot, positions) in all_positions.iter().enumerate().skip(1) {
        assert_eq!(
            positions.len(),
            reference.len(),
            "slot {slot}: joint count mismatch"
        );
        for (j, (r, p)) in reference.iter().zip(positions.iter()).enumerate() {
            let err = (*r - *p).length();
            assert!(
                err == 0.0,
                "slot {slot} joint {j}: expected bit-exact match to slot 0, got err={err}"
            );
        }
    }

    // Verify slot 0 also matches single-fabric CPU reference.
    let mut fabric_cpu = fabric.clone();
    for _ in 0..500 {
        fabric_cpu.iterate(&physics);
    }
    let cpu = cpu_positions(&fabric_cpu);
    let error = compare(&cpu, reference, "batch_slot_0");
    assert!(
        error < PARITY_TOLERANCE,
        "slot 0 vs CPU: max error {error} exceeds {PARITY_TOLERANCE}"
    );
}
