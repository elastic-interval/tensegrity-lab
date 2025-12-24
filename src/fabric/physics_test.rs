use crate::crucible_context::CrucibleContext;
use crate::fabric::fabric_sampler::{FabricAnalysis, FabricSampler};
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{Radio, StateChange, TesterAction};

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
    iterations_since_stats_update: usize,
    fabric_sampler: Option<FabricSampler>,
    fabric_analysis: Option<FabricAnalysis>,
    showing_analysis: bool,
}

impl PhysicsTester {
    pub fn new(fabric: Fabric, physics: Physics, radio: Radio) -> Self {
        Self {
            fabric,
            physics,
            radio,
            iterations_since_stats_update: 0,
            fabric_sampler: None,
            fabric_analysis: None,
            showing_analysis: false,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        self.fabric = context.fabric.clone();

        // Use our own physics (which has user modifications) instead of context.physics
        for _ in 0..iterations_per_frame {
            self.fabric.iterate(&self.physics);
        }

        // Record sample if sampler is active
        if let Some(sampler) = &mut self.fabric_sampler {
            let prev_count = sampler.sample_count();
            sampler.record_sample(&self.fabric);

            // Show progress update when a new sample is recorded
            if sampler.sample_count() > prev_count {
                let progress = sampler.format_progress();
                StateChange::ShowMovementAnalysis(Some(progress)).send(&self.radio);
            }

            // Check if sampling is complete
            if sampler.is_complete() {
                // Analyze and show results
                if let Some(analysis) = sampler.analyze(&self.fabric, &self.physics) {
                    let text = analysis.format();
                    StateChange::ShowMovementAnalysis(Some(text)).send(&self.radio);

                    self.fabric_analysis = Some(analysis);
                    self.showing_analysis = true;

                    // Clear the sampler
                    self.fabric_sampler = None;
                }
            }
        }

        // Track iterations for stats updates (count frames, not iterations)
        self.iterations_since_stats_update += 1;
        context.replace_fabric(self.fabric.clone());
        *context.physics = self.physics.clone();
    }

    pub fn action(&mut self, action: TesterAction) {
        use TesterAction::*;
        match action {
            SetTweakParameter(parameter) => {
                self.physics.accept_tweak(parameter);
                // Mass/rigidity changes take effect on the next iterate() call
            }
            DumpPhysics => {
                println!("{:?}", self.physics);
            }
            ToggleMovementSampler => {
                if self.showing_analysis {
                    // Hide analysis
                    self.showing_analysis = false;
                    self.fabric_analysis = None;
                    StateChange::ShowMovementAnalysis(None).send(&self.radio);
                } else if self.fabric_sampler.is_some() {
                    // Cancel active sampling
                    self.fabric_sampler = None;
                    StateChange::ShowMovementAnalysis(None).send(&self.radio);
                } else {
                    // Start new sampling
                    let sampler = FabricSampler::new(self.fabric.joints.len());
                    let progress = sampler.format_progress();
                    StateChange::ShowMovementAnalysis(Some(progress)).send(&self.radio);
                    self.fabric_sampler = Some(sampler);
                }
            }
        }
    }
}
