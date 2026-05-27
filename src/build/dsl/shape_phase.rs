use crate::build::dsl::brick_dsl::FaceLabel;
use crate::build::dsl::shape_phase::ShapeCommand::*;
use crate::build::dsl::FaceLabelBinding;
use crate::fabric::interval::Role;
use crate::fabric::joint_path::JointPath;
use crate::fabric::vulcanize::VulcanizeMode;
use crate::fabric::{Fabric, FaceKey, IntervalKey, JointKey};
use crate::units::{Meters, Percent, Seconds, Unit};
use glam::{Mat4, Quat, Vec3};
use std::collections::HashMap;

const DEFAULT_JOINER_COUNTDOWN: Seconds = Seconds(3.0);

#[derive(Debug)]
pub enum ShapeCommand {
    Noop,
    StartProgress(Seconds),
    Rigidity(f32),
    Terminate,
}

#[derive(Debug, Clone)]
pub struct ShapeStep {
    pub seconds: Seconds,
    pub action: ShapeAction,
}

#[derive(Debug, Clone)]
pub enum ShapeAction {
    Joiner {
        alpha: FaceLabel,
        omega: FaceLabel,
    },
    PointDownwards {
        labels: Vec<FaceLabel>,
    },
    Centralize,
    CentralizeAt {
        altitude: Meters,
    },
    /// Spacer over all pairs of the given labeled faces. With 2 labels
    /// this is a single interval; with N labels it's N*(N-1)/2. Each
    /// interval pushes apart if the target distance is greater than the
    /// current, otherwise pulls together.
    Spacer {
        labels: Vec<FaceLabel>,
        distance: Percent,
    },
    /// Multiple spacer groups in one window — all intervals created
    /// together so their Approaching spans interpolate concurrently.
    ParallelSpacers {
        specs: Vec<SpacerSpec>,
    },
    Anchor {
        joint_path: JointPath,
        surface: (f32, f32),
    },
    GuyLine {
        joint_path: JointPath,
        length: f32,
        surface: (f32, f32),
    },
    PrepareVulcanize {
        contraction: f32,
        mode: VulcanizeMode,
    },
    Vulcanize,
    Omit {
        alpha_path: JointPath,
        omega_path: JointPath,
    },
    Add {
        alpha_path: JointPath,
        omega_path: JointPath,
        length_factor: f32,
    },
}

#[derive(Debug, Clone)]
pub struct Joiner {
    interval: IntervalKey,
    alpha_face: FaceKey,
    omega_face: FaceKey,
}

#[derive(Debug, Clone)]
pub struct SpacerSpec {
    pub labels: Vec<FaceLabel>,
    pub distance: Percent,
}

#[derive(Debug, Clone)]
pub struct ShapePhase {
    pub steps: Vec<ShapeStep>,
    pub labels: HashMap<FaceLabel, FaceKey>,
    pub spacers: Vec<IntervalKey>,
    pub joiners: Vec<Joiner>,
    pub anchors: Vec<IntervalKey>,
    pub(crate) step_index: usize,
    pub(crate) scale: Meters,
}

impl ShapePhase {
    /// Build the unique-label lookup, panicking if any label is bound to
    /// more than one face. Call this once after the build phase finishes,
    /// before any shape step runs.
    pub fn install_labels(&mut self, bindings: &[FaceLabelBinding]) {
        for FaceLabelBinding { face_label, face_key } in bindings {
            if let Some(existing) = self.labels.insert(*face_label, *face_key) {
                panic!(
                    "Face label {face_label} bound twice (was {existing:?}, now {face_key:?}). \
                     Labels must be unique per fabric."
                );
            }
        }
    }

    fn labeled_face(&self, label: FaceLabel) -> FaceKey {
        *self.labels.get(&label).unwrap_or_else(|| {
            panic!("No face is labeled {label}")
        })
    }

    fn labeled_middle_joint(&self, fabric: &Fabric, label: FaceLabel) -> JointKey {
        fabric.face(self.labeled_face(label)).middle_joint(fabric)
    }

    /// Choose Pushing if the target distance is greater than the current,
    /// Pulling otherwise. The asymmetry follows from the physics: push
    /// intervals only push, pull intervals only pull.
    fn create_spacer(
        &mut self,
        fabric: &mut Fabric,
        alpha_joint: JointKey,
        omega_joint: JointKey,
        distance: Percent,
        seconds: Seconds,
    ) {
        let alpha_pt = fabric.joints[alpha_joint].location;
        let omega_pt = fabric.joints[omega_joint].location;
        let current = alpha_pt.distance(omega_pt);
        let target = current * distance.as_factor();
        let role = if target > current { Role::Pushing } else { Role::Pulling };
        let interval = fabric.create_approaching_interval(
            alpha_joint,
            omega_joint,
            Meters(target),
            role,
            seconds,
        );
        self.spacers.push(interval);
    }
}

impl ShapePhase {
    pub fn needs_shaping(&self) -> bool {
        !self.steps.is_empty()
    }

    pub fn shaping_step(&mut self, fabric: &mut Fabric) -> ShapeCommand {
        if let Some(countdown) = self.complete_joiners(fabric) {
            return countdown;
        }
        let Some(step) = self.steps.get(self.step_index) else {
            self.cleanup(fabric);
            return Terminate;
        };
        self.step_index += 1;
        self.execute_step(fabric, step.clone())
    }

