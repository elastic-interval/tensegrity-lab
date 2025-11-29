use crate::build::dsl::animate_phase::{AnimatePhase, Muscle, MuscleAttachment, MuscleSpec};
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::{Role, Span};
use crate::fabric::UniqueId;
use crate::ITERATION_DURATION;
use cgmath::Point3;
use std::f32::consts::PI;

/// Sine wave oscillator that produces values from 0.0 to 1.0
struct Oscillator {
    phase: f32,           // Current phase in radians [0, 2π)
    radians_per_tick: f32,
}

impl Oscillator {
    fn new(period_secs: f32) -> Self {
        let ticks_per_cycle = period_secs / ITERATION_DURATION.secs;
        Self {
            phase: 0.0,
            radians_per_tick: 2.0 * PI / ticks_per_cycle,
        }
    }

    fn set_period(&mut self, period_secs: f32) {
        let ticks_per_cycle = period_secs / ITERATION_DURATION.secs;
        self.radians_per_tick = 2.0 * PI / ticks_per_cycle;
    }

    fn tick(&mut self) {
        self.phase += self.radians_per_tick;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }
    }

    /// Returns value in range [0, 1] using a sine wave
    /// 0.0 at phase=0, 1.0 at phase=π/2, 0.0 at phase=π, etc.
    fn value(&self) -> f32 {
        (1.0 - self.phase.cos()) / 2.0
    }
}

struct MuscleInterval {
    id: UniqueId,
    rest_length: f32,
    contracted_length: f32,
    direction: MuscleSpec,
    anchor_joint: Option<usize>,
}

pub struct Animator {
    oscillator: Oscillator,
    period_secs: f32,
    muscles: Vec<MuscleInterval>,
}

impl Animator {
    pub fn new(animate_phase: AnimatePhase, context: &mut CrucibleContext) -> Self {
        let contraction_factor = animate_phase.amplitude.contraction_factor();
        let period_secs = animate_phase.period.0;
        let muscles = Self::create_muscles(context, &animate_phase.muscles, contraction_factor);

        Self {
            oscillator: Oscillator::new(period_secs),
            period_secs,
            muscles,
        }
    }

    pub fn period_secs(&self) -> f32 {
        self.period_secs
    }

    pub fn adjust_period(&mut self, factor: f32) {
        self.period_secs *= factor;
        self.oscillator.set_period(self.period_secs);
    }

    fn create_muscles(
        context: &mut CrucibleContext,
        muscles: &[Muscle],
        contraction_factor: f32,
    ) -> Vec<MuscleInterval> {
        let mut result = Vec::new();

        for muscle in muscles {
            match &muscle.attachment {
                MuscleAttachment::Between { alpha, omega } => {
                    let rest_length = context.fabric.distance(*alpha, *omega);
                    let id = context.fabric.create_interval(
                        *alpha,
                        *omega,
                        rest_length,
                        Role::Pulling,
                    );
                    // Start slack: set span to Fixed at current distance
                    if let Some(interval) = &mut context.fabric.intervals[id.0] {
                        interval.span = Span::Fixed { length: rest_length };
                    }
                    result.push(MuscleInterval {
                        id,
                        rest_length,
                        contracted_length: rest_length * contraction_factor,
                        direction: muscle.direction,
                        anchor_joint: None,
                    });
                }
                MuscleAttachment::ToSurface { joint, surface } => {
                    // Create anchor joint at surface position (x, 0, z)
                    let anchor_point = Point3::new(surface.0, 0.0, surface.1);
                    let anchor_index = context.fabric.create_joint(anchor_point);

                    // Create muscle interval from fabric joint to anchor
                    let rest_length = context.fabric.distance(*joint, anchor_index);
                    let id = context.fabric.create_interval(
                        *joint,
                        anchor_index,
                        rest_length,
                        Role::Pulling,
                    );

                    // Start slack: set span to Fixed at current distance
                    if let Some(interval) = &mut context.fabric.intervals[id.0] {
                        interval.span = Span::Fixed { length: rest_length };
                    }

                    result.push(MuscleInterval {
                        id,
                        rest_length,
                        contracted_length: rest_length * contraction_factor,
                        direction: muscle.direction,
                        anchor_joint: Some(anchor_index),
                    });
                }
            }
        }

        result
    }

    pub fn remove_muscles(&self, context: &mut CrucibleContext) {
        // First remove intervals, then anchor joints (in reverse order to avoid index shifts)
        for muscle in &self.muscles {
            context.fabric.remove_interval(muscle.id);
        }
        // Collect anchor joints and sort descending to remove from highest index first
        let mut anchors: Vec<usize> = self.muscles
            .iter()
            .filter_map(|m| m.anchor_joint)
            .collect();
        anchors.sort_by(|a, b| b.cmp(a));
        for anchor in anchors {
            context.fabric.remove_joint(anchor);
        }
    }

    fn update_muscle_lengths(&self, context: &mut CrucibleContext) {
        let oscillator_value = self.oscillator.value();
        for muscle in &self.muscles {
            if let Some(interval) = &mut context.fabric.intervals[muscle.id.0] {
                // Alpha muscles contract when oscillator is high, Omega when low
                let contraction = match muscle.direction {
                    MuscleSpec::Alpha => oscillator_value,
                    MuscleSpec::Omega => 1.0 - oscillator_value,
                };
                let length = muscle.rest_length * (1.0 - contraction)
                    + muscle.contracted_length * contraction;
                interval.span = Span::Fixed { length };
            }
        }
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        for _ in 0..iterations_per_frame {
            self.oscillator.tick();
            self.update_muscle_lengths(context);
            context.fabric.iterate(context.physics);
        }
    }
}
