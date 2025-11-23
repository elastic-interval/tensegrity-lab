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
        
        // Update stage label to show convergence progress
        let progress_pct = (progress * 100.0) as u32;
        context.send_event(LabEvent::UpdateState(SetStageLabel(
            format!("Converging {}%", progress_pct)
        )));
        
        // Gradually increase damping as we approach the end of convergence time
        // This gives the fabric time to settle naturally before freezing
        context.physics.update_convergence_progress(progress);
        
        for _ in 0..1000 {  // Nominal value, outer loop adjusts dynamically
            context.fabric.iterate(context.physics);
        }
        
        // Check if we've reached the end of convergence time
        if elapsed >= self.duration.0 {
            // Zero out velocities to prevent accumulated velocity artifacts
            context.fabric.zero_velocities();
            context.fabric.frozen = true;
            context.transition_to(crate::crucible::Stage::Viewing);
            
            // Calculate fresh stats with convergence data
            let stats_with_dynamics = context.fabric.stats_with_dynamics(context.physics);
            
            // Send FabricBuilt with convergence stats - this will trigger Viewing state
            context.queue_event(LabEvent::FabricBuilt(stats_with_dynamics));
            context.send_event(LabEvent::UpdateState(SetStageLabel("Viewing".to_string())));
        }
    }
}
