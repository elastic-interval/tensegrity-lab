use crate::crucible_context::CrucibleContext;
use crate::fabric::fabric_sampler::{FabricAnalysis, FabricSampler};
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;
use crate::{Radio, StateChange, TesterAction};

/// Refresh the strut-force readout every this many frames (~0.5 s at 60fps).
const FORCE_READOUT_FRAMES: usize = 30;

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
                if let Some(analysis) = sampler.analyze() {
                    let text = analysis.format();
                    StateChange::ShowMovementAnalysis(Some(text)).send(&self.radio);

                    self.fabric_analysis = Some(analysis);
                    self.showing_analysis = true;

                    // Clear the sampler
                    self.fabric_sampler = None;
                }
            }
        }

        // Live strut-force readout, unless the movement sampler owns the overlay.
        self.iterations_since_stats_update += 1;
        if self.fabric_sampler.is_none()
            && !self.showing_analysis
            && self.iterations_since_stats_update >= FORCE_READOUT_FRAMES
        {
            self.iterations_since_stats_update = 0;
            let stats = self.fabric.push_force_stats();
            let text = format!(
                "Strut force (kN)\nmax {:.1}  median {:.1}  avg {:.1}\nn={}",
                stats.max_kn, stats.median_kn, stats.mean_kn, stats.count
            );
            StateChange::ShowMovementAnalysis(Some(text)).send(&self.radio);
        }

        context.replace_fabric(self.fabric.clone());
        *context.physics = self.physics.clone();
    }

    pub fn action(&mut self, action: TesterAction) {
        use TesterAction::*;
        match action {
            SetTweakParameter(parameter) => {
                self.physics.accept_tweak(parameter);
            }
            DumpPhysics => {
                println!("{:?}", self.physics);
            }
            Reorient => {
                use glam::Mat4;
                // Quarter-turn onto its side, then drop from a little height so it
                // falls and topples onto a stable rest (e.g. two legs) instead of
                // being set down perched on one — a frictionless floor gives a
                // balanced, at-rest structure no sideways nudge to slide off.
                self.fabric
                    .apply_matrix4(Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2));
                let (min_y, max_y) = self.fabric.altitude_range();
                let drop_height = 0.25 * (max_y - min_y);
                let translation = self.fabric.centralize_translation(Some(drop_height));
                self.fabric.apply_translation(translation);
                self.fabric.zero_velocities();
            }
            ToggleMovementSampler => {
                if self.showing_analysis {
                    self.showing_analysis = false;
                    self.fabric_analysis = None;
                    StateChange::ShowMovementAnalysis(None).send(&self.radio);
                } else if self.fabric_sampler.is_some() {
                    self.fabric_sampler = None;
                    StateChange::ShowMovementAnalysis(None).send(&self.radio);
                } else {
                    let sampler = FabricSampler::new(self.fabric.joints.len());
                    let progress = sampler.format_progress();
                    StateChange::ShowMovementAnalysis(Some(progress)).send(&self.radio);
                    self.fabric_sampler = Some(sampler);
                }
            }
        }
    }
}
