//! `GpuBatch`: the one GPU-side entry point users need.
//!
//! Phase 2 of the GPU compute backend port (see `docs/gpu-compute-backend.md`).
//!
//! The usage is simple: build a `Fabric` on the CPU however you like
//! (DSL, brick assembly, hand-wiring), then call
//! `GpuBatch::parallelize(&device, &queue, &[&fabric], &physics)` to
//! upload it to the GPU. Call `step(n)` to advance it `n` iterations,
//! and `read_positions()` to copy the current joint positions back.
//!
//! Phase 2 supports single-fabric batches only. Phase 4 will lift the
//! `slot_count == 1` assertion and generalize the buffer layout to
//! pad-to-max over N slots.

use std::collections::HashMap;

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::fabric::interval::{Role, Span};
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, JointKey};
use crate::physics_gpu::params::{GpuPhysicsConfig, PhysicsParams};
use crate::units::{Meters, Unit};

/// All per-fabric data extracted on the CPU before upload. One of these
/// is produced per input `Fabric` during `parallelize`.
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
    joint_index_by_key: HashMap<JointKey, u32>,
}

impl FabricSnapshot {
    fn from_fabric(fabric: &Fabric, physics: &Physics) -> Self {
        let mut positions = Vec::with_capacity(fabric.joints.len());
        let mut velocities = Vec::with_capacity(fabric.joints.len());
        let mut joint_index_by_key = HashMap::with_capacity(fabric.joints.len());

        for (idx, (key, joint)) in fabric.joints.iter().enumerate() {
            positions.push([joint.location.x, joint.location.y, joint.location.z, 1.0]);
            velocities.push([joint.velocity.x, joint.velocity.y, joint.velocity.z, 0.0]);
            joint_index_by_key.insert(key, idx as u32);
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
            let alpha = *joint_index_by_key
                .get(&interval.alpha_key)
                .expect("interval alpha_key not present in joint map");
            let omega = *joint_index_by_key
                .get(&interval.omega_key)
                .expect("interval omega_key not present in joint map");

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
            joint_index_by_key,
        }
    }
}

/// Resolve a span to a concrete `Meters`. Approaching spans are resolved
/// to the current interpolated length at the fabric's current age,
/// so mid-flight fabrics parallelize to the state a CPU observer would
/// currently see. Measuring spans (vulcanize bow ties) are unsupported
/// and panic — the build pipeline should have resolved them by the time
/// the fabric reaches the GPU.
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
            panic!(
                "physics_gpu::parallelize: Span::Measuring is not supported; \
                 the build must resolve vulcanize bow ties to Fixed spans first"
            );
        }
    }
}

pub struct GpuBatch {
    joint_bind_group: wgpu::BindGroup,
    elastic_bind_group: wgpu::BindGroup,
    params_bind_group: wgpu::BindGroup,
    push_bind_group: wgpu::BindGroup,

    half_kick_pipeline: wgpu::ComputePipeline,
    elastic_forces_pipeline: wgpu::ComputePipeline,
    push_forces_pipeline: wgpu::ComputePipeline,
    second_half_kick_pipeline: wgpu::ComputePipeline,
    ground_collision_pipeline: wgpu::ComputePipeline,

    position_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    velocity_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    force_buffers: [wgpu::Buffer; 3],
    #[allow(dead_code)]
    mass_buffer: wgpu::Buffer,
    frozen_buffer: wgpu::Buffer,

    #[allow(dead_code)]
    elastic_buffers: ElasticBuffers,
    #[allow(dead_code)]
    push_buffers: PushBuffers,

    #[allow(dead_code)]
    params_buffer: wgpu::Buffer,
    positions_staging_buffer: wgpu::Buffer,
    frozen_staging_buffer: wgpu::Buffer,

    num_joints: u32,
    num_elastic: u32,
    num_push: u32,

    #[allow(dead_code)]
    joint_index_by_key: HashMap<JointKey, u32>,
}

#[allow(dead_code)]
struct ElasticBuffers {
    alpha: wgpu::Buffer,
    omega: wgpu::Buffer,
    ideal: wgpu::Buffer,
    k: wgpu::Buffer,
    linear_density: wgpu::Buffer,
}

#[allow(dead_code)]
struct PushBuffers {
    alpha: wgpu::Buffer,
    omega: wgpu::Buffer,
    ideal: wgpu::Buffer,
    k: wgpu::Buffer,
    linear_density: wgpu::Buffer,
}

