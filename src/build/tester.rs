use crate::application::AppStateChange;
use crate::build::failure_test::FailureTest;
use crate::crucible::TesterAction;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{LabEvent, Scenario};
use winit::event_loop::EventLoopProxy;

const MAX_NEW_ITERATIONS: u64 = 100000;

pub struct Tester {
    test_number: usize,
    default_fabric: Fabric,
    min_damage: f32,
    max_damage: f32,
    test_cases: Vec<FailureTest>,
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
        Self {
            test_number: 0,
            default_fabric: fabric.clone(),
            min_damage: Self::min_damage(scenario),
            max_damage: Self::max_damage(scenario),
            test_cases: FailureTest::generate(&fabric, scenario),
            physics,
            event_loop_proxy,
        }
    }

    pub fn iterate(&mut self) {
        let test_case = self
            .test_cases
            .get_mut(self.test_number)
            .expect("No test case");
        let iterations = test_case.fabric.age - self.default_fabric.age;
        if iterations >= MAX_NEW_ITERATIONS {
            test_case.finish(
                &self.default_fabric,
                self.min_damage,
                self.max_damage,
                self.event_loop_proxy.clone(),
            );
        } else {
            test_case.fabric.iterate(&self.physics);
        }
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
                    title: self.test_case().title(),
                    fabric_stats: self.fabric().fabric_stats(),
                }));
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &FailureTest {
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
