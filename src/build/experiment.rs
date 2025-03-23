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
}

const TIMEOUT_ITERATIONS: usize = 10000;
const MAX_AGE: u64 = 180000;
const MIN_DAMAGE: f32 = 100.0;
const MAX_DAMAGE: f32 = 1000.0;

#[derive(Clone)]
struct TestCase {
    fabric: Fabric,
    interval_missing: Option<(usize, usize)>,
    damage: f32,
}

pub struct Experiment {
    default_fabric: Fabric,
    test_cases: Vec<TestCase>,
    max_damage: f32,
    stage: Stage,
    current_fabric: usize,
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
    ) -> Self {
        let interval_keys: Vec<_> = fabric
            .intervals
            .iter()
            .flat_map(|(id, interval)| {
                if interval.material == Material::PushMaterial {
                    None
                } else {
                    Some(*id)
                }
            })
            .collect();
        let case_count = interval_keys.len() + 1;
        let mut test_cases = vec![
            TestCase {
                fabric: fabric.clone(),
                interval_missing: None,
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
            test_cases,
            max_damage: 0.0,
            current_fabric: 0,
            stage: Paused,
            physics,
            pretense_phase,
            timeout_iterations: TIMEOUT_ITERATIONS,
        }
    }

    pub fn iterate(&mut self) -> Option<LabEvent> {
        let test_case = self
            .test_cases
            .get_mut(self.current_fabric)
            .expect("No test case");
        if self.current_fabric > 0 && test_case.fabric.age >= MAX_AGE {
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
                    if self.max_damage < damage {
                        self.max_damage = damage;
                    }
                    test_case.damage = damage;
                } else {
                    return Some(LabEvent::Crucible(CrucibleAction::Experiment(
                        LabAction::NextExperiment(true),
                    )));
                }
                let key = test_case.interval_missing.unwrap();
                let clamped = test_case.damage.clamp(MIN_DAMAGE, MAX_DAMAGE);
                let redness = (clamped - MIN_DAMAGE) / MAX_DAMAGE;
                let color = [redness, 0.01, 0.01, 1.0];
                return Some(LabEvent::AppStateChanged(AppStateChange::SetIntervalColor(
                    (key, color),
                )));
            }
            return None;
        }
        self.stage = match self.stage {
            Paused => {
                test_case.fabric.iterate(&self.physics);
                Paused
            }
            MuscleCycle(increment) => {
                test_case.fabric.iterate(&self.physics);
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
                if forward {
                    if self.current_fabric + 1 < self.test_cases.len() {
                        self.current_fabric += 1;
                    } else {
                        self.current_fabric = 0;
                        // #[cfg(not(target_arch = "wasm32"))]
                        // std::fs::write(
                        //     chrono::Local::now()
                        //         .format("displacements-%Y-%m-%d-%H-%M.txt")
                        //         .to_string(),
                        //     self.test_cases
                        //         .iter()
                        //         .map(
                        //             |TestCase {
                        //                  interval_missing,
                        //                  damage: displacement,
                        //                  ..
                        //              }| {
                        //                 if let Some((alpha, omega)) = interval_missing {
                        //                     format!("({alpha},{omega}), {:.1}", displacement)
                        //                 } else {
                        //                     "(0,0) 0.0".to_string()
                        //                 }
                        //             },
                        //         )
                        //         .join("\n"),
                        // )
                        // .unwrap();
                    }
                } else {
                    if self.current_fabric > 0 {
                        self.current_fabric -= 1;
                    }
                }
                return Some(LabEvent::AppStateChanged(AppStateChange::SetFabricNumber {
                    number: self.current_fabric,
                    fabric_stats: self.fabric().fabric_stats(),
                }));
            }
        }
        None
    }

    pub fn fabric(&self) -> &Fabric {
        &self.test_case().fabric
    }

    fn test_case(&self) -> &TestCase {
        &self.test_cases[self.current_fabric]
    }

    fn test_case_mut(&mut self) -> &mut TestCase {
        self.test_cases
            .get_mut(self.current_fabric)
            .expect("a current fabric")
    }
}
