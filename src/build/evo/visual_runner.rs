use crate::build::evo::engine::{EngineState, EvolutionEngine};
use crate::build::evo::fitness::StabilityFitness;
use crate::build::evo::genomes::GrowthGenome;
use crate::build::evo::populations::SimplePopulation;
use crate::build::evo::traits::{CompositeFitness, ExpressionContext, Genome, TrialConfig};
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::Fabric;
use crate::units::Seconds;
use crate::{LabEvent, StateChange};

pub struct VisualEvolutionRunner {
    engine: EvolutionEngine<GrowthGenome>,
    pub fabric: Fabric,
    frame_counter: usize,
}

impl VisualEvolutionRunner {
    pub fn new(seed: u64) -> Self {
        let mut fitness = CompositeFitness::new();
        fitness.add_dimension(Box::new(StabilityFitness::new()));

        let population: Box<dyn crate::build::evo::traits::PopulationStrategy<GrowthGenome>> =
            Box::new(SimplePopulation::new(20, 4, seed));

        let trial_config = TrialConfig::new(CONSTRUCTION, Seconds(1.0));

        let mut engine = EvolutionEngine::new(population, fitness, trial_config);

        let seed_genome = GrowthGenome::new(seed);
        engine.initialize(vec![seed_genome.clone()]);

        let context = ExpressionContext::new(CONSTRUCTION);
        let fabric = seed_genome.express(&context);

        Self {
            engine,
            fabric,
            frame_counter: 0,
        }
    }

    pub fn adopt_physics(&self, context: &mut CrucibleContext) {
        *context.physics = CONSTRUCTION;
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        self.frame_counter += 1;

        let changed = self.engine.visual_iterate(1000);

        if let Some(fabric) = self.engine.current_fabric() {
            self.fabric = fabric.clone();
            *context.fabric = self.fabric.clone();
        }

        if changed || self.frame_counter % 30 == 0 {
            self.send_status_label(context);
        }
    }

    fn send_status_label(&self, context: &CrucibleContext) {
        let label = self.engine.status_description();
        let _ = context
            .radio
            .send_event(LabEvent::UpdateState(StateChange::SetStageLabel(label)));
    }

    pub fn should_terminate(&self) -> bool {
        self.engine.state() == EngineState::Terminated
    }
}
