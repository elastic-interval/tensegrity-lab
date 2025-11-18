use crate::crucible_context::CrucibleContext;
use crate::fabric::Fabric;
use crate::fabric::physics::Physics;
use crate::units::Seconds;
use crate::{PhysicsFeature, Radio, StateChange, TesterAction, ITERATIONS_PER_FRAME};

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
    iterations_since_stats_update: usize,
}

impl PhysicsTester {
    pub fn new(fabric: Fabric, physics: Physics, radio: Radio) -> Self {
        Self {
            fabric,
            physics,
            radio,
            iterations_since_stats_update: 0,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        self.fabric = context.fabric.clone();

        // Use our own physics (which has user modifications) instead of context.physics
        for _ in 0..ITERATIONS_PER_FRAME {
            self.fabric.iterate(&self.physics);
        }
        
        // Track iterations for stats updates (count frames, not iterations)
        self.iterations_since_stats_update += 1;
        
        // Update stats approximately every second (60 frames)
        if self.iterations_since_stats_update >= 60 {
            self.iterations_since_stats_update = 0;
            
            // Recalculate and broadcast updated stats
            let stats = self.fabric.stats_with_dynamics(&self.physics);
            StateChange::SetFabricStats(Some(stats)).send(&self.radio);
        }

        // Update the context's fabric and physics with our changes
        context.replace_fabric(self.fabric.clone());
        *context.physics = self.physics.clone();
    }

    pub fn action(&mut self, action: TesterAction) {
        use TesterAction::*;
        match action {
            SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
                match parameter.feature {
                    PhysicsFeature::Pretenst => {
                        self.fabric.set_pretenst(parameter.value, Seconds(10.0));
                    }
                    _ => {}
                }
            }
            SetTweakParameter(parameter) => {
                self.physics.accept_tweak(parameter);
                // Mass/rigidity changes take effect on the next iterate() call
            }
            DumpPhysics => {
                println!("{:?}", self.physics);
            }
        }
    }
}
