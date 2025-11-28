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

/// Path to brick_library directory (relative to project root)
const BRICK_LIBRARY_DIR: &str = "src/build/dsl/brick_library";

pub struct Oven {
    brick_names: Vec<BrickName>,
    current_index: usize,
    radio: Radio,
    baked_fabrics: Vec<Option<Fabric>>,
    reoriented: bool,
}

impl Oven {
    pub fn new(radio: Radio) -> Self {
        let brick_names: Vec<BrickName> = BrickName::iter().collect();
        let baked_fabrics = vec![None; brick_names.len()];

        Self {
            brick_names,
            current_index: 0,
            radio,
            baked_fabrics,
            reoriented: false,
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

    /// Create a fresh fabric from the current brick's prototype
    pub fn create_fresh_fabric(&self) -> Fabric {
        let prototype = brick_library::get_prototype(self.current_brick_name());
        Fabric::from(prototype)
    }

    /// Get the fabric for the current brick - either baked or fresh
    fn current_fabric(&self) -> Fabric {
        if let Some(fabric) = &self.baked_fabrics[self.current_index] {
            fabric.clone()
        } else {
            self.create_fresh_fabric()
        }
    }

    /// Cycle to the next brick (manual) - only cycles through baked bricks when all are done
    pub fn next_brick(&mut self) -> Fabric {
        self.current_index = (self.current_index + 1) % self.brick_names.len();
        self.reoriented = false;
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

    /// Iterate the oven. Returns Some(fabric) if we need to switch to a new brick.
    pub fn iterate(&mut self, context: &mut CrucibleContext) -> Option<Fabric> {
        // Skip iterations if current brick is already baked
        if self.current_is_baked() {
            return None;
        }

        for _ in 0..60 {
            context.fabric.iterate(context.physics);
        }

        // Reorient at 0.5 seconds so user can see the final orientation
        if !self.reoriented && context.fabric.age.as_duration() >= REORIENT_DURATION {
            let prototype = brick_library::get_prototype(self.current_brick_name());
            let rotation = context.fabric.down_rotation(prototype.max_seed());
            context.fabric.apply_matrix4(rotation);
            let translation = context.fabric.centralize_translation(Some(0.0));
            context.fabric.apply_translation(translation);
            context.fabric.zero_velocities();
            self.reoriented = true;
        }

        // Check if baked (1 second of fabric time)
        if context.fabric.age.as_duration() >= BAKED_DURATION {
            let brick_name = self.current_brick_name();
            let code = self.generate_baked_code(&context.fabric);

            // Store baked fabric
            self.baked_fabrics[self.current_index] = Some(context.fabric.clone());

            // Auto-export
            self.export_brick(brick_name, &code);

            // Auto-cycle to next unbaked brick if any
            if let Some(next_index) = self.next_unbaked_index() {
                self.current_index = next_index;
                self.reoriented = false;
                self.send_name_and_label();
                return Some(self.create_fresh_fabric());
            } else {
                // All done
                self.send_stage_label();
            }
        }

        None
    }

    /// Generate the baked code string from the current fabric state
    fn generate_baked_code(&self, fabric: &Fabric) -> String {
        // Orient and centralize the fabric for baked output
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

        // Build joints lines
        let joints_str: Vec<String> = joint_incidents
            .iter()
            .filter(|inc| !face_joints.contains(&inc.index))
            .map(|inc| {
                let loc = inc.location;
                format!("            ({:.4}, {:.4}, {:.4}),", loc.x, loc.y, loc.z)
            })
            .collect();

        // Build pushes and pulls
        let mut pushes: Vec<String> = Vec::new();
        let mut pulls: Vec<String> = Vec::new();

        for interval in oriented.interval_values() {
            if interval.role == Role::FaceRadial {
                continue;
            }
            let alpha = fabric_to_baked.get(&interval.alpha_index);
            let omega = fabric_to_baked.get(&interval.omega_index);
            if let (Some(&a), Some(&o)) = (alpha, omega) {
                let entry = format!("            ({}, {}, {:.4}),", a, o, interval.strain);
                if interval.role == Role::Pushing {
                    pushes.push(entry);
                } else {
                    pulls.push(entry);
                }
            }
        }

        // Format pushes and pulls - each on its own line
        let pushes_str = if pushes.is_empty() {
            String::new()
        } else {
            format!("\n{}\n        ", pushes.join("\n"))
        };
        let pulls_str = if pulls.is_empty() {
            String::new()
        } else {
            format!("\n{}\n        ", pulls.join("\n"))
        };

        format!(
            ".baked()
        .joints([
{}
        ])
        .pushes([{}])
        .pulls([{}])
        .build()",
            joints_str.join("\n"),
            pushes_str,
            pulls_str,
        )
    }

    /// Get the source file path for a brick
    fn brick_file_path(brick_name: BrickName) -> String {
        let file_name = match brick_name {
            BrickName::SingleLeftBrick => "single_left",
            BrickName::SingleRightBrick => "single_right",
            BrickName::OmniBrick => "omni",
            BrickName::TorqueBrick => "torque",
        };
        format!("{}/{}.rs", BRICK_LIBRARY_DIR, file_name)
    }

    /// Export baked brick by substituting directly into the brick's source file
    /// Only works in native builds (not WASM)
    #[cfg(not(target_arch = "wasm32"))]
    fn export_brick(&self, brick_name: BrickName, baked_code: &str) {
        let file_path = Self::brick_file_path(brick_name);

        // Read the current source file
        let source = match std::fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", file_path, e);
                return;
            }
        };

        // Find the brick and replace between .baked() and .build()
        let Some(new_source) = Self::substitute_baked_section(&source, baked_code) else {
            eprintln!("Failed to find baked section in {}", file_path);
            return;
        };

        // Write back
        match std::fs::write(&file_path, &new_source) {
            Ok(_) => {
                println!("=== Updated {} ===", file_path);
            }
            Err(e) => {
                eprintln!("Failed to write {}: {}", file_path, e);
            }
        }
    }

    /// WASM stub - does nothing
    #[cfg(target_arch = "wasm32")]
    fn export_brick(&self, _brick_name: BrickName, _baked_code: &str) {
        // Cannot write to filesystem in WASM
    }

    /// Replace the section from .baked() to .build() (inclusive) in a brick source file
    fn substitute_baked_section(source: &str, replacement: &str) -> Option<String> {
        // Find .baked() in the source
        let baked_start = source.find(".baked()")?;

        // Find .build() after .baked()
        let after_baked = &source[baked_start..];
        let build_offset = after_baked.find(".build()")?;
        let build_end = baked_start + build_offset + ".build()".len();

        // Construct the new source
        let mut new_source = String::with_capacity(source.len());
        new_source.push_str(&source[..baked_start]);
        new_source.push_str(replacement);
        new_source.push_str(&source[build_end..]);

        Some(new_source)
    }
}
