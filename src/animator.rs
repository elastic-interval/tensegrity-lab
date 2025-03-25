use crate::animator::Stage::{MuscleCycle, Paused};
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::AnimatorAction;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

#[derive(Clone, PartialEq)]
enum Stage {
    Paused,
    MuscleCycle(f32),
}

pub struct Animator {
    pretense_phase: PretensePhase,
    muscles_active: bool,
    stage: Stage,
    physics: Physics,
}

impl Animator {
    pub fn new(
        Pretenser {
            pretense_phase,
            physics,
            ..
        }: Pretenser,
    ) -> Self {
        Self {
            muscles_active: false,
            stage: Paused,
            pretense_phase,
            physics,
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        let physics = &self.physics;
        self.stage = match self.stage {
            Paused => {
                fabric.iterate(physics);
                Paused
            }
            MuscleCycle(increment) => {
                fabric.iterate(physics);
                fabric.muscle_nuance += increment;
                if fabric.muscle_nuance < 0.0 {
                    fabric.muscle_nuance = 0.0;
                    MuscleCycle(-increment)
                } else if fabric.muscle_nuance > 1.0 {
                    fabric.muscle_nuance = 1.0;
                    MuscleCycle(-increment)
                } else {
                    MuscleCycle(increment)
                }
            }
        };
    }

    pub fn action(&mut self, action: AnimatorAction, fabric: &mut Fabric) {
        match action {
            AnimatorAction::MusclesActive(muscles_active) => {
                if self.muscles_active != muscles_active {
                    self.muscles_active = muscles_active;
                }
                if self.stage == Paused {
                    if muscles_active {
                        if let Some(movement) = &self.pretense_phase.muscle_movement {
                            self.stage = MuscleCycle(1.0 / movement.countdown as f32)
                        }
                    } else {
                        self.stage = Paused;
                    }
                } else {
                    fabric.muscle_nuance = 0.5;
                    self.stage = Paused
                }
            }
            AnimatorAction::ToggleMusclesActive => {
                let opposite = !self.muscles_active;
                self.action(AnimatorAction::MusclesActive(opposite), fabric);
            }
        }
    }

    pub fn physics(&self) -> &Physics {
        &self.physics
    }
}
