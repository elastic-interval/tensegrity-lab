use crate::fabric::material::Material;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::Fabric;
use crate::{Age, PhysicsFeature, TesterAction};
use cgmath::Point3;
use itertools::Itertools;
use std::collections::VecDeque;

pub struct BoxingTest {
    pub fabric: Fabric,
    pub physics: Physics,
    steps: VecDeque<BoxingStep>,
}

impl BoxingTest {
    pub fn new(fabric: &Fabric, physics: Physics) -> Self {
        let fabric = fabric.clone();
        let steps = VecDeque::from([
            BoxingStep::RemoveSupport,
            BoxingStep::Deflate,
            BoxingStep::Disconnect,
        ]);
        Self {
            fabric,
            physics,
            steps,
        }
    }

    pub fn iterate(&mut self) {
        self.fabric.iterate(&self.physics);
        if let Some(step) = self.steps.front() {
            if !self.fabric.age.within(&step.age()) {
                step.execute(&mut self.fabric, &mut self.physics);
                self.steps.pop_front();
            }
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
}

enum BoxingStep {
    RemoveSupport,
    Deflate,
    Disconnect,
}

impl BoxingStep {
    fn age(&self) -> Age {
        match self {
            BoxingStep::RemoveSupport => Age::seconds(38),
            BoxingStep::Deflate => Age::seconds(50),
            BoxingStep::Disconnect => Age::seconds(60),
        }
    }

    fn execute(&self, fabric: &mut Fabric, physics: &mut Physics) {
        match self {
            BoxingStep::RemoveSupport => {
                remove_supports(fabric);
                for joint in &mut fabric.joints {
                    joint.fixed = false;
                }
                let base = fabric.create_fixed_joint(Point3::new(10.0, 0.0, 0.0));
                let length = 10.0;
                fabric.create_interval(3, base, length, Material::GuyLine);
                fabric.progress.start(20000);
                physics.surface_character = SurfaceCharacter::Sticky;
            }
            BoxingStep::Deflate => {
                remove_supports(fabric);
                physics.pretenst = -10.0;
                fabric.set_pretenst(physics.pretenst, 20000);
            }
            BoxingStep::Disconnect => [
                (2, 3),
                (14, 15),
                (16, 17),
                (89, 88),
                (87, 86),
                (69, 68),
                (70, 71),
            ]
            .iter()
            .for_each(|&alpha_omega| {
                fabric.remove_interval_joining(alpha_omega);
            }),
        }
    }
}

fn remove_supports(fabric: &mut Fabric) {
    fabric
        .intervals
        .iter()
        .filter_map(|(id, interval)| interval.material.properties().support.then_some(id))
        .cloned()
        .collect_vec()
        .into_iter()
        .for_each(|support| {
            let omega = fabric.remove_interval(support).omega_index;
            fabric.remove_joint(omega);
        });
}
