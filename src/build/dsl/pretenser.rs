use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::pretenser::Stage::*;
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::interval::Span::Approaching;
use crate::fabric::physics::presets::PRETENSING;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, IntervalKey};
use crate::units::Seconds;
use crate::LabEvent::DumpCSV;
use crate::{Age, Radio};
use std::collections::HashMap;

const MIN_STRAIN: f32 = 0.01;
const MAX_STRAIN: f32 = 0.03;
const EXTENSION_SECONDS: Seconds = Seconds(0.2);
const SETTLE_SECONDS: Seconds = Seconds(0.2);

#[derive(Clone, Debug, PartialEq, Copy)]
enum Stage {
    Start,
    Slacken,
    Settling,
    Measuring,
    Extending,
    Pretenst,
}

#[derive(Clone, Debug)]
pub struct SymmetricGroup {
    #[allow(dead_code)]
    pub born: Age,
    pub intervals: Vec<IntervalKey>,
    pub avg_strain: f32,
}

impl Fabric {
    pub fn discover_symmetric_groups(&self) -> Vec<SymmetricGroup> {
        let mut by_age: HashMap<Age, Vec<IntervalKey>> = HashMap::new();
        for (key, interval) in self.intervals.iter() {
            if interval.has_role(Role::Pushing) {
                let born = self.joints[interval.alpha_key].born;
                by_age.entry(born).or_default().push(key);
            }
        }
        by_age
            .into_iter()
            .map(|(born, intervals)| SymmetricGroup {
                born,
                intervals,
                avg_strain: 0.0,
            })
            .collect()
    }

    pub fn update_group_strains(&self, groups: &mut [SymmetricGroup]) {
        for group in groups {
            let mut total = 0.0;
            let mut count = 0;
            for &key in &group.intervals {
                if let Some(interval) = self.intervals.get(key) {
                    total += interval.strain;
                    count += 1;
                }
            }
            group.avg_strain = if count > 0 { total / count as f32 } else { 0.0 };
        }
    }

    pub fn find_group_needing_extension(&self, groups: &[SymmetricGroup]) -> Option<usize> {
        let increment = self.dimensions.push_length_increment.map(|m| *m)?;

        let mut best_idx = None;
        let mut best_strain = f32::NEG_INFINITY;

        for (idx, group) in groups.iter().enumerate() {
            if group.intervals.is_empty() {
                continue;
            }
            if group.avg_strain <= -MIN_STRAIN {
                continue;
            }
            let can_extend = group.intervals.iter().all(|&key| {
                if let Some(interval) = self.intervals.get(key) {
                    let current_length = interval.ideal();
                    let estimated_new_strain = interval.strain - increment / current_length;
                    estimated_new_strain >= -MAX_STRAIN
                } else {
                    false
                }
            });
            if can_extend && group.avg_strain > best_strain {
                best_strain = group.avg_strain;
                best_idx = Some(idx);
            }
        }
        best_idx
    }

    pub fn extend_symmetric_group(&mut self, group: &SymmetricGroup) {
        let increment = match self.dimensions.push_length_increment {
            Some(m) => *m,
            None => return,
        };
        for &key in &group.intervals {
            if let Some(interval) = self.intervals.get_mut(key) {
                let current_length = interval.ideal();
                let new_length = current_length + increment;
                interval.span = Approaching {
                    start_length: current_length,
                    target_length: new_length,
                };
            }
        }
        self.progress.start(EXTENSION_SECONDS);
    }
}

fn all_groups_satisfied(groups: &[SymmetricGroup]) -> bool {
    groups
        .iter()
        .filter(|g| !g.intervals.is_empty())
        .all(|g| g.avg_strain <= -MIN_STRAIN)
}

#[derive(Clone)]
pub struct Pretenser {
    pub pretense_phase: PretensePhase,
    stage: Stage,
    radio: Radio,
    groups: Vec<SymmetricGroup>,
    settle_started_at: Option<Age>,
}

impl Pretenser {
    pub fn new(pretense_phase: PretensePhase, radio: &Radio) -> Self {
        Self {
            stage: Start,
            pretense_phase,
            radio: radio.clone(),
            groups: Vec::new(),
            settle_started_at: None,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = PRETENSING;
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        self.iterate_fabric(&mut context.fabric, context.physics);
    }

    pub fn iterate_fabric(&mut self, fabric: &mut Fabric, physics: &Physics) {
        self.stage = match self.stage {
            Start => {
                use crate::fabric::face::FaceEnding;
                let face_keys: Vec<_> = fabric.faces.keys().collect();
                for face_key in face_keys {
                    let face = fabric.face(face_key);
                    match face.ending {
                        FaceEnding::Triangle => {
                            fabric.add_face_triangle(face_key);
                        }
                        FaceEnding::Prism | FaceEnding::Radial => {}
                    }
                    fabric.remove_face(face_key);
                }
                Slacken
            }
            Slacken => {
                fabric.slacken();
                self.groups = fabric.discover_symmetric_groups();
                DumpCSV.send(&self.radio);
                self.settle_started_at = None;
                Settling
            }
            Settling => {
                for _ in 0..1000 {
                    fabric.iterate(physics);
                }
                let started = self.settle_started_at.get_or_insert(fabric.age);
                let elapsed = fabric.age.elapsed_since(*started);
                if elapsed >= SETTLE_SECONDS {
                    self.settle_started_at = None;
                    Measuring
                } else {
                    Settling
                }
            }
            Measuring => {
                fabric.update_group_strains(&mut self.groups);
                if let Some(group_idx) = fabric.find_group_needing_extension(&self.groups) {
                    fabric.extend_symmetric_group(&self.groups[group_idx]);
                    Extending
                } else if all_groups_satisfied(&self.groups) {
                    Pretenst
                } else {
                    Pretenst
                }
            }
            Extending => {
                for _ in 0..1000 {
                    fabric.iterate(physics);
                }
                if fabric.progress.is_busy() {
                    Extending
                } else {
                    self.settle_started_at = None;
                    Settling
                }
            }
            Pretenst => {
                for _ in 0..1000 {
                    fabric.iterate(physics);
                }
                Pretenst
            }
        };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Pretenst
    }

    pub fn physics(&self) -> Physics {
        self.pretense_phase.viewing_physics()
    }
}
