//! `GpuBatch`: upload N fabrics, step them all in lockstep on the GPU.
//!
//! Usage: build fabrics on the CPU however you like, then call
//! `GpuBatch::parallelize(&device, &queue, &[&fabric], &physics)`.
//! Call `step(n)` to advance all slots by n iterations, and
//! `read_all_positions()` to read back per-slot joint positions.
//!
//! Buffer layout: pad-to-max. Every slot occupies the same number of
//! entries (the largest fabric's count), padded with zeros. Dispatch
//! is 2D: x = local index within slot, y = slot index.

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::fabric::interval::{Role, Span};
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, JointKey};
use crate::physics_gpu::params::{GpuPhysicsConfig, PhysicsParams};
use crate::units::{Meters, Unit};

use std::collections::HashMap;

struct FabricSnapshot {
    positions: Vec<[f32; 4]>,
    velocities: Vec<[f32; 4]>,
    elastic_alpha: Vec<u32>,
    elastic_omega: Vec<u32>,
    elastic_ideal: Vec<f32>,
    elastic_k: Vec<f32>,
    elastic_linear_density: Vec<f32>,
    push_alpha: Vec<u32>,
    push_omega: Vec<u32>,
    push_ideal: Vec<f32>,
    push_k: Vec<f32>,
    push_linear_density: Vec<f32>,
    #[allow(dead_code)]
    joint_key_map: HashMap<JointKey, u32>,
}

impl FabricSnapshot {
    fn from_fabric(fabric: &Fabric, physics: &Physics) -> Self {
        let mut positions = Vec::with_capacity(fabric.joints.len());
        let mut velocities = Vec::with_capacity(fabric.joints.len());
        let mut joint_key_map = HashMap::with_capacity(fabric.joints.len());

        for (idx, (key, joint)) in fabric.joints.iter().enumerate() {
            positions.push([joint.location.x, joint.location.y, joint.location.z, 1.0]);
            velocities.push([joint.velocity.x, joint.velocity.y, joint.velocity.z, 0.0]);
            joint_key_map.insert(key, idx as u32);
        }

        let mut elastic_alpha = Vec::new();
        let mut elastic_omega = Vec::new();
        let mut elastic_ideal = Vec::new();
        let mut elastic_k = Vec::new();
        let mut elastic_linear_density = Vec::new();
        let mut push_alpha = Vec::new();
        let mut push_omega = Vec::new();
        let mut push_ideal = Vec::new();
        let mut push_k = Vec::new();
        let mut push_linear_density = Vec::new();

        for interval in fabric.intervals.values() {
            let alpha = joint_key_map[&interval.alpha_key];
            let omega = joint_key_map[&interval.omega_key];
            let ideal = resolve_span(&interval.span, fabric.age);
            let k = interval.material.spring_constant(ideal, physics).f32()
                * interval.stiffness.as_factor();
            let linear_density = fabric
                .dimensions
                .linear_density(interval.material, physics)
                .f32();

            match interval.role {
                Role::Pushing => {
                    push_alpha.push(alpha);
                    push_omega.push(omega);
                    push_ideal.push(ideal.f32());
                    push_k.push(k);
                    push_linear_density.push(linear_density);
                }
                _ => {
                    elastic_alpha.push(alpha);
                    elastic_omega.push(omega);
                    elastic_ideal.push(ideal.f32());
                    elastic_k.push(k);
                    elastic_linear_density.push(linear_density);
                }
            }
        }

        Self {
            positions,
            velocities,
            elastic_alpha,
            elastic_omega,
            elastic_ideal,
            elastic_k,
            elastic_linear_density,
            push_alpha,
            push_omega,
            push_ideal,
            push_k,
            push_linear_density,
            joint_key_map,
        }
    }

    fn num_joints(&self) -> u32 {
        self.positions.len() as u32
    }
    fn num_elastic(&self) -> u32 {
        self.elastic_alpha.len() as u32
    }
    fn num_push(&self) -> u32 {
        self.push_alpha.len() as u32
    }
}

