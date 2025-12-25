use crate::build::dsl::animate_phase::{Actuator, ActuatorAttachment, AnimatePhase, Waveform};
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::{Role, Span};
use crate::fabric::{IntervalKey, JointKey};
use crate::units::Percent;
use crate::ITERATION_DURATION;
use cgmath::Point3;
use std::f32::consts::PI;

/// Oscillator that tracks phase from 0.0 to 1.0 over one period
struct Oscillator {
    phase: f32, // Current phase [0.0, 1.0)
    phase_per_tick: f32,
}

impl Oscillator {
    fn new(period_secs: f32) -> Self {
        let ticks_per_cycle = period_secs / ITERATION_DURATION.secs;
        Self {
            phase: 0.0,
            phase_per_tick: 1.0 / ticks_per_cycle,
        }
    }

    fn set_period(&mut self, period_secs: f32) {
        let ticks_per_cycle = period_secs / ITERATION_DURATION.secs;
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
    rest_length: f32,
    contracted_length: f32,
    phase_offset: f32,
    anchor_joint: Option<JointKey>,
}

pub struct Animator {
    oscillator: Oscillator,
    period_secs: f32,
    waveform: Waveform,
    actuators: Vec<ActuatorInterval>,
}

impl Animator {
    pub fn new(animate_phase: AnimatePhase, context: &mut CrucibleContext) -> Self {
        let contraction_factor = 1.0 - animate_phase.amplitude.as_factor();
        let period_secs = animate_phase.period.0;
        let stiffness = animate_phase.stiffness;
        let actuators = Self::create_actuators(
            context,
            &animate_phase.actuators,
            contraction_factor,
            stiffness,
        );

        Self {
            oscillator: Oscillator::new(period_secs),
            period_secs,
            waveform: animate_phase.waveform,
            actuators,
        }
    }

    pub fn period_secs(&self) -> f32 {
        self.period_secs
    }

    pub fn adjust_period(&mut self, factor: f32) {
        self.period_secs *= factor;
        self.oscillator.set_period(self.period_secs);
    }

    fn create_actuators(
        context: &mut CrucibleContext,
        actuators: &[Actuator],
        contraction_factor: f32,
        stiffness: Percent,
    ) -> Vec<ActuatorInterval> {
        let fabric = &mut context.fabric;
        let mut result = Vec::new();

        for actuator in actuators {
            let phase_offset = actuator.phase_offset.as_factor();
            match &actuator.attachment {
                ActuatorAttachment::Between { joint_a, joint_b } => {
                    let Some(alpha_key) = fabric.joint_key_by_path(joint_a) else {
                        continue;
                    };
                    let Some(omega_key) = fabric.joint_key_by_path(joint_b) else {
                        continue;
                    };
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
                    });
                }
                ActuatorAttachment::ToSurface { joint, point } => {
                    let Some(joint_key) = fabric.joint_key_by_path(joint) else {
                        continue;
                    };
                    let anchor_point = Point3::new(point.0, 0.0, point.1);
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
                    });
                }
            }
        }

        result
    }

    pub fn remove_actuators(&self, context: &mut CrucibleContext) {
        // First remove intervals, then anchor joints
        for actuator in &self.actuators {
            context.fabric.remove_interval(actuator.id);
        }
        // Remove anchor joints (with SlotMap, order doesn't matter)
        for anchor_key in self.actuators.iter().filter_map(|a| a.anchor_joint) {
            context.fabric.remove_joint(anchor_key);
        }
    }

    fn update_actuator_lengths(&self, context: &mut CrucibleContext) {
        for actuator in &self.actuators {
            if let Some(interval) = context.fabric.intervals.get_mut(actuator.id) {
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

    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        for _ in 0..iterations_per_frame {
            self.oscillator.tick();
            self.update_actuator_lengths(context);
            context.fabric.iterate(context.physics);
        }
    }
}
