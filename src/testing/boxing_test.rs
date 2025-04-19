use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::Fabric;
use crate::{Age, PhysicsFeature, TesterAction};
use itertools::Itertools;

pub struct BoxingTest {
    pub fabric: Fabric,
    pub physics: Physics,
    steps: Vec<TimedStep>,
}

impl BoxingTest {
    pub fn new(fabric: &Fabric, physics: Physics) -> Self {
        let fabric = fabric.clone();
        let steps = vec![Self::remove_support()];
        Self {
            fabric,
            physics,
            steps,
        }
    }

    pub fn iterate(&mut self) {
        self.fabric.iterate(&self.physics);
        if let Some(step) = self.steps.first() {
            if self.fabric.age.within(&step.age) {
                return;
            }
            (step.execute)(&mut self.fabric, &mut self.physics);
            self.steps.remove(0);
        }
    }

    pub fn action(&mut self, action: TesterAction) {
        use crate::TesterAction::*;
        match action {
            SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
                match parameter.feature {
                    PhysicsFeature::Pretenst => {
                        self.fabric.set_pretenst(parameter.value, 100);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn remove_support() -> TimedStep {
        TimedStep {
            age: Age::seconds(40),
            execute: Box::new(|fabric, physics| {
                physics.surface_character = SurfaceCharacter::Bouncy;
                let supports = fabric
                    .intervals
                    .iter()
                    .filter_map(|(id, interval)| {
                        interval.material.properties().support.then_some(id)
                    })
                    .cloned()
                    .collect_vec();
                for support in supports {
                    fabric.remove_interval(support)
                }
            }),
        }
    }
}

struct TimedStep {
    age: Age,
    execute: Box<dyn Fn(&mut Fabric, &mut Physics) -> ()>,
}
