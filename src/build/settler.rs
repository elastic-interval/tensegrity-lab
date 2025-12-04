use crate::build::dsl::settle_phase::SettlePhase;
use crate::crucible_context::CrucibleContext;
use crate::units::Seconds;
use crate::LabEvent;
use crate::StateChange::*;
use std::time::Instant;

/// Handles the settling phase where the fabric settles into equilibrium
/// Gradually increases damping over the specified time period
pub struct Settler {
    duration: Seconds,
    start_time: Instant,
}

impl Settler {
    pub fn new(settle_phase: &SettlePhase) -> Self {
        Self {
            duration: settle_phase.seconds,
            start_time: Instant::now(),
        }
    }

    pub fn iterate(&self, context: &mut CrucibleContext) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let progress = (elapsed / self.duration.0).min(1.0);

        // Update stage label to show settling progress
        let progress_pct = (progress * 100.0) as u32;
        context.send_event(LabEvent::UpdateState(SetStageLabel(
            format!("Settling {}%", progress_pct)
        )));

        // Gradually increase damping as we approach the end of settling time
        // This gives the fabric time to settle naturally before freezing
        context.physics.update_settling_progress(progress);

        for _ in 0..1000 {  // Nominal value, outer loop adjusts dynamically
            context.fabric.iterate(context.physics);
        }

        // Check if we've reached the end of settling time
        if elapsed >= self.duration.0 {
            // Zero out velocities to prevent accumulated velocity artifacts
            context.fabric.zero_velocities();
            context.transition_to(crate::crucible::Stage::Viewing);
            context.send_event(LabEvent::UpdateState(SetStageLabel("Viewing".to_string())));
        }
    }
}
