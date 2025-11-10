use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::Physics;
use crate::LabEvent::DumpCSV;
use crate::units::{Seconds, MOMENT};
use crate::Radio;

#[derive(Clone, Debug, PartialEq, Copy)]
enum Stage {
    Start,
    Slacken,
    Pretensing,
    CreateMuscles,
    Pretenst,
}

#[derive(Clone)]
pub struct Pretenser {
    pub pretense_phase: PretensePhase,
    pub physics: Physics,
    stage: Stage,
    seconds_to_pretense: Seconds,
    radio: Radio,
}

impl Pretenser {
    pub fn new(pretense_phase: PretensePhase, radio: &Radio) -> Self {
        let pretenst = pretense_phase.pretenst.unwrap_or(AIR_GRAVITY.pretenst);
        let surface_character = pretense_phase.surface_character;
        let stiffness = pretense_phase.stiffness.unwrap_or(AIR_GRAVITY.stiffness);
        // Viscosity and drag are percentages of the default values
        let viscosity = pretense_phase.viscosity
            .map(|percent| AIR_GRAVITY.viscosity * percent / 100.0)
            .unwrap_or(AIR_GRAVITY.viscosity);
        let drag = pretense_phase.drag
            .map(|percent| AIR_GRAVITY.drag * percent / 100.0)
            .unwrap_or(AIR_GRAVITY.drag);
        let physics = Physics {
            pretenst,
            surface_character,
            stiffness,
            viscosity,
            drag,
            ..AIR_GRAVITY
        };
        let seconds_to_pretense = pretense_phase.seconds.unwrap_or(Seconds(15.0));
        Self {
            stage: Start,
            pretense_phase,
            seconds_to_pretense,
            physics,
            radio: radio.clone(),
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Process the current stage
        self.stage = match self.stage {
            Start => {
                let face_ids: Vec<_> = context.fabric.faces.keys().copied().collect();
                for face_id in face_ids {
                    let face = context.fabric.face(face_id);
                    if !face.has_prism {
                        context.fabric.add_face_triangle(face_id);
                    }
                    context.fabric.remove_face(face_id);
                }
                Slacken
            },
            Slacken => {
                context.fabric.slacken();
                let altitude = self.pretense_phase.altitude.unwrap_or(0.0) / context.fabric.scale;
                context.fabric.centralize(Some(altitude));
                
                let factor = self
                    .pretense_phase
                    .pretenst
                    .unwrap_or(self.physics.pretenst);
                context.fabric.set_pretenst(factor, self.seconds_to_pretense);
                DumpCSV.send(&self.radio);
                Pretensing
            }
            Pretensing => {
                for _ in context.physics.iterations() {
                    context.fabric.iterate(context.physics);
                }

                if context.fabric.progress.is_busy() {
                    Pretensing
                } else {
                    if self.pretense_phase.muscle_movement.is_some() {
                        CreateMuscles
                    } else {
                        Pretenst
                    }
                }
            }
            CreateMuscles => {
                if context.fabric.progress.is_busy() {
                    // Perform a single physics iteration
                    context.fabric.iterate(context.physics);

                    CreateMuscles
                } else {
                    let Some(muscle_movement) = &self.pretense_phase.muscle_movement else {
                        panic!("expected a muscle movement")
                    };
                    context.fabric.create_muscles(muscle_movement.contraction);
                    self.physics.cycle_ticks = muscle_movement.countdown as f32;
                    // Update physics when cycle_ticks changes
                    *context.physics = self.physics.clone();
                    context.fabric.progress.start(MOMENT);

                    Pretenst
                }
            }
            Pretenst => {
                for _ in context.physics.iterations() {
                    context.fabric.iterate(context.physics);
                }

                Pretenst
            }
        };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Pretenst
    }

    pub fn physics(&self) -> &Physics {
        &self.physics
    }
}
