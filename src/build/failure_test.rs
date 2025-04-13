use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{CrucibleAction, Radio, StateChange, TestScenario, TesterAction};
use crate::Age;
use cgmath::InnerSpace;

pub struct FailureTester {
    pub physics: Physics,
    test_number: usize,
    default_fabric: Fabric,
    test_cases: Vec<FailureTest>,
    radio: Radio,
}

impl FailureTester {
    pub fn new(scenario: TestScenario, fabric: &Fabric, physics: Physics, radio: Radio) -> Self {
        let max_age = fabric.age.advanced(100_000);
        Self {
            test_number: 0,
            default_fabric: fabric.clone(),
            test_cases: FailureTest::generate(&fabric, scenario, max_age),
            physics: Physics {
                iterations_per_frame: 1000.0,
                ..physics
            },
            radio,
        }
    }

    pub fn iterate(&mut self) {
        let test_case = self
            .test_cases
            .get_mut(self.test_number)
            .expect("No test case");
        if !test_case.completed(&self.default_fabric, self.radio.clone()) {
            test_case.fabric.iterate(&self.physics);
        }
    }

    pub fn action(&mut self, action: TesterAction) {
        use StateChange::*;
        use TesterAction::*;
        match action {
            SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
            }
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
                SetExperimentTitle {
                    title: self.test_case().title(),
                    fabric_stats: self.fabric().fabric_stats(),
                }
                .send(&self.radio);
            }
            _ => {}
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &FailureTest {
        &self.test_cases[self.test_number]
    }
}

#[derive(Clone)]
pub struct FailureTest {
    pub fabric: Fabric,
    scenario: TestScenario,
    max_age: Age,
    finished: bool,
    interval_missing: Option<(usize, usize)>,
}

impl FailureTest {
    pub fn title(&self) -> String {
        match self.interval_missing {
            None => "Reference".to_string(),
            Some(pair) => {
                format!("Missing {pair:?}")
            }
        }
    }

    pub fn generate(
        default_fabric: &Fabric,
        scenario: TestScenario,
        max_age: Age,
    ) -> Vec<FailureTest> {
        let interval_keys: Vec<_> = default_fabric
            .intervals
            .iter()
            .flat_map(|(id, interval)| match (interval.material, &scenario) {
                (Material::Pull, TestScenario::TensionTest) => Some(*id),
                (Material::Push, TestScenario::CompressionTest) => Some(*id),
                (Material::GuyLine, TestScenario::TensionTest) => Some(*id),
                _ => None,
            })
            .collect();
        let mut test_cases = vec![
            FailureTest {
                scenario,
                fabric: default_fabric.clone(),
                max_age,
                interval_missing: None,
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
            } = default_fabric.interval(missing_key);
            test_cases[index].fabric.remove_interval(missing_key);
            test_cases[index].interval_missing = Some(if alpha_index < omega_index {
                (alpha_index, omega_index)
            } else {
                (omega_index, alpha_index)
            });
        }
        test_cases
    }

    pub fn damage(&self, default_fabric: &Fabric) -> f32 {
        let mut damage = 0.0;
        for joint_id in 0..default_fabric.joints.len() {
            let default_location = default_fabric.location(joint_id);
            let new_location = self.fabric.location(joint_id);
            damage += (default_location - new_location).magnitude();
        }
        damage
    }

    pub fn completed(&mut self, default_fabric: &Fabric, radio: Radio) -> bool {
        if self.finished {
            return true;
        }
        if self.fabric.age.within(&self.max_age) {
            return false;
        }
        self.finished = true;
        let key = self.interval_missing.unwrap();
        let min_damage = Self::min_damage(&self.scenario);
        let max_damage = Self::max_damage(&self.scenario);
        let clamped = self.damage(default_fabric).clamp(min_damage, max_damage);
        let redness = (clamped - min_damage) / (max_damage - min_damage);
        let color = [redness, 0.01, 0.01, 1.0];
        StateChange::SetIntervalColor { key, color }.send(&radio);
        CrucibleAction::TesterDo(TesterAction::NextExperiment).send(&radio);
        true
    }

    fn min_damage(scenario: &TestScenario) -> f32 {
        match scenario {
            TestScenario::TensionTest => 100.0,
            TestScenario::CompressionTest => 500.0,
            _ => unreachable!(),
        }
    }

    fn max_damage(scenario: &TestScenario) -> f32 {
        match scenario {
            TestScenario::TensionTest => 500.0,
            TestScenario::CompressionTest => 1000.0,
            _ => unreachable!(),
        }
    }
}