impl GpuBatch {
    /// Upload a collection of CPU-built fabrics to the GPU as one batch.
    /// Phase 2 supports `fabrics.len() == 1`; phase 4 will generalize to N.
    pub fn parallelize(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fabrics: &[&Fabric],
        physics: &Physics,
    ) -> Self {
        assert_eq!(
            fabrics.len(),
            1,
            "phase 2: GpuBatch::parallelize supports exactly one fabric; \
             phase 4 will lift this assertion"
        );
        let fabric = fabrics[0];

        let snapshot = FabricSnapshot::from_fabric(fabric, physics);
        let config = GpuPhysicsConfig::from_fabric(fabric, physics);

        let num_joints = snapshot.positions.len() as u32;
        let num_elastic = snapshot.elastic_alpha.len() as u32;
        let num_push = snapshot.push_alpha.len() as u32;

        assert!(num_joints > 0, "GpuBatch: fabric has no joints");

        // ---- Joint state buffers ----
        let writable = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let position_usage = writable | wgpu::BufferUsages::COPY_SRC;

        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu positions"),
            contents: bytemuck::cast_slice(&snapshot.positions),
            usage: position_usage,
        });
        let velocity_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu velocities"),
            contents: bytemuck::cast_slice(&snapshot.velocities),
            usage: writable,
        });
        let force_buffers = make_force_buffers(device, num_joints);

        let ambient_i32 = (config.ambient_mass * 1e4) as i32;
        let mass_init = vec![ambient_i32; num_joints as usize];
        let mass_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu masses"),
            contents: bytemuck::cast_slice(&mass_init),
            usage: writable,
        });

        let frozen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu frozen"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        // ---- Elastic interval buffers ----
        let elastic_buffers = ElasticBuffers {
            alpha: make_u32_buffer(device, "elastic_alpha", &snapshot.elastic_alpha),
            omega: make_u32_buffer(device, "elastic_omega", &snapshot.elastic_omega),
            ideal: make_f32_buffer(device, "elastic_ideal", &snapshot.elastic_ideal),
            k: make_f32_buffer(device, "elastic_k", &snapshot.elastic_k),
            linear_density: make_f32_buffer(
                device,
                "elastic_linear_density",
                &snapshot.elastic_linear_density,
            ),
        };

        // ---- Push interval buffers ----
        let push_buffers = PushBuffers {
            alpha: make_u32_buffer(device, "push_alpha", &snapshot.push_alpha),
            omega: make_u32_buffer(device, "push_omega", &snapshot.push_omega),
            ideal: make_f32_buffer(device, "push_ideal", &snapshot.push_ideal),
            k: make_f32_buffer(device, "push_k", &snapshot.push_k),
            linear_density: make_f32_buffer(
                device,
                "push_linear_density",
                &snapshot.push_linear_density,
            ),
        };

        // ---- Params uniform ----
        let mut params = PhysicsParams::from_config(&config);
        params.num_joints = num_joints;
        params.num_elastic = num_elastic;
        params.num_push = num_push;

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ---- Staging ----
        let positions_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physics_gpu positions staging"),
            size: (num_joints as u64) * 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frozen_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("physics_gpu frozen staging"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- Bind group layouts ----
        let joint_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("physics_gpu joint BGL"),
            entries: &[
                storage_entry(0, false),
                storage_entry(1, false),
                storage_entry(2, false),
                storage_entry(3, false),
                storage_entry(4, false),
                storage_entry(5, false),
                storage_entry(6, false),
            ],
        });
        let elastic_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("physics_gpu elastic BGL"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, true),
            ],
        });
        let params_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("physics_gpu params BGL"),
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
            label: Some("physics_gpu push BGL"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, true),
            ],
        });

        // ---- Bind groups ----
        let joint_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("physics_gpu joint BG"),
            layout: &joint_bgl,
            entries: &[
                binding(0, &position_buffer),
                binding(1, &velocity_buffer),
                binding(2, &force_buffers[0]),
                binding(3, &force_buffers[1]),
                binding(4, &force_buffers[2]),
                binding(5, &mass_buffer),
                binding(6, &frozen_buffer),
            ],
        });
        let elastic_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("physics_gpu elastic BG"),
            layout: &elastic_bgl,
            entries: &[
                binding(0, &elastic_buffers.alpha),
                binding(1, &elastic_buffers.omega),
                binding(2, &elastic_buffers.ideal),
                binding(3, &elastic_buffers.k),
                binding(4, &elastic_buffers.linear_density),
            ],
        });
        let params_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("physics_gpu params BG"),
            layout: &params_bgl,
            entries: &[binding(0, &params_buffer)],
        });
        let push_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("physics_gpu push BG"),
            layout: &push_bgl,
            entries: &[
                binding(0, &push_buffers.alpha),
                binding(1, &push_buffers.omega),
                binding(2, &push_buffers.ideal),
                binding(3, &push_buffers.k),
                binding(4, &push_buffers.linear_density),
            ],
        });

        // ---- Pipelines ----
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("physics_gpu pipeline layout"),
            bind_group_layouts: &[&joint_bgl, &elastic_bgl, &params_bgl, &push_bgl],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("physics_gpu shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/physics.wgsl").into()),
        });

        let make_pipeline = |entry: &'static str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let _ = queue; // no early writes needed; everything uploaded via create_buffer_init

        Self {
            joint_bind_group,
            elastic_bind_group,
            params_bind_group,
            push_bind_group,
            half_kick_pipeline: make_pipeline("half_kick_and_drift"),
            elastic_forces_pipeline: make_pipeline("elastic_forces"),
            push_forces_pipeline: make_pipeline("push_forces"),
            second_half_kick_pipeline: make_pipeline("second_half_kick"),
            ground_collision_pipeline: make_pipeline("ground_collision"),
            position_buffer,
            velocity_buffer,
            force_buffers,
            mass_buffer,
            frozen_buffer,
            elastic_buffers,
            push_buffers,
            params_buffer,
            positions_staging_buffer,
            frozen_staging_buffer,
            num_joints,
            num_elastic,
            num_push,
            joint_index_by_key: snapshot.joint_index_by_key,
        }
    }

    /// Advance the batch by `iterations` physics ticks. All slots advance
    /// in lockstep (phase 2: one slot).
    pub fn step(&self, device: &wgpu::Device, queue: &wgpu::Queue, iterations: u32) {
        if iterations == 0 || self.num_joints == 0 {
            return;
        }

        let joint_groups = (self.num_joints + 63) / 64;
        let elastic_groups = if self.num_elastic > 0 {
            (self.num_elastic + 63) / 64
        } else {
            0
        };
        let push_groups = if self.num_push > 0 {
            (self.num_push + 63) / 64
        } else {
            0
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("physics_gpu step encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("physics_gpu step pass"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &self.joint_bind_group, &[]);
            pass.set_bind_group(1, &self.elastic_bind_group, &[]);
            pass.set_bind_group(2, &self.params_bind_group, &[]);
            pass.set_bind_group(3, &self.push_bind_group, &[]);

            for _ in 0..iterations {
                pass.set_pipeline(&self.half_kick_pipeline);
                pass.dispatch_workgroups(joint_groups, 1, 1);

                if elastic_groups > 0 {
                    pass.set_pipeline(&self.elastic_forces_pipeline);
                    pass.dispatch_workgroups(elastic_groups, 1, 1);
                }

                if push_groups > 0 {
                    pass.set_pipeline(&self.push_forces_pipeline);
                    pass.dispatch_workgroups(push_groups, 1, 1);
                }

                pass.set_pipeline(&self.second_half_kick_pipeline);
                pass.dispatch_workgroups(joint_groups, 1, 1);

                pass.set_pipeline(&self.ground_collision_pipeline);
                pass.dispatch_workgroups(joint_groups, 1, 1);
            }
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Read all joint positions back from the GPU as a `Vec<Vec3>`.
    /// Blocks until the copy completes.
    pub fn read_positions(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<Vec3> {
        if self.num_joints == 0 {
            return Vec::new();
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("physics_gpu readback encoder"),
        });
        encoder.copy_buffer_to_buffer(
            &self.position_buffer,
            0,
            &self.positions_staging_buffer,
            0,
            (self.num_joints as u64) * 16,
        );
        queue.submit(Some(encoder.finish()));

        let slice = self
            .positions_staging_buffer
            .slice(..(self.num_joints as u64 * 16));
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
        let raw: Vec<[f32; 4]> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.positions_staging_buffer.unmap();

        raw.into_iter().map(|v| Vec3::new(v[0], v[1], v[2])).collect()
    }

    /// True if the GPU has tripped the per-slot speed-limit freeze since
    /// the last reset.
    pub fn read_frozen(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("physics_gpu frozen readback"),
        });
        encoder.copy_buffer_to_buffer(&self.frozen_buffer, 0, &self.frozen_staging_buffer, 0, 4);
        queue.submit(Some(encoder.finish()));

        let slice = self.frozen_staging_buffer.slice(..);
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
        let value: u32 = *bytemuck::from_bytes(&data);
        drop(data);
        self.frozen_staging_buffer.unmap();
        value != 0
    }

    #[allow(dead_code)]
    pub fn num_joints(&self) -> u32 {
        self.num_joints
    }

    #[allow(dead_code)]
    pub fn num_elastic(&self) -> u32 {
        self.num_elastic
    }

    #[allow(dead_code)]
    pub fn num_push(&self) -> u32 {
        self.num_push
    }
}

fn make_u32_buffer(device: &wgpu::Device, label: &str, data: &[u32]) -> wgpu::Buffer {
    if data.is_empty() {
        return device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::STORAGE,
        });
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE,
    })
}

fn make_f32_buffer(device: &wgpu::Device, label: &str, data: &[f32]) -> wgpu::Buffer {
    if data.is_empty() {
        return device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&[0.0f32]),
            usage: wgpu::BufferUsages::STORAGE,
        });
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE,
    })
}

fn make_force_buffers(device: &wgpu::Device, num_joints: u32) -> [wgpu::Buffer; 3] {
    let zeros = vec![0i32; num_joints as usize];
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    [
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu force_x"),
            contents: bytemuck::cast_slice(&zeros),
            usage,
        }),
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu force_y"),
            contents: bytemuck::cast_slice(&zeros),
            usage,
        }),
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("physics_gpu force_z"),
            contents: bytemuck::cast_slice(&zeros),
            usage,
        }),
    ]
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

fn binding(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}
