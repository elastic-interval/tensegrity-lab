use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{AppearanceMode, PhysicsFeature, Radio, Role, StateChange, TesterAction};
use std::rc::Rc;

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
}

impl PhysicsTester {
    pub fn new(fabric: &Fabric, physics: Physics, radio: Radio) -> Self {
        let mut fabric = fabric.clone();
        physics.broadcast(&radio);
        fabric.activate_muscles(true);
        Self {
            fabric,
            physics,
            radio,
        }
    }

    pub fn iterate(&mut self, context: &mut crate::crucible_context::CrucibleContext) {
        // Set the physics directly to avoid expensive cloning on every iteration
        *context.physics = self.physics.clone();

        // Update our fabric from the context
        self.fabric = context.fabric.clone();

        // Use the physics-defined number of iterations
        for _ in context.physics.iterations() {
            // Iterate our fabric
            self.fabric.iterate(context.physics);
        }

        // Update the context's fabric with our changes after all iterations
        context.replace_fabric(self.fabric.clone());
    }

    pub fn action(&mut self, action: TesterAction) {
        use TesterAction::*;
        match action {
            SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
                match parameter.feature {
                    PhysicsFeature::Pretenst => {
                        self.fabric.set_pretenst(parameter.value, 100);
                    }
                    PhysicsFeature::StrainLimit => {
                        let strain_limit = self.physics.strain_limit;
                        StateChange::SetAppearanceFunction(Rc::new(move |interval| {
                            if interval.strain > strain_limit {
                                let role = interval.material.properties().role;
                                Some(match role {
                                    Role::Pushing => role
                                        .appearance()
                                        .apply_mode(AppearanceMode::HighlightedPush),
                                    _ => role
                                        .appearance()
                                        .apply_mode(AppearanceMode::HighlightedPull),
                                })
                            } else {
                                None
                            }
                        }))
                        .send(&self.radio.clone());
                    }
                    _ => {}
                }
            }
            DumpPhysics => {
                println!("{:?}", self.physics);
            }
            _ => {}
        }
    }
}
