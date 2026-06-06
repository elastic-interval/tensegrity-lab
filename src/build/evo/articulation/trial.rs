//! Step / hold / release evaluation of an articulating brick.
//!
//! Unlike the generic continuous-drive `ActiveTrial`, articulation needs
//! to observe three distinct poses — rest, deformed, returned — so the
//! trial is a small phase machine:
//!
//! 1. **Settle** with actuators dormant → capture the rest pose, confirm
//!    it is a stable, finite, properly-tensioned equilibrium.
//! 2. **Hold** the actuators contracted → capture the deformed pose and
//!    the actuator effort (how hard the muscles had to pull).
//! 3. **Release** the actuators back to rest → capture the returned pose.
//!
//! The same machine is stepped per-frame by the visual runner and run to
//! completion by the headless `evaluate`.

use crate::build::evo::articulation::metric::{pose, shape_distance};
use crate::build::evo::articulation::structure::BrickStructure;
use crate::fabric::interval::{Role, Span};
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Unit};
use crate::Age;
use glam::Vec3;

/// KE below this counts as settled. Under the heavily-damped BAKING
/// physics every structure reaches a low-KE equilibrium (~0.01), so this
/// is really a divergence catch — the *pretension* gate below is what
/// distinguishes a valid tensegrity from a floppy one. Tuned against the
/// seed bricks; see the diagnostic test.
const SETTLED_KE: f32 = 0.1;
/// A push that has lost compression (strain above this) or a pull/radial
/// that has gone slack (strain below the negative of this) fails the
/// pretension gate.
const SLACK_EPS: f32 = 0.02;

#[derive(Clone, Debug)]
pub struct ArticulationConfig {
    pub physics: Physics,
    pub settle: f32,
    pub hold: f32,
    pub release: f32,
}

impl ArticulationConfig {
    pub fn new(physics: Physics) -> Self {
        Self {
            physics,
            settle: 1.0,
            hold: 1.5,
            release: 1.5,
        }
    }

    fn iters(secs: f32) -> usize {
        (secs / Age::iteration_duration()) as usize
    }
}

#[derive(Clone, Debug, Default)]
pub struct ArticulationOutcome {
    /// Rest pose settled to low kinetic energy.
    pub settled: bool,
    /// No push lost compression and no pull/radial went slack at rest.
    pub tensioned: bool,
    /// No NaN/inf crept into joint positions.
    pub finite: bool,
    /// Shape change from rest to the deformed (held) pose, metres.
    pub deformation: f32,
    /// Shape change from rest to the returned (released) pose, metres.
    pub return_error: f32,
    /// Total actuator effort at hold: sum of actuator tension strain.
    pub effort: f32,
    /// Diagnostics.
    pub rest_ke: f32,
    pub max_strain: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Settle,
    Hold,
    Release,
    Done,
}

struct ActuatorRef {
    key: IntervalKey,
    rest: f32,
    contracted: f32,
}

pub struct ArticulationTrial {
    pub fabric: Fabric,
    physics: Physics,
    structural_joints: Vec<JointKey>,
    actuators: Vec<ActuatorRef>,
    phase: Phase,
    phase_iter: usize,
    settle_iters: usize,
    hold_iters: usize,
    release_iters: usize,
    rest_pose: Vec<Vec3>,
    deformed_pose: Vec<Vec3>,
    outcome: ArticulationOutcome,
}

impl ArticulationTrial {
    pub fn new(structure: &BrickStructure, config: &ArticulationConfig) -> Self {
        let expressed = structure.express("Articulation".to_string());
        let actuators = expressed
            .actuators
            .iter()
            .zip(&structure.actuators)
            .map(|(&key, spec)| {
                let rest = expressed.fabric.interval(key).ideal().f32();
                ActuatorRef {
                    key,
                    rest,
                    contracted: rest * spec.contraction,
                }
            })
            .collect();
        Self {
            fabric: expressed.fabric,
            physics: config.physics.clone(),
            structural_joints: expressed.structural_joints,
            actuators,
            phase: Phase::Settle,
            phase_iter: 0,
            settle_iters: ArticulationConfig::iters(config.settle),
            hold_iters: ArticulationConfig::iters(config.hold),
            release_iters: ArticulationConfig::iters(config.release),
            rest_pose: Vec::new(),
            deformed_pose: Vec::new(),
            outcome: ArticulationOutcome {
                finite: true,
                ..Default::default()
            },
        }
    }

    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Overall progress 0..1 across all three phases (for the status bar).
    pub fn progress(&self) -> f32 {
        let total = (self.settle_iters + self.hold_iters + self.release_iters).max(1);
        let done = match self.phase {
            Phase::Settle => 0,
            Phase::Hold => self.settle_iters,
            Phase::Release => self.settle_iters + self.hold_iters,
            Phase::Done => total,
        };
        ((done + self.phase_iter) as f32 / total as f32).min(1.0)
    }

