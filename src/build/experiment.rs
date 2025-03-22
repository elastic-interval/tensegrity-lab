use crate::application::AppStateChange;
use crate::build::experiment::Stage::*;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::LabAction;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, UniqueId};
use crate::messages::LabEvent;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Clone, PartialEq)]
enum Stage {
    Paused,
    MuscleCycle(f32),
}

pub struct Experiment {
    stage: Stage,
    frozen_fabrics: Vec<Fabric>,
    current_fabric: usize,
    physics: Physics,
    pretense_phase: PretensePhase,
    random: ChaCha8Rng,
    interval_keys: Vec<UniqueId>,
}

impl Experiment {
    pub fn new(
        Pretenser {
            pretense_phase,
            physics,
            ..
        }: Pretenser,
        fabric: &Fabric,
    ) -> Self {
        Self {
            frozen_fabrics: vec![fabric.clone()],
            current_fabric: 0,
            stage: Paused,
            physics,
            pretense_phase,
            random: ChaCha8Rng::seed_from_u64(0),
            interval_keys: fabric.intervals.keys().cloned().collect(),
        }
    }

    pub fn iterate(&mut self) {
        let fabric = self
            .frozen_fabrics
            .get_mut(self.current_fabric)
            .expect("No frozen fabric");
        if self.current_fabric > 0 && fabric.age > 200000 {
            return;
        }
        self.stage = match self.stage {
            Paused => {
                fabric.iterate(&self.physics);
                Paused
            }
            MuscleCycle(increment) => {
                fabric.iterate(&self.physics);
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

    pub fn action(&mut self, action: LabAction, default_fabric: &Fabric) -> Option<LabEvent> {
        let fabric = self
            .frozen_fabrics
            .get_mut(self.current_fabric)
            .expect("a current fabric");
        match action {
            LabAction::GravityChanged(gravity) => self.physics.gravity = gravity,
            LabAction::MuscleChanged(nuance) => {
                fabric.muscle_nuance = nuance;
            }
            LabAction::MusclesActive(yes) => {
                if self.stage == Paused {
                    if yes {
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
            LabAction::KillAnInterval => {
                let keys: Vec<_> = fabric.intervals.keys().cloned().collect();
                let which = self.random.gen_range(0..keys.len());
                fabric.remove_interval(keys[which]);
            }
            LabAction::NextExperiment(forward) => {
                if forward {
                    if self.current_fabric < self.interval_keys.len() {
                        self.current_fabric += 1;
                    } else {
                        self.current_fabric = 0;
                    }
                    let fabric = &self.current_fabric(default_fabric);
                    if fabric.age == default_fabric.age {
                        self.frozen_fabrics[self.current_fabric]
                            .remove_interval(self.interval_keys[self.current_fabric - 1]);
                    }
                } else {
                    if self.current_fabric > 0 {
                        self.current_fabric -= 1;
                    }
                }
                return Some(LabEvent::AppStateChanged(AppStateChange::SetFabricNumber {
                    number: self.current_fabric,
                    fabric_stats: self.current_fabric(default_fabric).fabric_stats(),
                }));
            }
        }
        None
    }

    pub fn current_fabric(&mut self, fabric: &Fabric) -> &Fabric {
        self.get_fabric(self.current_fabric, fabric)
    }

    fn get_fabric(&mut self, index: usize, default_fabric: &Fabric) -> &Fabric {
        // Ensure the vector is large enough
        if self.frozen_fabrics.len() <= index {
            // Clone default_fabric for all missing indices
            for _ in self.frozen_fabrics.len()..=index {
                self.frozen_fabrics.push(default_fabric.clone());
            }
        }

        // Now we can safely return a reference
        &self.frozen_fabrics[index]
    }
}
