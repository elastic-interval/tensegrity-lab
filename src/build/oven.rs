use crate::build::dsl::brick::BakedBrick;
use crate::build::dsl::brick::BrickPrototype;
use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::brick_library;
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::BAKING;
use crate::fabric::Fabric;
use crate::{Radio, StateChange};
use std::collections::HashMap;
use std::time::Duration;
use strum::IntoEnumIterator;

/// Bricks are considered done after this much fabric time
const BAKED_DURATION: Duration = Duration::from_secs(1);

/// Reorient the brick at this time so user can see it
const REORIENT_DURATION: Duration = Duration::from_millis(500);

/// Path to baked_bricks.rs (relative to project root)
const BAKED_BRICKS_PATH: &str = "src/build/dsl/brick_library/baked_bricks.rs";

/// Tolerance for face strain convergence
const STRAIN_TOLERANCE: f32 = 0.001;

struct TuningState {
    scale: f32,
    low_scale: Option<f32>,  // Scale that gave strain < target
    high_scale: Option<f32>, // Scale that gave strain > target
    iteration: usize,
}

impl TuningState {
    fn new(initial_scale: f32) -> Self {
        Self {
            scale: initial_scale,
            low_scale: None,
            high_scale: None,
            iteration: 0,
        }
    }
}

pub struct Oven {
    brick_names: Vec<BrickName>,
    current_index: usize,
    radio: Radio,
    baked_fabrics: Vec<Option<Fabric>>,
    reoriented: bool,
    tuning: TuningState,
}

impl Oven {
    pub fn new(radio: Radio) -> Self {
        let brick_names: Vec<BrickName> = BrickName::iter().collect();
        let baked_fabrics = vec![None; brick_names.len()];
        let initial_scale = brick_library::get_scale(brick_names[0]);

        Self {
            brick_names,
            current_index: 0,
            radio,
            baked_fabrics,
            reoriented: false,
            tuning: TuningState::new(initial_scale),
        }
    }

    pub fn current_brick_name(&self) -> BrickName {
        self.brick_names[self.current_index]
    }

    /// Check if the current brick is already baked
    fn current_is_baked(&self) -> bool {
        self.baked_fabrics[self.current_index].is_some()
    }

    /// Check if all bricks are baked
    fn all_baked(&self) -> bool {
        self.baked_fabrics.iter().all(|f| f.is_some())
    }

    /// Find the next unbaked brick index, if any
    fn next_unbaked_index(&self) -> Option<usize> {
        for i in 0..self.brick_names.len() {
            let index = (self.current_index + 1 + i) % self.brick_names.len();
            if self.baked_fabrics[index].is_none() {
                return Some(index);
            }
        }
        None
    }

    pub fn create_fresh_fabric(&self) -> Fabric {
        let prototype = brick_library::get_prototype(self.current_brick_name());
        let scaled = Self::scale_prototype(&prototype, self.tuning.scale);
        Fabric::from(scaled)
    }

    fn scale_prototype(proto: &BrickPrototype, scale: f32) -> BrickPrototype {
        let mut scaled = proto.clone();
        for push in &mut scaled.pushes {
            push.ideal *= scale;
        }
        for pull in &mut scaled.pulls {
            pull.ideal *= scale;
        }
        scaled
    }

    /// Get the fabric for the current brick - either baked or fresh
    fn current_fabric(&self) -> Fabric {
        if let Some(fabric) = &self.baked_fabrics[self.current_index] {
            fabric.clone()
        } else {
            self.create_fresh_fabric()
        }
    }

    pub fn next_brick(&mut self) -> Fabric {
        self.current_index = (self.current_index + 1) % self.brick_names.len();
        self.reoriented = false;
        self.tuning = TuningState::new(brick_library::get_scale(self.current_brick_name()));
        self.send_name_and_label();
        self.current_fabric()
    }

    /// Send fabric name and stage label for current brick
    fn send_name_and_label(&self) {
        StateChange::SetFabricName(format!("{}", self.current_brick_name())).send(&self.radio);
        self.send_stage_label();
    }