    fn set_actuators(&mut self, contracted: bool) {
        for actuator in &self.actuators {
            if let Some(interval) = self.fabric.intervals.get_mut(actuator.key) {
                let length = if contracted {
                    actuator.contracted
                } else {
                    actuator.rest
                };
                interval.span = Span::Fixed {
                    length: Meters(length),
                };
            }
        }
    }

    fn capture_pose(&self) -> Vec<Vec3> {
        pose(&self.fabric, &self.structural_joints)
    }

    fn all_finite(&self) -> bool {
        self.fabric
            .joints
            .values()
            .all(|j| j.location.is_finite())
    }

    /// True if every push is still compressed and every pull/radial still
    /// in tension (actuators excluded — they sit at rest).
    fn tensioned(&self) -> bool {
        let actuator_keys: Vec<IntervalKey> = self.actuators.iter().map(|a| a.key).collect();
        self.fabric.intervals.iter().all(|(key, interval)| {
            if actuator_keys.contains(&key) {
                return true;
            }
            let Some(reading) = self.fabric.interval_reading(key) else {
                return true;
            };
            if interval.has_role(Role::Pushing) {
                reading.strain < SLACK_EPS
            } else {
                reading.strain > -SLACK_EPS
            }
        })
    }

    fn actuator_effort(&self) -> f32 {
        self.actuators
            .iter()
            .filter_map(|a| self.fabric.interval_reading(a.key))
            .map(|r| r.strain.max(0.0))
            .sum()
    }

    /// Advance one physics tick; returns true while still running.
    pub fn step(&mut self) -> bool {
        if self.phase == Phase::Done {
            return false;
        }
        self.fabric.iterate(&self.physics);
        self.phase_iter += 1;
        if self.fabric.stats.max_strain > self.outcome.max_strain {
            self.outcome.max_strain = self.fabric.stats.max_strain;
        }
        match self.phase {
            Phase::Settle if self.phase_iter >= self.settle_iters => {
                self.outcome.rest_ke = self.fabric.kinetic_energy();
                self.outcome.settled = self.outcome.rest_ke < SETTLED_KE;
                self.outcome.finite = self.all_finite();
                self.outcome.tensioned = self.tensioned();
                self.rest_pose = self.capture_pose();
                self.set_actuators(true);
                self.phase = Phase::Hold;
                self.phase_iter = 0;
            }
            Phase::Hold if self.phase_iter >= self.hold_iters => {
                self.deformed_pose = self.capture_pose();
                self.outcome.effort = self.actuator_effort();
                self.outcome.deformation = shape_distance(&self.rest_pose, &self.deformed_pose);
                self.set_actuators(false);
                self.phase = Phase::Release;
                self.phase_iter = 0;
            }
            Phase::Release if self.phase_iter >= self.release_iters => {
                let returned = self.capture_pose();
                self.outcome.return_error = shape_distance(&self.rest_pose, &returned);
                self.outcome.finite &= self.all_finite();
                self.phase = Phase::Done;
            }
            _ => {}
        }
        self.phase != Phase::Done
    }

    pub fn step_batch(&mut self, count: usize) -> bool {
        for _ in 0..count {
            if !self.step() {
                return false;
            }
        }
        true
    }

    pub fn outcome(&self) -> ArticulationOutcome {
        self.outcome.clone()
    }
}

/// Run a full step/hold/release trial headlessly and return the outcome.
pub fn evaluate(structure: &BrickStructure, config: &ArticulationConfig) -> ArticulationOutcome {
    let mut trial = ArticulationTrial::new(structure, config);
    let max_iters = trial.settle_iters + trial.hold_iters + trial.release_iters + 8;
    let mut guard = 0;
    while trial.step() {
        guard += 1;
        if guard > max_iters {
            break;
        }
    }
    trial.outcome()
}
