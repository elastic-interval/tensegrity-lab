use crate::build::evo::evolving_push::EvolvingPush;
use crate::crucible_context::CrucibleContext;
use crate::fabric::physics::presets::CONSTRUCTION;
use crate::fabric::Fabric;
use crate::fabric::IntervalEnd;
use cgmath::{InnerSpace, Vector3};
use rand::Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

const DELAY: usize = 300;

#[derive()]
pub struct Evolution {
    pub fabric: Fabric,
    random: ChaCha8Rng,
    countdown: usize,
    evolving_pushes: Vec<EvolvingPush>,
}

impl Evolution {
    pub fn new(seed: u64) -> Self {
        Self {
            fabric: Fabric::new(seed.to_string()),
            random: ChaCha8Rng::seed_from_u64(seed),
            countdown: DELAY,
            evolving_pushes: Default::default(),
        }
    }

    pub fn adopt_physica(&self, context: &mut CrucibleContext) {
        *context.physics = CONSTRUCTION;
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext) {
        if self.countdown > 0 {
            // Use the physics-defined number of iterations
            for _ in 0..1000 {
                // Nominal value, outer loop adjusts dynamically
                context.fabric.iterate(context.physics);
            }
            self.countdown -= 1;
        } else {
            // Update our fabric from the context for the step operation
            self.fabric = context.fabric.clone();

            self.countdown = DELAY;
            self.step();

            // Update the context's fabric with our changes only when needed
            *context.fabric = self.fabric.clone();
        }
    }

    fn step(&mut self) {
        if self.evolving_pushes.is_empty() {
            self.evolving_pushes
                .push(EvolvingPush::first_push(&mut self.fabric));
        } else if self.evolving_pushes.len() < 5 || self.random_bool() {
            self.sprout();
        } else {
            self.join();
        }
    }

    fn sprout(&mut self) {
        let end = if self.random_bool() {
            IntervalEnd::Alpha
        } else {
            IntervalEnd::Omega
        };
        let choice = self.random_push();
        let project = self.random_unit();
        let evolving_push = self.evolving_pushes.get_mut(choice).unwrap();
        let snapshot = self.fabric.interval_snapshot(evolving_push.interval_key);
        let next = evolving_push.end_push(&mut self.fabric, snapshot, end, project);
        self.evolving_pushes.push(next);
    }

    fn join(&mut self) {
        let index_a = self.random_push();
        let mut index_b = self.random_push();
        while index_b == index_a {
            index_b = self.random_push()
        }
        let push_a = (
            index_a,
            self.fabric
                .interval_snapshot(self.evolving_pushes[index_a].interval_key),
        );
        let push_b = (
            index_b,
            self.fabric
                .interval_snapshot(self.evolving_pushes[index_b].interval_key),
        );
        EvolvingPush::join_pushes(&mut self.fabric, &mut self.evolving_pushes, push_a, push_b);
    }

    fn random_bool(&mut self) -> bool {
        self.random.next_u32() % 2 == 1
    }

    fn random_push(&mut self) -> usize {
        self.random.random_range(0..self.evolving_pushes.len())
    }

    fn random_unit(&mut self) -> Vector3<f32> {
        Vector3::new(self.random_f32(), self.random_f32(), self.random_f32()).normalize()
    }

    fn random_f32(&mut self) -> f32 {
        self.random.random_range(-1.0..1.0)
    }
}
