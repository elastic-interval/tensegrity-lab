use crate::application::AppStateChange;
use crate::build::tester::Stage::*;
use crate::crucible::{CrucibleAction, TesterAction};
use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::messages::LabEvent;
use cgmath::InnerSpace;
use std::fmt::{Display, Formatter};
use winit::event_loop::EventLoopProxy;

#[derive(Clone, PartialEq)]
enum Stage {
    Paused,
    RunningTestCase(usize),
}

const MAX_NEW_ITERATIONS: u64 = 100000;

#[derive(Clone)]
struct TestCase {
    fabric: Fabric,
    interval_missing: Option<(usize, usize)>,
    tension: bool,
    damage: f32,
    finished: bool,
}

impl Display for TestCase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.interval_missing {
            None => {
                write!(f, "")
            }
            Some(pair) => {
                write!(f, "Missing {pair:?}")
            }
        }
    }
}

pub struct Tester {
    default_fabric: Fabric,
    min_damage: f32,
    max_damage: f32,
    test_cases: Vec<TestCase>,
    stage: Stage,
    physics: Physics,
    event_loop_proxy: EventLoopProxy<LabEvent>,
}

impl Tester {
    pub fn new(
        fabric: &Fabric,
        physics: Physics,
        tension: bool,
        event_loop_proxy: EventLoopProxy<LabEvent>,
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
                finished: false,
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
            event_loop_proxy,
        }
    }

    pub fn iterate(&mut self) {
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        let physics = &self.physics;
        self.stage = match self.stage {
            Paused => {
                let test_case = &mut self.test_cases[0];
                test_case.fabric.iterate(physics);
                Paused
            }
            RunningTestCase(fabric_number) => {
                let test_case = self
                    .test_cases
                    .get_mut(fabric_number)
                    .expect("No test case");
                let iterations = test_case.fabric.age - self.default_fabric.age;
                if fabric_number > 0 && iterations >= MAX_NEW_ITERATIONS && !test_case.finished {
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
                    let tension = test_case.tension;
                    send(LabEvent::AppStateChanged(
                        AppStateChange::SetIntervalColor {
                            key,
                            color,
                            tension,
                        },
                    ));
                    send(LabEvent::Crucible(CrucibleAction::TesterDo(
                        TesterAction::NextExperiment(true),
                    )));
                }
                test_case.fabric.iterate(physics);
                RunningTestCase(fabric_number)
            }
        };
    }

    pub fn action(&mut self, action: TesterAction) {
        let send = |lab_event: LabEvent| self.event_loop_proxy.send_event(lab_event).unwrap();
        match action {
            TesterAction::GravityChanged(gravity) => self.physics.gravity = gravity,
            TesterAction::NextExperiment(forward) => {
                self.stage = match self.stage.clone() {
                    Paused => {
                        send(LabEvent::AppStateChanged(
                            AppStateChange::SetExperimentTitle {
                                title: self.test_cases[1].to_string(),
                                fabric_stats: self.fabric().fabric_stats(),
                            },
                        ));
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
                        send(LabEvent::AppStateChanged(
                            AppStateChange::SetExperimentTitle {
                                title: self.test_cases[current_fabric].to_string(),
                                fabric_stats: self.fabric().fabric_stats(),
                            },
                        ));
                        RunningTestCase(current_fabric)
                    }
                };
            }
        }
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &TestCase {
        &self.test_cases[self.current_fabric()]
    }

    fn current_fabric(&self) -> usize {
        match self.stage {
            Paused => 0,
            RunningTestCase(fabric_number) => fabric_number,
        }
    }

    fn min_damage(tension: bool) -> f32 {
        if tension {
            100.0
        } else {
            500.0
        }
    }

    fn max_damage(tension: bool) -> f32 {
        if tension {
            500.0
        } else {
            1000.0
        }
    }
}
