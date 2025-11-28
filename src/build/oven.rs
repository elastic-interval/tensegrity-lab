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

/// Path to brick_library.rs source file (relative to project root)
const BRICK_LIBRARY_PATH: &str = "src/build/dsl/brick_library.rs";

pub struct Oven {
    brick_names: Vec<BrickName>,
    current_index: usize,
    radio: Radio,
    baked_code: Option<String>,
}

impl Oven {
    pub fn new(radio: Radio) -> Self {
        let brick_names: Vec<BrickName> = BrickName::iter().collect();

        Self {
            brick_names,
            current_index: 0,
            radio,
            baked_code: None,
        }
    }

    pub fn current_brick_name(&self) -> BrickName {
        self.brick_names[self.current_index]
    }

    /// Create a fresh fabric from the current brick's prototype
    pub fn create_fresh_fabric(&self) -> Fabric {
        let prototype = brick_library::get_prototype(self.current_brick_name());
        Fabric::from(prototype)
    }

    /// Cycle to the next brick and return a fresh fabric for it
    pub fn next_brick(&mut self) -> Fabric {
        self.current_index = (self.current_index + 1) % self.brick_names.len();
        self.baked_code = None;
        StateChange::SetFabricName(format!("{}", self.current_brick_name())).send(&self.radio);
        self.send_stage_label();
        self.create_fresh_fabric()
    }

    /// Send the appropriate stage label based on baked state
    pub fn send_stage_label(&self) {
        let label = if self.baked_code.is_some() { "Baked" } else { "Baking" };
        StateChange::SetStageLabel(label.to_string()).send(&self.radio);
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = BAKING;
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Skip iterations if already baked
        if self.baked_code.is_none() {
            for _ in 0..60 {
                context.fabric.iterate(context.physics);
            }

            // Check if baked (1 second of fabric time)
            if context.fabric.age.as_duration() >= BAKED_DURATION {
                self.baked_code = Some(self.generate_baked_code(&context.fabric));
                self.send_stage_label();
            }
        }
    }

    /// Generate the baked code string from the current fabric state
    fn generate_baked_code(&self, fabric: &Fabric) -> String {
        // Get face center joints to exclude them
        let face_joints: Vec<usize> = fabric
            .faces
            .values()
            .map(|face| face.middle_joint(fabric))
            .collect();

        // Build mapping from fabric joint index to baked joint index
        let mut fabric_to_baked: HashMap<usize, usize> = HashMap::new();
        let mut baked_index = 0;
        let joint_incidents = fabric.joint_incidents();
        for incident in &joint_incidents {
            if !face_joints.contains(&incident.index) {
                fabric_to_baked.insert(incident.index, baked_index);
                baked_index += 1;
            }
        }

        // Build joints string
        let joints_str: Vec<String> = joint_incidents
            .iter()
            .filter(|inc| !face_joints.contains(&inc.index))
            .map(|inc| {
                let loc = inc.location;
                format!("            ({:.4}, {:.4}, {:.4}),", loc.x, loc.y, loc.z)
            })
            .collect();

        // Build pushes and pulls strings
        let mut pushes: Vec<String> = Vec::new();
        let mut pulls: Vec<String> = Vec::new();

        for interval in fabric.interval_values() {
            if interval.role == Role::FaceRadial {
                continue;
            }
            let alpha = fabric_to_baked.get(&interval.alpha_index);
            let omega = fabric_to_baked.get(&interval.omega_index);
            if let (Some(&a), Some(&o)) = (alpha, omega) {
                let entry = format!("({}, {}, {:.4})", a, o, interval.strain);
                if interval.role == Role::Pushing {
                    pushes.push(entry);
                } else {
                    pulls.push(entry);
                }
            }
        }

        format!(
            r#".baked()
        .joints([
{}
        ])
        .pushes([{}])
        .pulls([{}])
        .build()"#,
            joints_str.join("\n"),
            pushes.join(", "),
            pulls.join(", "),
        )
    }

    /// Export baked data by substituting directly into brick_library.rs source
    /// Only works in native builds (not WASM)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_baked_data(&self) {
        let Some(baked_code) = &self.baked_code else {
            eprintln!("Cannot export: brick not yet baked");
            return;
        };

        let brick_name = self.current_brick_name();

        // Read the current source file
        let source = match std::fs::read_to_string(BRICK_LIBRARY_PATH) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", BRICK_LIBRARY_PATH, e);
                return;
            }
        };

        // Find the brick and replace between .baked() and .build()
        let Some(new_source) = Self::substitute_baked_section(&source, brick_name, baked_code) else {
            eprintln!("Failed to find baked section for {:?} in source", brick_name);
            return;
        };

        // Write back
        match std::fs::write(BRICK_LIBRARY_PATH, &new_source) {
            Ok(_) => {
                println!("=== Updated {:?} in {} ===", brick_name, BRICK_LIBRARY_PATH);
            }
            Err(e) => {
                eprintln!("Failed to write {}: {}", BRICK_LIBRARY_PATH, e);
            }
        }
    }

    /// WASM stub - does nothing
    #[cfg(target_arch = "wasm32")]
    pub fn export_baked_data(&self) {
        // Cannot write to filesystem in WASM
    }

    /// Find the brick by name and replace the section from .baked() to .build() (inclusive)
    fn substitute_baked_section(source: &str, brick_name: BrickName, replacement: &str) -> Option<String> {
        // Find the proto call: "proto(BrickName, " - using Debug format for enum variant
        let proto_pattern = format!("proto({:?},", brick_name);
        let proto_start = source.find(&proto_pattern)?;

        // Find .baked() after the proto call
        let after_proto = &source[proto_start..];
        let baked_offset = after_proto.find(".baked()")?;
        let baked_start = proto_start + baked_offset;

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
