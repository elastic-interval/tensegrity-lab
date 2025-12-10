use crate::build::dsl::animate_phase::{AnimatePhase, Actuator, ActuatorAttachment, ActuatorSpec, Waveform};
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::{Role, Span};
use crate::fabric::UniqueId;
use crate::units::Percent;
use crate::ITERATION_DURATION;
use cgmath::Point3;
use std::f32::consts::PI;

/// Oscillator that tracks phase from 0.0 to 1.0 over one period
struct Oscillator {
    phase: f32,           // Current phase [0.0, 1.0)
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

    /// Returns contraction value in range [0, 1] based on waveform
    fn value(&self, waveform: Waveform) -> f32 {
        match waveform {
            Waveform::Sine => {
                // Sine wave: 0 at phase=0, 1 at phase=0.5, 0 at phase=1
                (1.0 - (self.phase * 2.0 * PI).cos()) / 2.0
            }
            Waveform::Pulse { duty_cycle } => {
                // Square wave: 1 during "on" portion, 0 during "off"
                if self.phase < duty_cycle { 1.0 } else { 0.0 }
            }
        }
    }
}

struct ActuatorInterval {
    id: UniqueId,
    rest_length: f32,
    contracted_length: f32,
    direction: ActuatorSpec,
    anchor_joint: Option<usize>,
}

pub struct Animator {
    oscillator: Oscillator,
    period_secs: f32,
    waveform: Waveform,
    actuators: Vec<ActuatorInterval>,
}

impl Animator {
    pub fn new(animate_phase: AnimatePhase, context: &mut CrucibleContext) -> Self {
        let contraction_factor = animate_phase.amplitude.contraction_factor();
        let period_secs = animate_phase.period.0;
        let stiffness = animate_phase.stiffness;
        let actuators = Self::create_actuators(context, &animate_phase.actuators, contraction_factor, stiffness);

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
        let mut result = Vec::new();

        for actuator in actuators {
            match &actuator.attachment {
                ActuatorAttachment::Between { alpha, omega } => {
                    let rest_length = context.fabric.distance(*alpha, *omega);
                    let id = context.fabric.create_interval(
                        *alpha,
                        *omega,
                        rest_length,
                        Role::Pulling,
                    );
                    // Start slack: set span to Fixed at current distance
                    // Set stiffness to reduce jiggling with rapid waveforms
                    if let Some(interval) = &mut context.fabric.intervals[id.0] {
                        interval.span = Span::Fixed { length: rest_length };
                        interval.stiffness = stiffness;
                    }
                    result.push(ActuatorInterval {
                        id,
                        rest_length,
                        contracted_length: rest_length * contraction_factor,
                        direction: actuator.direction,
                        anchor_joint: None,
                    });
                }
                ActuatorAttachment::ToSurface { joint, surface } => {
                    // Create anchor joint at surface position (x, 0, z)
                    let anchor_point = Point3::new(surface.0, 0.0, surface.1);
                    let anchor_index = context.fabric.create_joint(anchor_point);

                    // Create actuator interval from fabric joint to anchor
                    let rest_length = context.fabric.distance(*joint, anchor_index);
                    let id = context.fabric.create_interval(
                        *joint,
                        anchor_index,
                        rest_length,
                        Role::Pulling,
                    );

                    // Start slack: set span to Fixed at current distance
                    // Set stiffness to reduce jiggling with rapid waveforms
                    if let Some(interval) = &mut context.fabric.intervals[id.0] {
                        interval.span = Span::Fixed { length: rest_length };
                        interval.stiffness = stiffness;
                    }

                    result.push(ActuatorInterval {
                        id,
                        rest_length,
                        contracted_length: rest_length * contraction_factor,
                        direction: actuator.direction,
                        anchor_joint: Some(anchor_index),
                    });
                }
            }
        }

        result
    }

    pub fn remove_actuators(&self, context: &mut CrucibleContext) {
        // First remove intervals, then anchor joints (in reverse order to avoid index shifts)
        for actuator in &self.actuators {
            context.fabric.remove_interval(actuator.id);
        }
        // Collect anchor joints and sort descending to remove from highest index first
        let mut anchors: Vec<usize> = self.actuators
            .iter()
            .filter_map(|a| a.anchor_joint)
            .collect();
        anchors.sort_by(|a, b| b.cmp(a));
        for anchor in anchors {
            context.fabric.remove_joint(anchor);
        }
    }

    fn update_actuator_lengths(&self, context: &mut CrucibleContext) {
        let oscillator_value = self.oscillator.value(self.waveform);
        for actuator in &self.actuators {
            if let Some(interval) = &mut context.fabric.intervals[actuator.id.0] {
                // Alpha actuators contract when oscillator is high, Omega when low
                let contraction = match actuator.direction {
                    ActuatorSpec::Alpha => oscillator_value,
                    ActuatorSpec::Omega => 1.0 - oscillator_value,
                };
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