    fn execute_step(&mut self, fabric: &mut Fabric, step: ShapeStep) -> ShapeCommand {
        let seconds = step.seconds;
        match step.action {
            ShapeAction::Joiner { alpha, omega } => {
                let alpha_face = self.labeled_face(alpha);
                let omega_face = self.labeled_face(omega);
                let alpha_joint = fabric.face(alpha_face).middle_joint(fabric);
                let omega_joint = fabric.face(omega_face).middle_joint(fabric);
                let interval = fabric.create_approaching_interval(
                    alpha_joint,
                    omega_joint,
                    Meters(0.01),
                    Role::Pulling,
                    seconds,
                );
                self.joiners.push(Joiner {
                    interval,
                    alpha_face,
                    omega_face,
                });
                StartProgress(seconds)
            }
            ShapeAction::PointDownwards { labels } => {
                let down = labels
                    .iter()
                    .map(|label| fabric.face(self.labeled_face(*label)).normal(fabric))
                    .sum::<Vec3>()
                    .normalize();
                let quaternion = Quat::from_rotation_arc(down, -Vec3::Y);
                fabric.apply_matrix4(Mat4::from_quat(quaternion));
                StartProgress(seconds)
            }
            ShapeAction::Spacer { labels, distance } => {
                let joints: Vec<JointKey> = labels
                    .iter()
                    .map(|label| self.labeled_middle_joint(fabric, *label))
                    .collect();
                for i in 0..joints.len() {
                    for j in (i + 1)..joints.len() {
                        self.create_spacer(fabric, joints[i], joints[j], distance, seconds);
                    }
                }
                StartProgress(seconds)
            }
            ShapeAction::ParallelSpacers { specs } => {
                for SpacerSpec { labels, distance } in specs {
                    let joints: Vec<JointKey> = labels
                        .iter()
                        .map(|label| self.labeled_middle_joint(fabric, *label))
                        .collect();
                    for i in 0..joints.len() {
                        for j in (i + 1)..joints.len() {
                            self.create_spacer(fabric, joints[i], joints[j], distance, seconds);
                        }
                    }
                }
                StartProgress(seconds)
            }
            ShapeAction::PrepareVulcanize { contraction, mode } => {
                fabric.prepare_vulcanize(contraction, mode);
                StartProgress(seconds)
            }
            ShapeAction::Vulcanize => {
                fabric.vulcanize(seconds);
                StartProgress(seconds)
            }
            ShapeAction::Omit {
                alpha_path,
                omega_path,
            } => {
                if let (Some(alpha_key), Some(omega_key)) = (
                    fabric.joint_key_by_path(&alpha_path),
                    fabric.joint_key_by_path(&omega_path),
                ) {
                    fabric
                        .joining((alpha_key, omega_key))
                        .map(|id| fabric.remove_interval(id));
                }
                StartProgress(seconds)
            }
            ShapeAction::Add {
                alpha_path,
                omega_path,
                length_factor,
            } => {
                if let (Some(alpha_key), Some(omega_key)) = (
                    fabric.joint_key_by_path(&alpha_path),
                    fabric.joint_key_by_path(&omega_path),
                ) {
                    let ideal = fabric.distance(alpha_key, omega_key) * length_factor;
                    fabric.create_approaching_interval(
                        alpha_key,
                        omega_key,
                        ideal,
                        Role::Pulling,
                        seconds,
                    );
                }
                StartProgress(seconds)
            }
            ShapeAction::Anchor {
                joint_path,
                surface,
            } => {
                if let Some(joint_key) = fabric.joint_key_by_path(&joint_path) {
                    let (x, z) = surface;
                    let base = fabric.create_joint(Vec3::new(x, 0.0, z));
                    let interval_key = fabric.create_approaching_interval(
                        joint_key,
                        base,
                        Meters(0.01),
                        Role::Support,
                        seconds,
                    );
                    self.anchors.push(interval_key);
                }
                StartProgress(seconds)
            }
            ShapeAction::GuyLine {
                joint_path,
                length,
                surface,
            } => {
                if let Some(joint_key) = fabric.joint_key_by_path(&joint_path) {
                    let (x, z) = surface;
                    let base = fabric.create_joint(Vec3::new(x, 0.0, z));
                    fabric.create_approaching_interval(
                        joint_key,
                        base,
                        Meters(length),
                        Role::Support,
                        seconds,
                    );
                }
                StartProgress(seconds)
            }
            ShapeAction::Centralize => {
                let translation = fabric.centralize_translation(None);
                fabric.apply_translation(translation);
                StartProgress(seconds)
            }
            ShapeAction::CentralizeAt { altitude } => {
                // Convert altitude to internal units: altitude / scale (both in meters)
                let internal_altitude = altitude.f32() / self.scale.f32();
                let translation = fabric.centralize_translation(Some(internal_altitude));
                fabric.apply_translation(translation);
                StartProgress(seconds)
            }
        }
    }

    pub fn complete_joiners(&mut self, fabric: &mut Fabric) -> Option<ShapeCommand> {
        let joiner_active = !self.joiners.is_empty();
        for Joiner {
            interval,
            alpha_face,
            omega_face,
            ..
        } in self.joiners.drain(..)
        {
            fabric.remove_interval(interval);
            fabric.join_faces(alpha_face, omega_face);
        }
        joiner_active.then_some(StartProgress(DEFAULT_JOINER_COUNTDOWN))
    }
    fn cleanup(&mut self, fabric: &mut Fabric) {
        for interval in self.spacers.drain(..) {
            fabric.remove_interval(interval);
        }
        for interval in self.anchors.drain(..) {
            fabric.remove_interval(interval);
        }
    }
}
