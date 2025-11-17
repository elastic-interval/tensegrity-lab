use crate::build::tenscript::converge_phase::ConvergePhase;
use crate::crucible_context::CrucibleContext;
use crate::units::Seconds;
use crate::LabEvent;
use crate::StateChange::*;
use std::time::Instant;

/// Handles the convergence phase where the fabric settles into equilibrium
/// Gradually increases damping over the specified time period
pub struct Converger {
    duration: Seconds,
    start_time: Instant,
}

impl Converger {
    pub fn new(converge_phase: &ConvergePhase) -> Self {
        Self {
            duration: converge_phase.seconds,
            start_time: Instant::now(),
        }
    }

    pub fn iterate(&self, context: &mut CrucibleContext) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let progress = (elapsed / self.duration.0).min(1.0);
        
        // Gradually increase damping as we approach the end of convergence time
        // This gives the fabric time to settle naturally before freezing
        context.physics.update_convergence_progress(progress);
        
        for _ in context.physics.iterations() {
            context.fabric.iterate(context.physics);
        }
        
        // Check if we've reached the end of convergence time
        if elapsed >= self.duration.0 {
            // Zero out velocities to prevent accumulated velocity artifacts
            context.fabric.zero_velocities();
            context.fabric.frozen = true;
            context.transition_to(crate::crucible::Stage::Viewing);
            
            context.send_event(LabEvent::UpdateState(SetStageLabel("Viewing".to_string())));
            context.send_event(LabEvent::UpdateState(SetFabricStats(
                Some(context.fabric.stats_with_convergence(context.physics))
            )));
        }
    }
}