fn resolve_span(span: &Span, age: crate::Age) -> Meters {
    match *span {
        Span::Fixed { length } => length,
        Span::Approaching {
            start_length,
            target_length,
            start_age,
            duration,
        } => {
            let elapsed = age.elapsed_since(start_age);
            let completion = (elapsed.f32() / duration.f32()).clamp(0.0, 1.0);
            Meters(start_length.f32() * (1.0 - completion) + target_length.f32() * completion)
        }
        Span::Measuring { .. } => {
            panic!("physics_gpu::parallelize: Span::Measuring not supported");
        }
    }
}

pub struct GpuBatch {
    joint_bind_group: wgpu::BindGroup,
    elastic_bind_group: wgpu::BindGroup,
    params_bind_group: wgpu::BindGroup,
    push_bind_group: wgpu::BindGroup,

    half_kick_pipeline: wgpu::ComputePipeline,
    reset_pipeline: wgpu::ComputePipeline,
    elastic_forces_pipeline: wgpu::ComputePipeline,
    push_forces_pipeline: wgpu::ComputePipeline,
    second_half_kick_pipeline: wgpu::ComputePipeline,
    ground_collision_pipeline: wgpu::ComputePipeline,

    position_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,

    elastic_ideal_buffer: wgpu::Buffer,
    elastic_k_buffer: wgpu::Buffer,
    push_ideal_buffer: wgpu::Buffer,
    push_k_buffer: wgpu::Buffer,
    frozen_buffer: wgpu::Buffer,

    num_slots: u32,
    max_joints: u32,
    max_elastic: u32,
    max_push: u32,
    slot_joint_counts: Vec<u32>,
}

