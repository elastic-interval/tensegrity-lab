use crate::application::AppStateChange;
use crate::crucible::{CrucibleAction, TesterAction};
use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{LabEvent, Scenario};
use cgmath::InnerSpace;
use winit::event_loop::EventLoopProxy;

const MAX_NEW_ITERATIONS: u64 = 100000;

#[derive(Clone)]
struct TestCase {
    fabric: Fabric,
    interval_missing: Option<(usize, usize)>,
    damage: f32,
    finished: bool,
}

pub struct Tester {
    test_number: usize,
    default_fabric: Fabric,
    min_damage: f32,
    max_damage: f32,
    test_cases: Vec<TestCase>,
    physics: Physics,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Tester {
    pub fn new(
        scenario: Scenario,
        fabric: &Fabric,
        physics: Physics,
        event_loop_proxy: EventLoopProxy<LabEvent>,
    ) -> Self {
        let interval_keys: Vec<_> = fabric
            .intervals
            .iter()
            .flat_map(|(id, interval)| match (interval.material, &scenario) {
                (Material::PullMaterial, Scenario::TensionTest) => Some(*id),
                (Material::PushMaterial, Scenario::CompressionTest) => Some(*id),
                (Material::GuyLineMaterial, Scenario::TensionTest) => Some(*id),
                _ => None,
            })
            .collect();
        let mut test_cases = vec![
            TestCase {
                fabric: fabric.clone(),
                interval_missing: None,
                damage: 0.0,
                finished: false,
            };
            interval_keys.len()
        ];
        for index in 0..interval_keys.len() {
            let missing_key = interval_keys[index];
            let &Interval {
                alpha_index,
                omega_index,
                ..
            } = fabric.interval(missing_key);
            test_cases[index].fabric.remove_interval(missing_key);
            test_cases[index].interval_missing = Some(if alpha_index < omega_index {
                (alpha_index, omega_index)
            } else {
                (omega_index, alpha_index)
            });
        }
        Self {
            test_number: 0,
            default_fabric: fabric.clone(),
            min_damage: Self::min_damage(scenario),
            max_damage: Self::max_damage(scenario),
            test_cases,
            physics,
            event_loop_proxy,
        }
    }

    pub fn iterate(&mut self) {
        use AppStateChange::*;
        use CrucibleAction::*;
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        let physics = &self.physics;
        let test_case = self
            .test_cases
            .get_mut(self.test_number)
            .expect("No test case");
        let iterations = test_case.fabric.age - self.default_fabric.age;
        if iterations >= MAX_NEW_ITERATIONS && !test_case.finished {
            test_case.finished = true;
            let mut damage = 0.0;
            for joint_id in 0..self.default_fabric.joints.len() {
                let default_location = self.default_fabric.location(joint_id);
                let new_location = test_case.fabric.location(joint_id);
                damage += (default_location - new_location).magnitude();
            }
            test_case.damage = damage;
            let key = test_case.interval_missing.unwrap();
            let clamped = test_case.damage.clamp(self.min_damage, self.max_damage);
            let redness = (clamped - self.min_damage) / (self.max_damage - self.min_damage);
            let color = [redness, 0.01, 0.01, 1.0];
            send(LabEvent::AppStateChanged(SetIntervalColor { key, color }));
            send(LabEvent::Crucible(TesterDo(TesterAction::NextExperiment)));
        }
        test_case.fabric.iterate(physics);
    }

    pub fn action(&mut self, action: TesterAction) {
        use AppStateChange::*;
        use LabEvent::*;
        use TesterAction::*;
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        match action {
            PrevExperiment | NextExperiment => {
                if matches!(action, NextExperiment) {
                    if self.test_number + 1 < self.test_cases.len() {
                        self.test_number += 1
                    }
                } else {
                    if self.test_number > 0 {
                        self.test_number -= 1;
                    }
                };
                send(AppStateChanged(SetExperimentTitle {
                    title: match self.test_cases[self.test_number].interval_missing {
                        None => {
                            format!("Test #{}", self.test_number)
                        }
                        Some(pair) => {
                            format!("Test #{} {pair:?}", self.test_number)
                        }
                    },
                    fabric_stats: self.fabric().fabric_stats(),
                }));
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &TestCase {
        &self.test_cases[self.test_number]
    }

    fn min_damage(scenario: Scenario) -> f32 {
        match scenario {
            Scenario::TensionTest => 100.0,
            Scenario::CompressionTest => 500.0,
            _ => {
                unreachable!()
            }
        }
    }

    fn max_damage(scenario: Scenario) -> f32 {
        match scenario {
            Scenario::TensionTest => 500.0,
            Scenario::CompressionTest => 1000.0,
            _ => {
                unreachable!()
            }
        }
    }
}
