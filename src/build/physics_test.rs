use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::{PhysicsTesterAction, Radio, StateChange};

pub struct PhysicsTester {
    test_number: usize,
    test_cases: Vec<PhysicsTest>,
    radio: Radio,
}

impl PhysicsTester {
    pub fn new(fabric: &Fabric, physics: Physics, radio: Radio) -> Self {
        Self {
            test_number: 0,
            test_cases: PhysicsTest::generate(&fabric, physics),
            radio,
        }
    }

    pub fn iterate(&mut self) {
        self.test_cases
            .get_mut(self.test_number)
            .expect("No test case")
            .iterate();
    }

    pub fn action(&mut self, action: PhysicsTesterAction) {
        use StateChange::*;
        use PhysicsTesterAction::*;
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
                SetExperimentTitle {
                    title: self.test_case().title(),
                    fabric_stats: self.fabric().fabric_stats(),
                }
                .send(&self.radio);
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &PhysicsTest {
        &self.test_cases[self.test_number]
    }
}

#[derive(Debug, Clone)]
pub struct PhysicsTest {
    pub fabric: Fabric,
    physics: Physics,
}

impl PhysicsTest {
    pub fn title(&self) -> String {
        format!("Stiffness {:.5}", self.physics.stiffness)
    }

    pub fn generate(default_fabric: &Fabric, physics: Physics) -> Vec<PhysicsTest> {
        let mut test_cases: Vec<PhysicsTest> = vec![
            PhysicsTest {
                fabric: default_fabric.clone(),
                physics: physics.clone(),
            };
            10
        ];
        for index in 0..test_cases.len() {
            test_cases[index].physics.stiffness *= (index + 1) as f32;
        }
        test_cases
    }

    pub fn iterate(&mut self) {
        self.fabric.iterate(&self.physics);
    }
}