impl GpuBatch {
    pub fn parallelize(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fabrics: &[&Fabric],
        physics: &Physics,
    ) -> Self {
        assert!(!fabrics.is_empty(), "GpuBatch: need at least one fabric");

        let snapshots: Vec<FabricSnapshot> = fabrics
            .iter()
            .map(|f| FabricSnapshot::from_fabric(f, physics))
            .collect();

        let num_slots = snapshots.len() as u32;
        let max_joints = snapshots.iter().map(|s| s.num_joints()).max().unwrap();
        let max_elastic = snapshots.iter().map(|s| s.num_elastic()).max().unwrap_or(0);
        let max_push = snapshots.iter().map(|s| s.num_push()).max().unwrap_or(0);
        let slot_joint_counts: Vec<u32> = snapshots.iter().map(|s| s.num_joints()).collect();

        assert!(max_joints > 0, "GpuBatch: all fabrics have zero joints");

        let config = GpuPhysicsConfig::from_fabric(fabrics[0], physics);

        // ---- Build flat padded buffers ----
        let total_joints = (num_slots * max_joints) as usize;
        let total_elastic = (num_slots * max_elastic) as usize;
        let total_push = (num_slots * max_push) as usize;

        let mut all_positions = vec![[0.0f32; 4]; total_joints];
        let mut all_velocities = vec![[0.0f32; 4]; total_joints];
        let mut all_elastic_alpha = vec![0u32; total_elastic.max(1)];
        let mut all_elastic_omega = vec![0u32; total_elastic.max(1)];
        let mut all_elastic_ideal = vec![0.0f32; total_elastic.max(1)];
        let mut all_elastic_k = vec![0.0f32; total_elastic.max(1)];
        let mut all_elastic_ld = vec![0.0f32; total_elastic.max(1)];
        let mut all_push_alpha = vec![0u32; total_push.max(1)];
        let mut all_push_omega = vec![0u32; total_push.max(1)];
        let mut all_push_ideal = vec![0.0f32; total_push.max(1)];
        let mut all_push_k = vec![0.0f32; total_push.max(1)];
        let mut all_push_ld = vec![0.0f32; total_push.max(1)];

        for (slot, snap) in snapshots.iter().enumerate() {
            let jo = slot * max_joints as usize;
            for (i, p) in snap.positions.iter().enumerate() {
                all_positions[jo + i] = *p;
            }
            for (i, v) in snap.velocities.iter().enumerate() {
                all_velocities[jo + i] = *v;
            }

            let eo = slot * max_elastic as usize;
            for (i, &v) in snap.elastic_alpha.iter().enumerate() {
                all_elastic_alpha[eo + i] = v;
            }
            for (i, &v) in snap.elastic_omega.iter().enumerate() {
                all_elastic_omega[eo + i] = v;
            }
            for (i, &v) in snap.elastic_ideal.iter().enumerate() {
                all_elastic_ideal[eo + i] = v;
            }
            for (i, &v) in snap.elastic_k.iter().enumerate() {
                all_elastic_k[eo + i] = v;
            }
            for (i, &v) in snap.elastic_linear_density.iter().enumerate() {
                all_elastic_ld[eo + i] = v;
            }

            let po = slot * max_push as usize;
            for (i, &v) in snap.push_alpha.iter().enumerate() {
                all_push_alpha[po + i] = v;
            }
            for (i, &v) in snap.push_omega.iter().enumerate() {
                all_push_omega[po + i] = v;
            }
            for (i, &v) in snap.push_ideal.iter().enumerate() {
                all_push_ideal[po + i] = v;
            }
            for (i, &v) in snap.push_k.iter().enumerate() {
                all_push_k[po + i] = v;
            }
            for (i, &v) in snap.push_linear_density.iter().enumerate() {
                all_push_ld[po + i] = v;
            }
        }

        // ---- GPU buffers ----
        let writable = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;

        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu positions"),
            contents: bytemuck::cast_slice(&all_positions),
            usage: writable | wgpu::BufferUsages::COPY_SRC,
        });
        let velocity_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu velocities"),
            contents: bytemuck::cast_slice(&all_velocities),
            usage: writable,
        });

        let force_buffers = {
            let zeros = vec![0i32; total_joints];
            [("fx", &zeros), ("fy", &zeros), ("fz", &zeros)].map(|(label, data)| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(data),
                    usage: writable,
                })
            })
        };

        let ambient_i32 = (config.ambient_mass * 1e4) as i32;
        let mass_init = vec![ambient_i32; total_joints];
        let mass_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu masses"),
            contents: bytemuck::cast_slice(&mass_init),
            usage: writable,
        });

        let frozen_init = vec![0u32; num_slots as usize];
        let frozen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu frozen"),
            contents: bytemuck::cast_slice(&frozen_init),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let mk = |label: &str, data: &[u32]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        let mkf = |label: &str, data: &[f32]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        let mkf_writable = |label: &str, data: &[f32]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
        };

        let ea = mk("elastic_alpha", &all_elastic_alpha);
        let eo_buf = mk("elastic_omega", &all_elastic_omega);
        let ei = mkf_writable("elastic_ideal", &all_elastic_ideal);
        let ek = mkf_writable("elastic_k", &all_elastic_k);
        let eld = mkf("elastic_ld", &all_elastic_ld);

        let pa = mk("push_alpha", &all_push_alpha);
        let po_buf = mk("push_omega", &all_push_omega);
        let pi = mkf_writable("push_ideal", &all_push_ideal);
        let pk = mkf_writable("push_k", &all_push_k);
        let pld = mkf("push_ld", &all_push_ld);

        let params = PhysicsParams::from_config(&config, max_joints, max_elastic, max_push, num_slots);
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu staging"),
            size: (total_joints as u64) * 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- Bind group layouts ----
        let joint_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu joint BGL"),
            entries: &(0..7).map(|i| storage_entry(i, false)).collect::<Vec<_>>(),
        });
        let elastic_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu elastic BGL"),
            entries: &(0..5).map(|i| storage_entry(i, true)).collect::<Vec<_>>(),
        });
        let params_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu params BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let push_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu push BGL"),
            entries: &(0..5).map(|i| storage_entry(i, true)).collect::<Vec<_>>(),
        });

        // ---- Bind groups ----
        macro_rules! bg_entry {
            ($binding:expr, $buffer:expr) => {
                wgpu::BindGroupEntry {
                    binding: $binding,
                    resource: $buffer.as_entire_binding(),
                }
            };
        }

        let joint_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu joint BG"),
            layout: &joint_bgl,
            entries: &[
                bg_entry!(0, position_buffer),
                bg_entry!(1, velocity_buffer),
                bg_entry!(2, force_buffers[0]),
                bg_entry!(3, force_buffers[1]),
                bg_entry!(4, force_buffers[2]),
                bg_entry!(5, mass_buffer),
                bg_entry!(6, frozen_buffer),
            ],
        });
        let elastic_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu elastic BG"),
            layout: &elastic_bgl,
            entries: &[
                bg_entry!(0, ea),
                bg_entry!(1, eo_buf),
                bg_entry!(2, ei),
                bg_entry!(3, ek),
                bg_entry!(4, eld),
            ],
        });
        let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu params BG"),
            layout: &params_bgl,
            entries: &[bg_entry!(0, params_buffer)],
        });
        let push_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu push BG"),
            layout: &push_bgl,
            entries: &[
                bg_entry!(0, pa),
                bg_entry!(1, po_buf),
                bg_entry!(2, pi),
                bg_entry!(3, pk),
                bg_entry!(4, pld),
            ],
        });

        // ---- Pipelines ----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpu pipeline layout"),
            bind_group_layouts: &[&joint_bgl, &elastic_bgl, &params_bgl, &push_bgl],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/physics.wgsl").into()),
        });

        let pipe = |entry: &'static str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let _ = queue;

        Self {
            joint_bind_group,
            elastic_bind_group,
            params_bind_group,
            push_bind_group,
            half_kick_pipeline: pipe("half_kick_and_drift"),
            reset_pipeline: pipe("reset_forces_and_mass"),
            elastic_forces_pipeline: pipe("elastic_forces"),
            push_forces_pipeline: pipe("push_forces"),
            second_half_kick_pipeline: pipe("second_half_kick"),
            ground_collision_pipeline: pipe("ground_collision"),
            position_buffer,
            staging_buffer,
            elastic_ideal_buffer: ei,
            elastic_k_buffer: ek,
            push_ideal_buffer: pi,
            push_k_buffer: pk,
            frozen_buffer,
            num_slots,
            max_joints,
            max_elastic,
            max_push,
            slot_joint_counts,
        }
    }

    /// Re-resolve `Span` values from the given fabrics (using each fabric's
    /// current `age`) and write the resulting `ideal` and `k` arrays into the
    /// GPU storage buffers, leaving positions and velocities untouched.
    ///
    /// Use this to drive a slow approach phase on GPU: advance each fabric's
    /// `age` on the CPU between calls, and `Span::Approaching` spans will
    /// interpolate toward their targets without a CPU iteration round trip.
    ///
    /// The fabrics must have the same interval topology (order and roles) as
    /// those passed to `parallelize`; only span/material-derived fields are
    /// updated. Panics if slot count or per-slot interval counts differ.
    pub fn update_ideals(
        &self,
        queue: &wgpu::Queue,
        fabrics: &[&Fabric],
        physics: &Physics,
    ) {
        assert_eq!(
            fabrics.len() as u32,
            self.num_slots,
            "update_ideals: slot count mismatch"
        );

        let total_elastic = (self.num_slots * self.max_elastic) as usize;
        let total_push = (self.num_slots * self.max_push) as usize;
        let mut all_elastic_ideal = vec![0.0f32; total_elastic.max(1)];
        let mut all_elastic_k = vec![0.0f32; total_elastic.max(1)];
        let mut all_push_ideal = vec![0.0f32; total_push.max(1)];
        let mut all_push_k = vec![0.0f32; total_push.max(1)];

        for (slot, fabric) in fabrics.iter().enumerate() {
            let mut elastic_i = 0usize;
            let mut push_i = 0usize;
            let eo = slot * self.max_elastic as usize;
            let po = slot * self.max_push as usize;
            for interval in fabric.intervals.values() {
                let ideal = resolve_span(&interval.span, fabric.age);
                let k = interval.material.spring_constant(ideal, physics).f32()
                    * interval.stiffness.as_factor();
                match interval.role {
                    Role::Pushing => {
                        assert!(
                            push_i < self.max_push as usize,
                            "update_ideals: push count exceeds parallelize-time max"
                        );
                        all_push_ideal[po + push_i] = ideal.f32();
                        all_push_k[po + push_i] = k;
                        push_i += 1;
                    }
                    _ => {
                        assert!(
                            elastic_i < self.max_elastic as usize,
                            "update_ideals: elastic count exceeds parallelize-time max"
                        );
                        all_elastic_ideal[eo + elastic_i] = ideal.f32();
                        all_elastic_k[eo + elastic_i] = k;
                        elastic_i += 1;
                    }
                }
            }
        }

        if self.max_elastic > 0 {
            queue.write_buffer(
                &self.elastic_ideal_buffer,
                0,
                bytemuck::cast_slice(&all_elastic_ideal),
            );
            queue.write_buffer(
                &self.elastic_k_buffer,
                0,
                bytemuck::cast_slice(&all_elastic_k),
            );
        }
        if self.max_push > 0 {
            queue.write_buffer(
                &self.push_ideal_buffer,
                0,
                bytemuck::cast_slice(&all_push_ideal),
            );
            queue.write_buffer(
                &self.push_k_buffer,
                0,
                bytemuck::cast_slice(&all_push_k),
            );
        }
    }

    pub fn step(&self, device: &wgpu::Device, queue: &wgpu::Queue, iterations: u32) {
        if iterations == 0 || self.max_joints == 0 {
            return;
        }

        let joint_x = (self.max_joints + 63) / 64;
        let elastic_x = if self.max_elastic > 0 {
            (self.max_elastic + 63) / 64
        } else {
            0
        };
        let push_x = if self.max_push > 0 {
            (self.max_push + 63) / 64
        } else {
            0
        };
        let slots_y = self.num_slots;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu step"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gpu step pass"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &self.joint_bind_group, &[]);
            pass.set_bind_group(1, &self.elastic_bind_group, &[]);
            pass.set_bind_group(2, &self.params_bind_group, &[]);
            pass.set_bind_group(3, &self.push_bind_group, &[]);

            for _ in 0..iterations {
                pass.set_pipeline(&self.half_kick_pipeline);
                pass.dispatch_workgroups(joint_x, slots_y, 1);

                pass.set_pipeline(&self.reset_pipeline);
                pass.dispatch_workgroups(joint_x, slots_y, 1);

                if elastic_x > 0 {
                    pass.set_pipeline(&self.elastic_forces_pipeline);
                    pass.dispatch_workgroups(elastic_x, slots_y, 1);
                }

                if push_x > 0 {
                    pass.set_pipeline(&self.push_forces_pipeline);
                    pass.dispatch_workgroups(push_x, slots_y, 1);
                }

                pass.set_pipeline(&self.second_half_kick_pipeline);
                pass.dispatch_workgroups(joint_x, slots_y, 1);

                pass.set_pipeline(&self.ground_collision_pipeline);
                pass.dispatch_workgroups(joint_x, slots_y, 1);
            }
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Read all joint positions back, partitioned by slot.
    pub fn read_all_positions(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<Vec<Vec3>> {
        let total = (self.num_slots * self.max_joints) as u64;
        if total == 0 {
            return vec![Vec::new(); self.num_slots as usize];
        }

        let raw: Vec<[f32; 4]> = read_buffer_typed(
            device,
            queue,
            &self.position_buffer,
            &self.staging_buffer,
            total * 16,
            "gpu readback",
        );

        (0..self.num_slots as usize)
            .map(|slot| {
                let offset = slot * self.max_joints as usize;
                let count = self.slot_joint_counts[slot] as usize;
                raw[offset..offset + count]
                    .iter()
                    .map(|v| Vec3::new(v[0], v[1], v[2]))
                    .collect()
            })
            .collect()
    }

    /// Convenience: read positions for a single-slot batch.
    pub fn read_positions(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<Vec3> {
        self.read_all_positions(device, queue).into_iter().next().unwrap()
    }

    /// Compute per-slot facts from the current GPU state. Reads back
    /// all positions and derives the facts on the CPU.
    pub fn read_facts(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<SlotFacts> {
        self.read_all_positions(device, queue)
            .iter()
            .map(|p| SlotFacts::from_positions(p))
            .collect()
    }

    pub fn num_slots(&self) -> u32 {
        self.num_slots
    }

    /// Read the per-slot frozen flag. A slot becomes frozen when the shader
    /// detects a speed above `params.speed_limit` (see `check_speed_limit_slot`
    /// in `physics.wgsl`); once set, kick/drift passes skip the slot, so it
    /// stops moving.
    pub fn read_frozen(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<bool> {
        let byte_size = (self.num_slots as u64) * 4;
        if byte_size == 0 {
            return Vec::new();
        }
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu frozen staging"),
            size: byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let raw: Vec<u32> = read_buffer_typed(
            device,
            queue,
            &self.frozen_buffer,
            &staging,
            byte_size,
            "gpu frozen readback",
        );
        raw.into_iter().map(|v| v != 0).collect()
    }
}

/// Copy a GPU buffer into a host-mappable staging buffer, map it, and
/// return the contents as a `Vec<T>`. Blocking on `device.poll(Wait)` —
/// callers must run on a thread that can spare a stall, which excludes
/// the wasm main thread (see `docs/gpu-compute-backend.md` browser
/// caveat).
fn read_buffer_typed<T: bytemuck::Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    src: &wgpu::Buffer,
    staging: &wgpu::Buffer,
    byte_size: u64,
    label: &str,
) -> Vec<T> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some(label),
    });
    encoder.copy_buffer_to_buffer(src, 0, staging, 0, byte_size);
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..byte_size);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();

    let data = slice.get_mapped_range();
    let out: Vec<T> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging.unmap();
    out
}

