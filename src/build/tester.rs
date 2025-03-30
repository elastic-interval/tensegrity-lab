use crate::application::AppStateChange;
use crate::crucible::{CrucibleAction, TesterAction};
use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, UniqueId};
use crate::messages::{LabEvent, Scenario};
use cgmath::InnerSpace;
use winit::event_loop::EventLoopProxy;

const MAX_NEW_ITERATIONS: u64 = 100000;

#[derive(Clone)]
struct TestCase {
    fabric: Fabric,
    interval_missing: Option<(UniqueId, Interval)>,
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
            interval_keys.len() + 1
        ];
        for index in 1..=interval_keys.len() {
            let missing_id = interval_keys[index - 1];
            test_cases[index].fabric.remove_interval(missing_id);
            test_cases[index].interval_missing =
                Some((missing_id, fabric.interval(missing_id).clone()));
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
        use CrucibleAction::*;
        use LabEvent::*;
        use TesterAction::*;
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        let physics = &self.physics;
        let test_case = self
            .test_cases
            .get_mut(self.test_number)
            .expect("No test case");
        if test_case.finished {
            return;
        }
        let iterations = test_case.fabric.age - self.default_fabric.age;
        if iterations >= MAX_NEW_ITERATIONS {
            test_case.finished = true;
            if self.test_number == 0 {
                return;
            }
            let mut damage = 0.0;
            for joint_id in 0..self.default_fabric.joints.len() {
                let default_location = self.default_fabric.location(joint_id);
                let new_location = test_case.fabric.location(joint_id);
                damage += (default_location - new_location).magnitude();
            }
            test_case.damage = damage;
            let (id, interval) = test_case.interval_missing.unwrap();
            let clamped = test_case.damage.clamp(self.min_damage, self.max_damage);
            let redness = (clamped - self.min_damage) / (self.max_damage - self.min_damage);
            let color = [redness, 0.01, 0.01, 1.0];
            let appearance = interval.appearance().with_color(color);
            send(Crucible(TesterDo(SetIntervalAppearance { id, appearance })));
            send(Crucible(TesterDo(NextExperiment)));
        } else {
            test_case.fabric.iterate(physics);
        }
        // const STRAIN_THRESHOLD: f32 = 0.17;
        // let broken: Vec<_> = test_case
        //     .fabric
        //     .intervals
        //     .iter()
        //     .map(|(id,interval)| (id, interval.strain))
        //     .filter(|(_, strain)| *strain > STRAIN_THRESHOLD)
        //     .map(|(id, _)| *id)
        //     .collect();
        // for id in broken {
        //     let color = [0.0, 1.0, 0.0, 1.0];
        //     let appearance = test_case.fabric.interval(id)
        //     self.event_loop_proxy
        //         .send_event(Crucible(SetIntervalAppearance { id, appearance }))
        //         .unwrap();
        //     // test_case.fabric.remove_interval(id);
        // }
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
                        Some((_, interval)) => {
                            let pair = interval.key();
                            format!("Test #{} {pair:?}", self.test_number)
                        }
                    },
                    fabric_stats: self.fabric().fabric_stats(),
                }));
            }
            SetIntervalAppearance { id, appearance } => {
                println!(
                    "Set Interval Appearance #{}: {:?}",
                    self.test_number, appearance
                );
                for test_case in self.test_cases.iter_mut() {
                    if let Some(interval) = test_case.fabric.intervals.get_mut(&id) {
                        interval.appearance = Some(appearance);
                    }
                }
            }
            SortOnDamage => {
                // todo: sort the cases
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
