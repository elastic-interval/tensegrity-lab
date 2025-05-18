use crate::fabric::material::Material;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::fabric::{Fabric, UniqueId};
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
            BoxingStep::RemoveIntervals,
            BoxingStep::RemoveJoints,
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
    RemoveIntervals,
    RemoveJoints,
}

impl BoxingStep {
    fn age(&self) -> Age {
        match self {
            BoxingStep::RemoveSupport => Age::seconds(38),
            BoxingStep::Deflate => Age::seconds(50),
            BoxingStep::RemoveIntervals => Age::seconds(55),
            BoxingStep::RemoveJoints => Age::seconds(60),
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
            BoxingStep::RemoveIntervals => [
                (2, 3),
                (14, 15),
                (16, 17),
                (89, 88),
                (87, 86),
                (69, 68),
                (70, 71),
                (52, 53),
                (42, 43),
                (45, 44),
                (51, 50),
                (46, 47),
                (48, 49),
            ]
            .iter()
            .for_each(|&alpha_omega| {
                fabric.joining(alpha_omega).map(|id| {
                    let _interval = fabric.remove_interval(id);
                    // fabric.remove_joint(interval.alpha_index);
                    // fabric.remove_joint(interval.omega_index);
                });
            }),
            BoxingStep::RemoveJoints => {
                fabric
                    .joint_incidents()
                    .iter()
                    .filter_map(|joint| {
                        let index = joint.index;
                        match joint.push() {
                            None => Some(index),
                            Some(_) => None,
                        }
                    })
                    .rev()
                    .for_each(|index| fabric.remove_joint(index));
            }
        }
    }
}

fn remove_supports(fabric: &mut Fabric) {
    fabric
        .intervals
        .iter()
        .enumerate()
        .filter_map(|(index, interval_opt)| {
            interval_opt.as_ref().and_then(|interval| {
                if interval.material.properties().support {
                    Some(UniqueId(index))
                } else {
                    None
                }
            })
        })
        .collect_vec()
        .into_iter()
        .for_each(|support| {
            let omega = fabric.remove_interval(support).omega_index;
            fabric.remove_joint(omega);
        });
}
