//! Compiles WalkingGenome into FabricPlan for execution.
//!
//! The compiler translates the simplified genome representation into the
//! full FabricPlan DSL structures that can be built by FabricPlanExecutor.

use crate::build::dsl::animate_phase::{Actuator, ActuatorAttachment, AnimatePhase};
use crate::build::dsl::brick_dsl::{BrickName, BrickRole, FaceName};
use crate::build::dsl::build_phase::{BuildNode, BuildPhase, Chirality, ColumnStyle};
use crate::build::dsl::fabric_library::FabricName;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::settle_phase::SettlePhase;
use crate::build::dsl::shape_phase::ShapePhase;
use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::SurfaceCharacter;
use crate::fabric::FabricDimensions;
use crate::units::{Meters, Percent, Seconds};

use super::genome::{ActuationPattern, BrickType, WalkingGenome};

impl WalkingGenome {
    /// Compile the genome into a complete FabricPlan.
    pub fn to_fabric_plan(&self) -> FabricPlan {
        let build_phase = self.compile_build_phase();
        let animate_phase = self.compile_animate_phase();

        let dimensions = FabricDimensions::default()
            .with_altitude(Meters(1.0))
            .with_scale(Meters(self.structure.seed_scale));

        FabricPlan {
            name: FabricName::Triped, // TODO: Add Walking variant
            build_phase,
            shape_phase: ShapePhase {
                steps: Vec::new(),
                marks: Vec::new(),
                spacers: Vec::new(),
                joiners: Vec::new(),
                anchors: Vec::new(),
                step_index: 0,
                scale: dimensions.scale,
            },
            pretense_phase: PretensePhase {
                surface: Some(SurfaceCharacter::Sticky),
                ..PretensePhase::default()
            },
            fall_phase: FallPhase {
                seconds: Seconds(0.5),
            },
            settle_phase: Some(SettlePhase {
                seconds: Seconds(1.0),
            }),
            animate_phase: Some(animate_phase),
            dimensions,
        }
    }

    /// Compile the structural genome into a BuildPhase.
    fn compile_build_phase(&self) -> BuildPhase {
        let (seed_brick_name, seed_brick_role) = self.structure.seed_brick.to_brick_name_role();

        // Create face nodes for each branch
        let face_nodes: Vec<BuildNode> = self
            .structure
            .branches
            .iter()
            .map(|branch| {
                let face_name = self.structure.seed_brick.face_name(branch.seed_face);
                let alias = seed_brick_role.calls_it(face_name);

                // Create the column node if there are bricks in this branch
                let inner_node = if branch.column_count > 0 {
                    BuildNode::Column {
                        style: ColumnStyle::new(branch.column_count as usize, Chirality::Alternating),
                        scale: Percent(branch.scale * 100.0),
                        post_column_nodes: vec![BuildNode::Prism],
                    }
                } else {
                    BuildNode::Prism
                };

                BuildNode::Face {
                    alias,
                    node: Box::new(inner_node),
                }
            })
            .collect();

        // Create the root Hub node
        let root = BuildNode::Hub {
            brick_name: seed_brick_name,
            brick_role: seed_brick_role,
            rotation: 0,
            scale: Percent(100.0),
            face_nodes,
        };

        // Seed altitude: 1 meter above ground (in fabric units)
        let seed_altitude = 1.0 / self.structure.seed_scale;

        BuildPhase::new(root, seed_altitude)
    }

    /// Compile the actuation genome into an AnimatePhase.
    fn compile_animate_phase(&self) -> AnimatePhase {
        let act = &self.actuation;

        // For now, create a simple set of actuators based on branch count
        // Each branch gets surface actuators at its base
        let actuators = self.compile_actuators();

        AnimatePhase {
            period: Seconds(act.period),
            amplitude: Percent(act.amplitude),
            stiffness: Percent(act.stiffness),
            waveform: act.waveform.clone(),
            actuators,
        }
    }

    /// Compile actuators based on the actuation pattern.
    fn compile_actuators(&self) -> Vec<Actuator> {
        let mut actuators = Vec::new();
        let num_branches = self.structure.branches.len();

        // Create surface actuators for each branch
        // These anchor branch tips to the ground
        for (i, branch) in self.structure.branches.iter().enumerate() {
            let phase_offset = self.calculate_phase_offset(i, num_branches);

            // Create a path to the branch endpoint
            // Format: branch letter + X + column count + Z + local index
            let branch_letter = (b'A' + i as u8) as char;
            let depth = branch.column_count.max(1);
            let joint_path_str = format!("{}X{}Z0", branch_letter, depth);
            let joint_path: JointPath = joint_path_str.parse().unwrap_or_default();

            // Surface anchor point - spread around center based on branch index
            let angle = std::f32::consts::TAU * (i as f32) / (num_branches as f32);
            let radius = 0.5; // meters from center
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;

            actuators.push(Actuator {
                phase_offset: Percent(phase_offset * 100.0),
                attachment: ActuatorAttachment::ToSurface {
                    joint: joint_path,
                    point: (x, z),
                },
            });
        }

        actuators
    }

    /// Calculate phase offset for an actuator based on the pattern.
    fn calculate_phase_offset(&self, branch_index: usize, _total_branches: usize) -> f32 {
        match &self.actuation.pattern {
            ActuationPattern::Synchronized => 0.0,

            ActuationPattern::Alternating { shift } => {
                if branch_index % 2 == 0 {
                    0.0
                } else {
                    *shift
                }
            }

            ActuationPattern::Wave { wavelength } => {
                // Phase increases with branch index
                (branch_index as f32 / wavelength) % 1.0
            }

            ActuationPattern::PerBranch { offsets } => {
                offsets.get(branch_index).copied().unwrap_or(0.0)
            }
        }
    }
}

