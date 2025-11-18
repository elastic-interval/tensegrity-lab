use crate::build::tenscript::animate_phase::{AnimatePhase, MuscleDirection};
use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Span;
use crate::fabric::UniqueId;
use crate::ITERATIONS_PER_FRAME;
use crate::TICK_DURATION;
use cgmath::InnerSpace;

/// Animation cycle phase - whether muscles are contracting or relaxing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationPhase {
    Relaxing,    // Moving from contracted to rest length
    Contracting, // Moving from rest to contracted length
}

/// Handles animation of muscle intervals
///
/// The Animator wraps specified intervals and progressively adjusts their ideal
/// lengths to create animation. Outside of the Animator, the system is unaware
/// of this "sorcery" - it just sees intervals with changing ideal lengths.
pub struct Animator {
    muscle_nuance: f32,
    animation_phase: AnimationPhase,
    cycle_ticks: f32, // Number of ticks for one complete animation cycle
}

impl Animator {
    /// Create a new Animator with the animate phase specification
    pub fn new(animate_phase: AnimatePhase, context: &mut CrucibleContext) -> Self {
        let contraction = animate_phase.contraction.unwrap_or(0.9);

        // Calculate cycle_ticks from frequency (Hz = cycles/second)
        // ticks_per_cycle = seconds_per_cycle / seconds_per_tick
        //                 = (1 / frequency_hz) / tick_duration_seconds
        let tick_duration_seconds = TICK_DURATION.as_secs_f32();
        let cycle_ticks = 1.0 / (animate_phase.frequency_hz * tick_duration_seconds);

        // Wrap the specified intervals in Muscle spans
        Self::wrap_muscles(context, &animate_phase.muscle_intervals, contraction);

        Self {
            muscle_nuance: 0.5,
            animation_phase: AnimationPhase::Relaxing,
            cycle_ticks,
        }
    }

    /// Wrap specified intervals in Muscle spans
    /// This converts Fixed intervals into Muscles based on the animate phase specification
    fn wrap_muscles(
        context: &mut CrucibleContext,
        muscle_intervals: &[(UniqueId, UniqueId, MuscleDirection)],
        contraction: f32,
    ) {
        for &(alpha_id, omega_id, direction) in muscle_intervals {
            // Find the interval with these endpoints
            for interval_opt in context.fabric.intervals.iter_mut() {
                if let Some(interval) = interval_opt {
                    let matches = (interval.alpha_index == alpha_id.0
                        && interval.omega_index == omega_id.0)
                        || (interval.alpha_index == omega_id.0
                            && interval.omega_index == alpha_id.0);

                    if matches {
                        if let Span::Fixed {
                            length: rest_length,
                        } = interval.span
                        {
                            let contracted_length = rest_length * contraction;
                            interval.span = Span::Muscle {
                                rest_length,
                                contracted_length,
                                reverse: direction == MuscleDirection::Omega,
                            };
                        }
                    }
                }
            }
        }
    }

    /// Unwrap muscles back to Fixed spans
    /// This is called when transitioning from Animating back to Viewing
    pub fn unwrap_muscles(context: &mut CrucibleContext) {
        for interval_opt in context.fabric.intervals.iter_mut() {
            if let Some(interval) = interval_opt {
                if let Span::Muscle { rest_length, .. } = interval.span {
                    interval.span = Span::Fixed {
                        length: rest_length,
                    };
                }
            }
        }
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        // Update muscle_nuance based on animation phase
        let increment = 1.0 / self.cycle_ticks
            * match self.animation_phase {
                AnimationPhase::Relaxing => 1.0,
                AnimationPhase::Contracting => -1.0,
            };
        self.muscle_nuance += increment;

        // Reverse direction at boundaries
        if self.muscle_nuance <= 0.0 {
            self.muscle_nuance = 0.0;
            self.animation_phase = AnimationPhase::Relaxing;
        } else if self.muscle_nuance >= 1.0 {
            self.muscle_nuance = 1.0;
            self.animation_phase = AnimationPhase::Contracting;
        }

        // Iterate physics - muscles will use the current muscle_nuance
        // to calculate their ideal lengths
        for _ in 0..ITERATIONS_PER_FRAME {
            self.iterate_fabric_with_muscles(context);
        }
    }

    /// Iterate the fabric, passing muscle_nuance to intervals
    fn iterate_fabric_with_muscles(&self, context: &mut CrucibleContext) {
        if context.fabric.frozen {
            return;
        }

        // Reset stats for this iteration
        context.fabric.stats.reset();

        for joint in &mut context.fabric.joints {
            joint.reset();
        }

        // Accumulate strain stats during interval iteration
        for interval_opt in context.fabric.intervals.iter_mut().filter(|i| i.is_some()) {
            let interval = interval_opt.as_mut().unwrap();
            interval.iterate(
                &mut context.fabric.joints,
                &context.fabric.progress,
                self.muscle_nuance,
                context.fabric.scale,
                context.physics,
            );

            // Accumulate strain (zero-cost pass-through)
            context.fabric.stats.accumulate_strain(interval.strain);
        }

        let elapsed = context.fabric.age.tick_scaled(context.physics.time_scale());

        // Check for excessive speed and accumulate velocity/energy stats
        const MAX_SPEED_SQUARED: f32 = 1000.0 * 1000.0; // (mm per tick)²
        let mut max_speed_squared = 0.0;

        for joint in context.fabric.joints.iter_mut() {
            joint.iterate(context.physics, context.fabric.scale);

            // Accumulate stats (zero-cost - already computing these values)
            let speed_squared = joint.velocity.magnitude2();
            let mass = *joint.accumulated_mass;

            context.fabric.stats.accumulate_joint(mass, speed_squared);
            context.fabric.stats.update_max_speed_squared(speed_squared);

            if speed_squared > max_speed_squared {
                max_speed_squared = speed_squared;
            }
        }

        // Finalize stats (compute sqrt once at the end)
        context.fabric.stats.finalize();

        if max_speed_squared > MAX_SPEED_SQUARED || max_speed_squared.is_nan() {
            eprintln!(
                "Excessive speed detected: {:.2} mm/tick - freezing fabric",
                max_speed_squared.sqrt()
            );
            context.fabric.zero_velocities();
            context.fabric.frozen = true;
            return;
        }

        // Handle progress completion
        if context.fabric.progress.step(elapsed) {
            // final step - finalize any ongoing transitions
            for interval_opt in context.fabric.intervals.iter_mut().filter(|i| i.is_some()) {
                let interval = interval_opt.as_mut().unwrap();
                match &mut interval.span {
                    Span::Fixed { .. } => {}
                    Span::Pretenst { finished, .. } => {
                        *finished = true;
                    }
                    Span::Approaching { target_length, .. } => {
                        interval.span = Span::Approaching {
                            start_length: *target_length,
                            target_length: *target_length,
                        };
                    }
                    Span::Muscle { .. } => {}
                }
            }
        }
    }
}