    /// Send the appropriate stage label based on baked state
    pub fn send_stage_label(&self) {
        let label = if self.current_is_baked() {
            if self.all_baked() { "All Baked" } else { "Baked" }
        } else {
            "Baking"
        };
        StateChange::SetStageLabel(label.to_string()).send(&self.radio);
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = BAKING;
    }

    fn average_face_strain(fabric: &Fabric) -> f32 {
        let strain_sum: f32 = fabric.faces.values().map(|face| face.strain(fabric)).sum();
        strain_sum / fabric.faces.len() as f32
    }

    fn compute_new_scale(&mut self, current_strain: f32) -> f32 {
        let target = BakedBrick::TARGET_FACE_STRAIN;

        // Update bounds based on current result
        if current_strain < target {
            // Strain too low - this scale is a lower bound, need higher scale
            self.tuning.low_scale = Some(self.tuning.scale);
        } else {
            // Strain too high - this scale is an upper bound, need lower scale
            self.tuning.high_scale = Some(self.tuning.scale);
        }

        // Use bisection if we have both bounds
        if let (Some(low), Some(high)) = (self.tuning.low_scale, self.tuning.high_scale) {
            return (low + high) / 2.0;
        }

        // Otherwise use proportional adjustment to find the other bound
        let ratio = target / current_strain.max(0.001);
        let clamped_ratio = ratio.clamp(0.5, 2.0);
        let damped_ratio = 1.0 + 0.5 * (clamped_ratio - 1.0);
        (self.tuning.scale * damped_ratio).clamp(0.1, 10.0)
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) -> Option<Fabric> {
        if self.current_is_baked() {
            return None;
        }

        for _ in 0..60 {
            context.fabric.iterate(context.physics);
        }

        if !self.reoriented && context.fabric.age.as_duration() >= REORIENT_DURATION {
            let prototype = brick_library::get_prototype(self.current_brick_name());
            let rotation = context.fabric.down_rotation(prototype.max_seed());
            context.fabric.apply_matrix4(rotation);
            let translation = context.fabric.centralize_translation(Some(0.0));
            context.fabric.apply_translation(translation);
            context.fabric.zero_velocities();
            self.reoriented = true;
        }

        if context.fabric.age.as_duration() >= BAKED_DURATION {
            let current_strain = Self::average_face_strain(&context.fabric);
            let error = (current_strain - BakedBrick::TARGET_FACE_STRAIN).abs();

            if error > STRAIN_TOLERANCE {
                let new_scale = self.compute_new_scale(current_strain);
                println!(
                    "Tuning {}: strain={:.4}, scale {:.4} -> {:.4}",
                    self.current_brick_name(),
                    current_strain,
                    self.tuning.scale,
                    new_scale
                );

                self.tuning.scale = new_scale;
                self.tuning.iteration += 1;
                self.reoriented = false;

                return Some(self.create_fresh_fabric());
            }

            let final_scale = self.tuning.scale;
            if self.tuning.iteration > 0 {
                println!(
                    "Tuned {} in {} iterations: scale={:.4}, strain={:.4}",
                    self.current_brick_name(),
                    self.tuning.iteration,
                    final_scale,
                    current_strain
                );
            }

            let brick_name = self.current_brick_name();
            let code = self.generate_baked_code(&context.fabric, final_scale);
            self.baked_fabrics[self.current_index] = Some(context.fabric.clone());
            self.export_brick(brick_name, &code);

            if let Some(next_index) = self.next_unbaked_index() {
                self.current_index = next_index;
                self.reoriented = false;
                self.tuning = TuningState::new(brick_library::get_scale(self.brick_names[next_index]));
                self.send_name_and_label();
                StateChange::RestartApproach.send(&self.radio);
                return Some(self.create_fresh_fabric());
            } else {
                self.send_stage_label();
            }
        }

        None
    }

