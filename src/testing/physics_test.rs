use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{AppearanceMode, PhysicsFeature, Radio, Role, Seconds, StateChange, TesterAction};
use std::rc::Rc;

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
}

impl PhysicsTester {
    pub fn new(fabric: Fabric, physics: Physics, radio: Radio) -> Self {
        Self {
            fabric,
            physics,
            radio,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        self.fabric = context.fabric.clone();

        for _ in context.physics.iterations() {
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
                        self.fabric.set_pretenst(parameter.value, Seconds(10.0));
                    }
                    PhysicsFeature::StrainLimit => {
                        let strain_limit = self.physics.strain_limit;
                        StateChange::SetAppearanceFunction(Rc::new(move |interval| {
                            if interval.strain > strain_limit {
                                let role = interval.role;
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
