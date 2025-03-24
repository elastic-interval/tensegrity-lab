use crate::application::AppStateChange;
use crate::build::experiment::Stage::*;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::pretenser::Pretenser;
use crate::crucible::{CrucibleAction, LabAction};
use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::LabEvent;
use cgmath::InnerSpace;

#[derive(Clone, PartialEq)]
enum Stage {
    Paused,
    MuscleCycle(f32),
    RunningTestCase(usize),
}

const TIMEOUT_ITERATIONS: usize = 10000;
const MAX_AGE: u64 = 180000;

#[derive(Clone)]
struct TestCase {
    fabric: Fabric,
    interval_missing: Option<(usize, usize)>,
    tension: bool,
    damage: f32,
}

pub struct Experiment {
    default_fabric: Fabric,
    min_damage: f32,
    max_damage: f32,
    test_cases: Vec<TestCase>,
    stage: Stage,
    physics: Physics,
    pretense_phase: PretensePhase,
    timeout_iterations: usize,
}

impl Experiment {
    pub fn new(
        Pretenser {
            pretense_phase,
            physics,
            ..
        }: Pretenser,
        fabric: &Fabric,
        tension: bool,
    ) -> Self {
        let interval_keys: Vec<_> = fabric
            .intervals
            .iter()
            .flat_map(|(id, interval)| {
                if interval.material == Material::PushMaterial {
                    if tension {
                        None
                    } else {
                        Some(*id)
                    }
                } else {
                    if tension {
                        Some(*id)
                    } else {
                        None
                    }
                }
            })
            .collect();
        let case_count = interval_keys.len() + 1;
        let mut test_cases = vec![
            TestCase {
                fabric: fabric.clone(),
                interval_missing: None,
                tension,
                damage: 0.0,
            };
            case_count
        ];
        for index in 1..case_count {
            let missing_key = interval_keys[index - 1];
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
            default_fabric: fabric.clone(),
            min_damage: Self::min_damage(tension),
            max_damage: Self::max_damage(tension),
            test_cases,
            stage: Paused,
            physics,
            pretense_phase,
            timeout_iterations: TIMEOUT_ITERATIONS,
        }
    }

    pub fn iterate(&mut self) -> Option<LabEvent> {
        let physics = &self.physics;
        self.stage = match self.stage {
            Paused => {
                let test_case = &mut self.test_cases[0];
                test_case.fabric.iterate(physics);
                Paused
            }
            MuscleCycle(increment) => {
                let test_case = &mut self.test_cases[0];
                test_case.fabric.iterate(physics);
                test_case.fabric.muscle_nuance += increment;
                if test_case.fabric.muscle_nuance < 0.0 {
                    test_case.fabric.muscle_nuance = 0.0;
                    MuscleCycle(-increment)
                } else if test_case.fabric.muscle_nuance > 1.0 {
                    test_case.fabric.muscle_nuance = 1.0;
                    MuscleCycle(-increment)
                } else {
                    MuscleCycle(increment)
                }
            }
            RunningTestCase(fabric_number) => {
                let test_case = self
                    .test_cases
                    .get_mut(fabric_number)
                    .expect("No test case");
                if fabric_number > 0 && test_case.fabric.age >= MAX_AGE {
                    self.timeout_iterations -= 1;
                    if self.timeout_iterations == 0 {
                        self.timeout_iterations = TIMEOUT_ITERATIONS;
                        if test_case.damage == 0.0 {
                            let mut damage = 0.0;
                            for joint_id in 0..self.default_fabric.joints.len() {
                                let default_location = self.default_fabric.location(joint_id);
                                let new_location = test_case.fabric.location(joint_id);
                                damage += (default_location - new_location).magnitude();
                            }
                            test_case.damage = damage;
                        } else {
                            return Some(LabEvent::Crucible(CrucibleAction::Experiment(
                                LabAction::NextExperiment(true),
                            )));
                        }
                        let key = test_case.interval_missing.unwrap();
                        let clamped = test_case.damage.clamp(self.min_damage, self.max_damage);
                        let redness = (clamped - self.min_damage) / (self.max_damage - self.min_damage);
                        let color = [redness, 0.01, 0.01, 1.0];
                        let tension = test_case.tension;
                        return Some(LabEvent::AppStateChanged(
                            AppStateChange::SetIntervalColor {
                                key,
                                color,
                                tension,
                            },
                        ));
                    }
                    return None;
                }
                test_case.fabric.iterate(physics);
                RunningTestCase(fabric_number)
            }
        };
        None
    }

    pub fn action(&mut self, action: LabAction) -> Option<LabEvent> {
        match action {
            LabAction::GravityChanged(gravity) => self.physics.gravity = gravity,
            LabAction::MuscleChanged(nuance) => {
                self.test_case_mut().fabric.muscle_nuance = nuance;
            }
            LabAction::MusclesActive(yes) => {
                if self.stage == Paused {
                    if yes {
                        if let Some(movement) = &self.pretense_phase.muscle_movement {
                            self.stage = MuscleCycle(1.0 / movement.countdown as f32)
                        }
                    } else {
                        self.stage = Paused;
                    }
                } else {
                    self.test_case_mut().fabric.muscle_nuance = 0.5;
                    self.stage = Paused
                }
            }
            LabAction::NextExperiment(forward) => {
                let mut event = None;
                self.stage = match self.stage.clone() {
                    Paused => {
                        event = Some(LabEvent::AppStateChanged(AppStateChange::SetFabricNumber {
                            number: 1,
                            fabric_stats: self.fabric().fabric_stats(),
                        }));
                        RunningTestCase(1)
                    }
                    RunningTestCase(fabric_number) => {
                        let mut current_fabric = fabric_number;
                        if forward {
                            if current_fabric + 1 < self.test_cases.len() {
                                current_fabric += 1
                            } else {
                                current_fabric = 0;
                            }
                        } else {
                            if current_fabric > 0 {
                                current_fabric -= 1;
                            } else {
                                current_fabric = 0;
                            }
                        };
                        event = Some(LabEvent::AppStateChanged(AppStateChange::SetFabricNumber {
                            number: current_fabric,
                            fabric_stats: self.fabric().fabric_stats(),
                        }));
                        RunningTestCase(current_fabric)
                    }
                    MuscleCycle(_) => Paused,
                };
                return event;
            }
        }
        None
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &TestCase {
        &self.test_cases[self.current_fabric()]
    }

    fn test_case_mut(&mut self) -> &mut TestCase {
        let current_fabric = self.current_fabric();
        self.test_cases
            .get_mut(current_fabric)
            .expect("a current fabric")
    }

    fn current_fabric(&self) -> usize {
        match self.stage {
            Paused => 0,
            MuscleCycle(_) => 0,
            RunningTestCase(fabric_number) => fabric_number,
        }
    }

    fn min_damage(tension: bool) -> f32 {
        if tension {
            100.0
        } else {
            300.0
        }
    }

    fn max_damage(tension: bool) -> f32 {
        if tension {
            500.0
        } else {
            2000.0
        }
    }
}