    fn generate_baked_code(&self, fabric: &Fabric, scale: f32) -> String {
        let mut oriented = fabric.clone();
        let prototype = brick_library::get_prototype(self.current_brick_name());
        let rotation = oriented.down_rotation(prototype.max_seed());
        oriented.apply_matrix4(rotation);
        let translation = oriented.centralize_translation(Some(0.0));
        oriented.apply_translation(translation);

        // Get face center joints to exclude them
        let face_joints: Vec<usize> = oriented
            .faces
            .values()
            .map(|face| face.middle_joint(&oriented))
            .collect();

        // Build mapping from fabric joint index to baked joint index
        let mut fabric_to_baked: HashMap<usize, usize> = HashMap::new();
        let mut baked_index = 0;
        let joint_incidents = oriented.joint_incidents();
        for incident in &joint_incidents {
            if !face_joints.contains(&incident.index) {
                fabric_to_baked.insert(incident.index, baked_index);
                baked_index += 1;
            }
        }

        // Build joints using helper function format
        let joints_str: Vec<String> = joint_incidents
            .iter()
            .filter(|inc| !face_joints.contains(&inc.index))
            .map(|inc| {
                let loc = inc.location;
                format!("            joint({:.4}, {:.4}, {:.4}),", loc.x, loc.y, loc.z)
            })
            .collect();

        // Build pushes and pulls using helper function format
        let mut pushes: Vec<String> = Vec::new();
        let mut pulls: Vec<String> = Vec::new();

        for interval in oriented.interval_values() {
            if interval.role == Role::FaceRadial {
                continue;
            }
            let alpha = fabric_to_baked.get(&interval.alpha_index);
            let omega = fabric_to_baked.get(&interval.omega_index);
            if let (Some(&a), Some(&o)) = (alpha, omega) {
                if interval.role == Role::Pushing {
                    pushes.push(format!("            push({}, {}, {:.4}),", a, o, interval.strain));
                } else {
                    pulls.push(format!("            pull({}, {}, {:.4}),", a, o, interval.strain));
                }
            }
        }

        // Combine intervals
        let mut intervals: Vec<String> = pushes;
        intervals.extend(pulls);

        format!(
            "        scale: {:.4},
        joints: vec![
{}
        ],
        intervals: vec![
{}
        ],",
            scale,
            joints_str.join("\n"),
            intervals.join("\n"),
        )
    }

    fn function_name(brick_name: BrickName) -> &'static str {
        match brick_name {
            BrickName::SingleLeftBrick => "single_left_baked",
            BrickName::SingleRightBrick => "single_right_baked",
            BrickName::OmniBrick => "omni_baked",
            BrickName::TorqueBrick => "torque_baked",
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_brick(&self, brick_name: BrickName, baked_code: &str) {
        let source = match std::fs::read_to_string(BAKED_BRICKS_PATH) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", BAKED_BRICKS_PATH, e);
                return;
            }
        };

        let func_name = Self::function_name(brick_name);
        let Some(new_source) = Self::substitute_baked_section(&source, func_name, baked_code) else {
            eprintln!("Failed to find {} in {}", func_name, BAKED_BRICKS_PATH);
            return;
        };

        match std::fs::write(BAKED_BRICKS_PATH, &new_source) {
            Ok(_) => println!("=== Updated {} in {} ===", func_name, BAKED_BRICKS_PATH),
            Err(e) => eprintln!("Failed to write {}: {}", BAKED_BRICKS_PATH, e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn export_brick(&self, _brick_name: BrickName, _baked_code: &str) {}

    fn substitute_baked_section(source: &str, func_name: &str, replacement: &str) -> Option<String> {
        // Find the function
        let func_start = source.find(&format!("fn {}()", func_name))?;

        // Find "scale:" after the function start
        let after_func = &source[func_start..];
        let scale_offset = after_func.find("scale:")?;
        let scale_start = func_start + scale_offset;

        // Find the closing of intervals vec ("],") followed by faces
        let after_scale = &source[scale_start..];
        let faces_offset = after_scale.find("faces:")?;
        let faces_start = scale_start + faces_offset;

        let mut new_source = String::with_capacity(source.len());
        new_source.push_str(&source[..scale_start]);
        new_source.push_str(replacement);
        new_source.push_str("\n        ");
        new_source.push_str(&source[faces_start..]);

        Some(new_source)
    }
}
