use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::brick_library;
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::BAKING;
use crate::fabric::Fabric;
use crate::{Radio, StateChange};
use std::collections::HashMap;
use strum::IntoEnumIterator;

pub struct Oven {
    brick_names: Vec<BrickName>,
    current_index: usize,
    radio: Radio,
}

impl Oven {
    pub fn new(radio: Radio) -> Self {
        let brick_names: Vec<BrickName> = BrickName::iter().collect();

        Self {
            brick_names,
            current_index: 0,
            radio,
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
        StateChange::SetFabricName(format!("{}", self.current_brick_name())).send(&self.radio);
        self.create_fresh_fabric()
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = BAKING;
    }

    pub fn iterate(&self, context: &mut CrucibleContext) {
        for _ in 0..10 {  // Nominal value, outer loop adjusts dynamically
            context.fabric.iterate(context.physics);
        }
    }

    /// Export baked data for the current brick
    pub fn export_baked_data(&self, fabric: &Fabric) {
        let brick_name = self.current_brick_name();

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

        // Get the function name for this brick
        let func_name = match brick_name {
            BrickName::SingleRightBrick => "single_right",
            BrickName::SingleLeftBrick => "single_left",
            BrickName::OmniBrick => "omni",
            BrickName::TorqueBrick => "torque",
        };

        // Generate the replacement code
        let replacement = format!(
            r#"        .baked()
        .joints([
{}
        ])
        .pushes([{}])
        .pulls([{}])"#,
            joints_str.join("\n"),
            pushes.join(", "),
            pulls.join(", "),
        );

        // Write to file (overwrites previous)
        let filename = format!("baked_{}.txt", func_name);
        match std::fs::write(&filename, &replacement) {
            Ok(_) => {
                println!("=== Exported baked data for {:?} ===", brick_name);
                println!("Age: {}", fabric.age);
                println!("File: {}", filename);
                println!();
                println!("{}", replacement);
                println!();
            }
            Err(e) => {
                eprintln!("Failed to write baked data: {}", e);
            }
        }
    }
}