impl BrickType {
    /// Convert to the actual brick name and role for building.
    fn to_brick_name_role(&self) -> (BrickName, BrickRole) {
        match self {
            BrickType::SingleTwist => (BrickName::SingleTwistLeft, BrickRole::Seed(1)),
            BrickType::Omni => (BrickName::OmniSymmetrical, BrickRole::Seed(4)),
            BrickType::Torque => (BrickName::TorqueSymmetrical, BrickRole::Seed(4)),
        }
    }

    /// Get a face name for a given index.
    fn face_name(&self, index: usize) -> FaceName {
        match self {
            BrickType::SingleTwist => {
                if index == 0 {
                    FaceName::SingleTop
                } else {
                    FaceName::SingleBot
                }
            }
            BrickType::Omni => {
                // Omni has 8 faces, use the bottom ones for legs
                match index % 8 {
                    0 => FaceName::OmniBotX,
                    1 => FaceName::OmniBotY,
                    2 => FaceName::OmniBotZ,
                    3 => FaceName::OmniTopX,
                    4 => FaceName::OmniTopY,
                    5 => FaceName::OmniTopZ,
                    6 => FaceName::OmniTop,
                    _ => FaceName::OmniBot,
                }
            }
            BrickType::Torque => {
                // Torque has similar face layout to Omni
                match index % 8 {
                    0 => FaceName::OmniBotX,
                    1 => FaceName::OmniBotY,
                    2 => FaceName::OmniBotZ,
                    3 => FaceName::OmniTopX,
                    4 => FaceName::OmniTopY,
                    5 => FaceName::OmniTopZ,
                    6 => FaceName::OmniTop,
                    _ => FaceName::OmniBot,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_plan_executor::{FabricPlanExecutor, IterateResult};

    #[test]
    fn test_compile_default_genome() {
        let genome = WalkingGenome::default();
        let plan = genome.to_fabric_plan();

        // Verify the plan has expected structure
        assert!(plan.animate_phase.is_some());
        assert!(plan.settle_phase.is_some());
    }

    #[test]
    fn test_compile_minimal_genome() {
        let genome = WalkingGenome::minimal();
        let plan = genome.to_fabric_plan();

        // Minimal genome should still produce valid plan
        assert!(plan.animate_phase.is_some());
    }

    #[test]
    fn test_actuator_count_matches_branches() {
        let genome = WalkingGenome::default();
        let plan = genome.to_fabric_plan();

        let actuators = &plan.animate_phase.unwrap().actuators;
        assert_eq!(actuators.len(), genome.structure.branches.len());
    }

    #[test]
    fn test_fabric_plan_builds_without_excessive_speed() {
        use crate::build::dsl::fabric_plan_executor::ExecutorStage;

        let genome = WalkingGenome::default();
        let plan = genome.to_fabric_plan();

        let mut executor = FabricPlanExecutor::new_headless(plan);

        // Run up to 100,000 iterations (should complete building in much less)
        let max_iterations = 100_000;
        let mut completed = false;
        let mut last_stage = executor.stage().clone();

        for i in 0..max_iterations {
            // Track stage changes
            let current_stage = executor.stage().clone();
            if current_stage != last_stage {
                println!("Stage changed to {:?} at iteration {} ({:.3}s)",
                    current_stage, i, executor.fabric.age.as_duration().as_secs_f32());

                // Print diagnostics when transitioning to Falling
                if current_stage == ExecutorStage::Falling {
                    println!("\n=== Fabric state at transition to Falling ===");
                    println!("Physics surface: {:?}", executor.physics.surface);
                    println!("Physics drag: {:?}, viscosity: {:?}",
                        executor.physics.drag(), executor.physics.viscosity());

                    // Print joint positions (Y coordinate = altitude)
                    let min_y = executor.fabric.joints.values()
                        .map(|j| j.location.y)
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(0.0);
                    let max_y = executor.fabric.joints.values()
                        .map(|j| j.location.y)
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(0.0);
                    println!("Joint Y range: {:.4}m to {:.4}m", min_y, max_y);

                    // Print interval strain stats
                    let strains: Vec<f32> = executor.fabric.intervals.values()
                        .map(|interval| interval.strain)
                        .collect();
                    let min_strain = strains.iter().cloned()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(0.0);
                    let max_strain = strains.iter().cloned()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(0.0);
                    println!("Interval strain range: {:.6} to {:.6}", min_strain, max_strain);

                    // Print max velocity
                    let max_velocity = executor.fabric.joints.values()
                        .map(|j| j.velocity.length())
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(0.0);
                    println!("Max velocity before first iteration: {:.6} m/s", max_velocity);
                    println!("===\n");
                }

                last_stage = current_stage;
            }

            match executor.iterate() {
                IterateResult::Complete => {
                    completed = true;
                    println!("Completed at iteration {}", i);
                    break;
                }
                IterateResult::Continue => {
                    // Check if fabric is frozen (excessive speed detected)
                    if executor.fabric.frozen {
                        println!("Stage at freeze: {:?}", executor.stage());
                        println!("Fabric age: {:.3}s", executor.fabric.age.as_duration().as_secs_f32());
                        println!("Joints: {}, Intervals: {}",
                            executor.fabric.joints.len(), executor.fabric.intervals.len());
                        panic!("Fabric froze at iteration {} - excessive speed detected", i);
                    }
                }
            }
        }

        assert!(completed, "Fabric plan did not complete in {} iterations", max_iterations);
        assert!(!executor.fabric.frozen, "Fabric should not be frozen after completion");
    }
}
