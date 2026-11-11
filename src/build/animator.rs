use crate::build::dsl::animate_phase::{Actuator, ActuatorAttachment, AnimatePhase, Waveform};
use crate::build::dsl::joint_names::Link;
use crate::fabric::physics::Physics;
use crate::fabric::interval::{Role, Span};
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Percent, Seconds};
use crate::Age;
use glam::Vec3;
use std::f32::consts::PI;

/// Oscillator that tracks phase from 0.0 to 1.0 over one period
struct Oscillator {
    phase: f32, // Current phase [0.0, 1.0)
    phase_per_tick: f32,
}

impl Oscillator {
    fn new(period: Seconds) -> Self {
        let ticks_per_cycle = period / Age::iteration_duration();
        Self {
            phase: 0.0,
            phase_per_tick: 1.0 / ticks_per_cycle,
        }
    }

    fn set_period(&mut self, period: Seconds) {
        let ticks_per_cycle = period / Age::iteration_duration();
        self.phase_per_tick = 1.0 / ticks_per_cycle;
    }

    fn tick(&mut self) {
        self.phase += self.phase_per_tick;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }

    /// Returns contraction value in range [0, 1] based on waveform at a specific phase
    fn value_at_phase(&self, phase: f32, waveform: Waveform) -> f32 {
        match waveform {
            Waveform::Sine => {
                // Sine wave: 0 at phase=0, 1 at phase=0.5, 0 at phase=1
                (1.0 - (phase * 2.0 * PI).cos()) / 2.0
            }
            Waveform::Pulse { duty_cycle } => {
                // Square wave: 1 during "on" portion, 0 during "off"
                if phase < duty_cycle.as_factor() {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

struct ActuatorInterval {
    id: IntervalKey,
    rest_length: Meters,
    contracted_length: Meters,
    phase_offset: f32,
    anchor_joint: Option<JointKey>,
    /// Span to restore for an interval that existed before the animation.
    original_span: Option<Span>,
}

pub struct Animator {
    oscillator: Oscillator,
    period: Seconds,
    waveform: Waveform,
    actuators: Vec<ActuatorInterval>,
}

impl Animator {
    pub fn new(animate_phase: AnimatePhase, fabric: &mut Fabric) -> Self {
        let contraction_factor = 1.0 - animate_phase.amplitude.as_factor();
        let period = animate_phase.period;
        let stiffness = animate_phase.stiffness;
        let actuators = Self::create_actuators(
            fabric,
            &animate_phase.actuators,
            contraction_factor,
            stiffness,
        );

        Self {
            oscillator: Oscillator::new(period),
            period,
            waveform: animate_phase.waveform,
            actuators,
        }
    }

    pub fn period(&self) -> Seconds {
        self.period
    }

    pub fn adjust_period(&mut self, factor: f32) {
        self.period = self.period * factor;
        self.oscillator.set_period(self.period);
    }

    /// The joints at each end of each of a link's images.
    fn link_joints(fabric: &Fabric, link: &Link) -> Vec<(JointKey, JointKey)> {
        let pairs = link.pairs(fabric.label_symmetry.as_ref()).unwrap_or_else(|failure| panic!("actuator {failure}"));
        let joint = |label| fabric.joint_key(label).unwrap_or_else(|| panic!("actuator joint not found: {label}"));
        pairs.into_iter().map(|(alpha, omega)| (joint(alpha), joint(omega))).collect()
    }

    fn create_actuators(
        fabric: &mut Fabric,
        actuators: &[Actuator],
        contraction_factor: f32,
        stiffness: Percent,
    ) -> Vec<ActuatorInterval> {
        let mut result = Vec::new();

        for actuator in actuators {
            let phase_offset = actuator.phase_offset.as_factor();
            match &actuator.attachment {
                ActuatorAttachment::Between { link } => {
                    for (alpha_key, omega_key) in Self::link_joints(fabric, link) {
                        let rest_length = fabric.distance(alpha_key, omega_key);
                        let id = fabric.create_slack_interval(alpha_key, omega_key, Role::Pulling);
                        if let Some(interval) = fabric.intervals.get_mut(id) {
                            interval.stiffness = stiffness;
                        }
                        result.push(ActuatorInterval {
                            id,
                            rest_length,
                            contracted_length: rest_length * contraction_factor,
                            phase_offset,
                            anchor_joint: None,
                            original_span: None,
                        });
                    }
                }
                ActuatorAttachment::Existing { link } => {
                    for (alpha_key, omega_key) in Self::link_joints(fabric, link) {
                        let id = fabric
                            .joining((alpha_key, omega_key))
                            .unwrap_or_else(|| panic!("no interval along {link}"));
                        let interval = &fabric.intervals[id];
                        let rest_length = interval.ideal();
                        result.push(ActuatorInterval {
                            id,
                            rest_length,
                            contracted_length: rest_length * contraction_factor,
                            phase_offset,
                            anchor_joint: None,
                            original_span: Some(interval.span),
                        });
                    }
                }
                ActuatorAttachment::ToSurface { joint, point } => {
                    let joint_key = fabric
                        .joint_key_by_path(joint)
                        .unwrap_or_else(|| panic!("actuator joint not found: {joint}"));
                    let anchor_point = Vec3::new(point.0, 0.0, point.1);
                    let anchor_key = fabric.create_joint(anchor_point);
                    let rest_length = fabric.distance(joint_key, anchor_key);
                    let id = fabric.create_slack_interval(joint_key, anchor_key, Role::Pulling);
                    if let Some(interval) = fabric.intervals.get_mut(id) {
                        interval.stiffness = stiffness;
                    }
                    result.push(ActuatorInterval {
                        id,
                        rest_length,
                        contracted_length: rest_length * contraction_factor,
                        phase_offset,
                        anchor_joint: Some(anchor_key),
                        original_span: None,
                    });
                }
            }
        }

        result
    }

    pub fn remove_actuators(&self, fabric: &mut Fabric) {
        // First restore or remove intervals, then anchor joints
        for actuator in &self.actuators {
            match actuator.original_span {
                Some(span) => fabric.intervals[actuator.id].span = span,
                None => {
                    fabric.remove_interval(actuator.id);
                }
            }
        }
        // Remove anchor joints (with SlotMap, order doesn't matter)
        for anchor_key in self.actuators.iter().filter_map(|a| a.anchor_joint) {
            fabric.remove_joint(anchor_key);
        }
    }

    fn update_actuator_lengths(&self, fabric: &mut Fabric) {
        for actuator in &self.actuators {
            if let Some(interval) = fabric.intervals.get_mut(actuator.id) {
                // Apply phase offset to get actuator-specific phase
                let phase_with_offset = (self.oscillator.phase + actuator.phase_offset) % 1.0;
                let contraction = self
                    .oscillator
                    .value_at_phase(phase_with_offset, self.waveform);
                let length = actuator.rest_length * (1.0 - contraction)
                    + actuator.contracted_length * contraction;
                interval.span = Span::Fixed { length };
            }
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric, physics: &Physics, iterations: usize) {
        for _ in 0..iterations {
            self.oscillator.tick();
            self.update_actuator_lengths(fabric);
            fabric.iterate(physics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;

    /// Each plan's animation runs headless: its actuators name joints its
    /// fabric has, one for each image of each link (Headless Hug's four guy
    /// lines, Open Claw's three legs). Through one period the figure moves
    /// and stays finite, and taking the actuators off leaves its intervals
    /// as they were.
    #[test]
    fn animations_run_headless() {
        use crate::fabric::physics::presets::ANIMATING;
        for (name, count) in [(FabricName::HeadlessHug, 4), (FabricName::OpenClaw, 3)] {
            let plan = fabric_library::get_fabric_plan(name);
            let animate = plan.animate_phase.clone().expect("the plan animates");
            let mut executor = FabricPlanExecutor::new(plan);
            while !executor.is_complete() {
                let _ = executor.iterate();
            }
            let physics = Physics { surface: executor.physics.surface, ..ANIMATING };
            let fabric = &mut executor.fabric;
            let (intervals, before) = (fabric.intervals.len(), fabric.joints.values().map(|j| j.location).collect::<Vec<_>>());
            let period = animate.period;
            let mut animator = Animator::new(animate, fabric);
            assert_eq!(animator.actuators.len(), count, "{name}");
            animator.iterate(fabric, &physics, (period / Age::iteration_duration()) as usize);
            let moved = before.iter().zip(fabric.joints.values()).map(|(a, j)| a.distance(j.location)).fold(0.0, f32::max);
            assert!(fabric.joints.values().all(|j| j.location.is_finite()), "{name} diverged");
            assert!(moved > 0.001, "{name} didn't move");
            animator.remove_actuators(fabric);
            assert_eq!(fabric.intervals.len(), intervals, "{name}");
        }
    }
}
