use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::pretenser::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::PRETENSING;
use crate::fabric::physics::Physics;
use crate::LabEvent::DumpCSV;
use crate::units::{Percent, Seconds};
use crate::Radio;

#[derive(Clone, Debug, PartialEq, Copy)]
enum Stage {
    Start,
    Slacken,
    Pretensing,
    Pretenst,
}

#[derive(Clone)]
pub struct Pretenser {
    pub pretense_phase: PretensePhase,
    stage: Stage,
    seconds_to_pretense: Seconds,
    radio: Radio,
}

impl Pretenser {
    pub fn new(pretense_phase: PretensePhase, radio: &Radio) -> Self {
        let seconds_to_pretense = pretense_phase.seconds.unwrap_or(Seconds(15.0));
        Self {
            stage: Start,
            pretense_phase,
            seconds_to_pretense,
            radio: radio.clone(),
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = PRETENSING;
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
                
                // Calculate translation, set it in context for synchronous camera update, then apply
                let translation = context.fabric.centralize_translation(Some(altitude));
                context.set_camera_translation(translation);
                context.fabric.apply_translation(translation);
                
                let pretenst_percent = self
                    .pretense_phase
                    .pretenst
                    .map(|p| Percent(p))
                    .unwrap_or(PRETENSING.pretenst);
                context.fabric.set_pretenst(pretenst_percent, self.seconds_to_pretense);
                DumpCSV.send(&self.radio);
                Pretensing
            }
            Pretensing => {
                for _ in 0..1000 {  // Nominal value, outer loop adjusts dynamically
                    context.fabric.iterate(context.physics);
                }

                if context.fabric.progress.is_busy() {
                    Pretensing
                } else {
                    Pretenst
                }
            }
            Pretenst => {
                for _ in 0..1000 {  // Nominal value, outer loop adjusts dynamically
                    context.fabric.iterate(context.physics);
                }

                Pretenst
            }
        };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Pretenst
    }

    pub fn physics(&self) -> Physics {
        self.pretense_phase.viewing_physics()
    }
}
