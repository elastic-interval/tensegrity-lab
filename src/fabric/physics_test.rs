use crate::crucible_context::CrucibleContext;
use crate::fabric::Fabric;
use crate::fabric::movement_sampler::{MovementSampler, MovementAnalysis};
use crate::fabric::physics::Physics;
use crate::units::{Percent, Seconds};
use crate::{PhysicsFeature, Radio, StateChange, TesterAction, ITERATIONS_PER_FRAME};

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
    iterations_since_stats_update: usize,
    movement_sampler: Option<MovementSampler>,
    movement_analysis: Option<MovementAnalysis>,
    showing_analysis: bool,
    fabric_was_frozen: bool,
}

impl PhysicsTester {
    pub fn new(fabric: Fabric, physics: Physics, radio: Radio) -> Self {
        Self {
            fabric,
            physics,
            radio,
            iterations_since_stats_update: 0,
            movement_sampler: None,
            movement_analysis: None,
            showing_analysis: false,
            fabric_was_frozen: false,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        self.fabric = context.fabric.clone();

        // If showing analysis, fabric should be frozen
        if self.showing_analysis {
            // Don't run physics while showing analysis
            context.replace_fabric(self.fabric.clone());
            *context.physics = self.physics.clone();
            return;
        }

        // Use our own physics (which has user modifications) instead of context.physics
        for _ in 0..ITERATIONS_PER_FRAME {
            self.fabric.iterate(&self.physics);
        }

        // Record sample if sampler is active
        if let Some(sampler) = &mut self.movement_sampler {
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
                if let Some(analysis) = sampler.analyze(self.fabric.scale) {
                    let text = analysis.format();
                    StateChange::ShowMovementAnalysis(Some(text)).send(&self.radio);

                    self.movement_analysis = Some(analysis);
                    self.showing_analysis = true;
                    self.fabric_was_frozen = self.fabric.frozen;
                    self.fabric.frozen = true;

                    // Clear the sampler
                    self.movement_sampler = None;
                }
            }
        }

        // Track iterations for stats updates (count frames, not iterations)
        self.iterations_since_stats_update += 1;

        // Update stats approximately every second (60 frames)
        if self.iterations_since_stats_update >= 60 {
            self.iterations_since_stats_update = 0;

            // Recalculate and broadcast updated stats
            let stats = self.fabric.stats_with_dynamics(&self.physics);
            StateChange::SetFabricStats(Some(stats)).send(&self.radio);
        }

        // Update the context's fabric and physics with our changes
        context.replace_fabric(self.fabric.clone());
        *context.physics = self.physics.clone();
    }

    pub fn action(&mut self, action: TesterAction) {
        use TesterAction::*;
        match action {
            SetPhysicalParameter(parameter) => {
                self.physics.accept(parameter);
                match parameter.feature {
                    PhysicsFeature::Pretenst => {
                        self.fabric.set_pretenst(Percent(parameter.value), Seconds(10.0));
                    }
                }
            }
            SetTweakParameter(parameter) => {
                self.physics.accept_tweak(parameter);
                // Mass/rigidity changes take effect on the next iterate() call
            }
            DumpPhysics => {
                println!("{:?}", self.physics);
            }
            ToggleMovementSampler => {
                if self.showing_analysis {
                    // Hide analysis and restore fabric state
                    self.showing_analysis = false;
                    self.movement_analysis = None;
                    self.fabric.frozen = self.fabric_was_frozen;
                    StateChange::ShowMovementAnalysis(None).send(&self.radio);
                } else if self.movement_sampler.is_some() {
                    // Cancel active sampling
                    self.movement_sampler = None;
                    StateChange::ShowMovementAnalysis(None).send(&self.radio);
                } else {
                    // Start new sampling
                    let sampler = MovementSampler::new(self.fabric.joints.len());
                    let progress = sampler.format_progress();
                    StateChange::ShowMovementAnalysis(Some(progress)).send(&self.radio);
                    self.movement_sampler = Some(sampler);
                }
            }
        }
    }
}
