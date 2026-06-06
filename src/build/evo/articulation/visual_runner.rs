use crate::build::dsl::brick_dsl::BrickName;
use crate::build::evo::articulation::fitness::{breakdown, score};
use crate::build::evo::articulation::genome::ArticulationGenome;
use crate::build::evo::articulation::trial::{ArticulationConfig, ArticulationTrial};
use crate::build::evo::simple_population::SimplePopulation;
use crate::build::evo::traits::{ExpressionContext, Genome, PopulationStrategy};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::{BAKING, CONSTRUCTION};
use crate::fabric::Fabric;
use crate::{LabEvent, StateChange};

const POPULATION_SIZE: usize = 16;
const ELITE_COUNT: usize = 4;
const ITERS_PER_FRAME: usize = 2000;

pub struct ArticulationVisualRunner {
    population: SimplePopulation<ArticulationGenome>,
    config: ArticulationConfig,
    current: Option<(ArticulationTrial, ArticulationGenome)>,
    best_score: f32,
    generation: usize,
    pub fabric: Fabric,
    frame_counter: usize,
}

impl ArticulationVisualRunner {
    pub fn new(seed: u64) -> Self {
        let mut population = SimplePopulation::new(POPULATION_SIZE, ELITE_COUNT, seed);
        // Seed from several distinct viable bricks so the run explores more
        // than one topology. (Torque is omitted — it isn't a stable
        // standalone articulator; see the diagnose_seeds test.)
        let seeds: Vec<ArticulationGenome> = [
            BrickName::OmniSymmetrical,
            BrickName::OmniTetrahedral,
            BrickName::SingleTwistLeft,
        ]
        .into_iter()
        .map(ArticulationGenome::from_brick)
        .collect();
        let first = seeds[0].clone();
        population.initialize(seeds);

        let context = ExpressionContext::new(CONSTRUCTION);
        let fabric = first.express(&context);

        Self {
            population,
            config: ArticulationConfig::new(BAKING),
            current: None,
            best_score: 0.0,
            generation: 0,
            fabric,
            frame_counter: 0,
        }
    }

    pub fn adopt_physics(&self, context: &mut CrucibleContext) {
        *context.physics = CONSTRUCTION;
    }

    fn ensure_trial(&mut self) {
        if self.current.is_some() {
            return;
        }
        match self.population.next_for_trial() {
            Some(genome) => {
                let trial = ArticulationTrial::new(genome.structure(), &self.config);
                self.current = Some((trial, genome));
            }
            None => {
                self.population.advance_generation();
                self.generation += 1;
            }
        }
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        self.frame_counter += 1;
        self.ensure_trial();

        let mut finished: Option<(ArticulationGenome, f32)> = None;
        if let Some((trial, genome)) = &mut self.current {
            let running = trial.step_batch(ITERS_PER_FRAME);
            self.fabric = trial.fabric.clone();
            if !running {
                finished = Some((genome.clone(), score(&trial.outcome())));
            }
        }
        if let Some((genome, fitness)) = finished {
            self.best_score = self.best_score.max(fitness);
            self.population.record_result(genome, fitness);
            self.current = None;
        }

        *context.fabric = self.fabric.clone();

        if self.frame_counter % 15 == 0 {
            let label = self.status_label();
            let _ = context
                .radio
                .send_event(LabEvent::UpdateState(StateChange::SetStageLabel(label)));
        }
    }

    fn status_label(&self) -> String {
        let progress = self
            .current
            .as_ref()
            .map(|(t, _)| t.progress() * 100.0)
            .unwrap_or(0.0);
        let breakdown = self
            .current
            .as_ref()
            .map(|(t, _)| breakdown(&t.outcome()));
        match breakdown {
            Some(b) => format!(
                "Gen {} • {:.0}% • gate {} gain {:.2} rev {:.2} • best {:.3}",
                self.generation,
                progress,
                if b.gate { "ok" } else { "—" },
                b.gain,
                b.reversibility,
                self.best_score,
            ),
            None => format!("Gen {} • best {:.3}", self.generation, self.best_score),
        }
    }

    pub fn should_terminate(&self) -> bool {
        false
    }
}