/// Run a whole generation of fabrics in one GPU dispatch. Each fabric
/// is stepped for `duration` of fabric time under the given `physics`,
/// then per-slot facts are read back. This is the core pattern for
/// GPU-accelerated evolution: build/mutate fabrics on the CPU, step
/// them in parallel on the GPU, evaluate fitness from the returned
/// facts on the CPU.
pub fn run_generation(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    fabrics: &[&Fabric],
    physics: &Physics,
    duration: crate::units::Seconds,
) -> Vec<SlotFacts> {
    let iterations = (duration.f32() / crate::Age::iteration_duration()) as u32;
    let batch = GpuBatch::parallelize(device, queue, fabrics, physics);
    batch.step(device, queue, iterations);
    batch.read_facts(device, queue)
}

/// Per-slot summary computed from joint positions after stepping.
#[derive(Clone, Debug)]
pub struct SlotFacts {
    pub centroid: Vec3,
    pub bounding_radius: f32,
    pub height: f32,
}

impl SlotFacts {
    pub fn from_positions(positions: &[Vec3]) -> Self {
        if positions.is_empty() {
            return Self {
                centroid: Vec3::ZERO,
                bounding_radius: 0.0,
                height: 0.0,
            };
        }
        let n = positions.len() as f32;
        let centroid = positions.iter().copied().sum::<Vec3>() / n;
        let bounding_radius = positions
            .iter()
            .map(|p| (*p - centroid).length())
            .fold(0.0f32, f32::max);
        let height = positions
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        Self {
            centroid,
            bounding_radius,
            height,
        }
    }
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
