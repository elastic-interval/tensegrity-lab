use crate::build::evo::evolving_push::EvolvingPush;
use crate::fabric::interval::End;
use crate::fabric::physics::presets::LIQUID;
use crate::fabric::Fabric;
use cgmath::{InnerSpace, Vector3};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

const DELAY: usize = 300;

#[derive()]
pub struct Evolution {
    random: ChaCha8Rng,
    countdown: usize,
    evolving_pushes: Vec<EvolvingPush>,
}

impl Evolution {
    pub fn new(seed: u64) -> Self {
        Self {
            random: ChaCha8Rng::seed_from_u64(seed),
            countdown: DELAY,
            evolving_pushes: Default::default(),
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        if self.countdown > 0 {
            fabric.iterate(&LIQUID);
            self.countdown -= 1;
        } else {
            self.countdown = DELAY;
            self.step(fabric);
        }
    }

    fn step(&mut self, fabric: &mut Fabric) {
        if self.evolving_pushes.is_empty() {
            self.evolving_pushes.push(EvolvingPush::first_push(fabric));
        } else if self.evolving_pushes.len() < 5 || self.random_bool() {
            self.sprout(fabric);
        } else {
            self.join(fabric);
        }
    }

    fn sprout(&mut self, fabric: &mut Fabric) {
        let end = if self.random_bool() {
            End::Alpha
        } else {
            End::Omega
        };
        let choice = self.random_push();
        let project = self.random_unit();
        let evolving_push = self.evolving_pushes.get_mut(choice).unwrap();
        let snapshot = fabric.interval_snapshot(evolving_push.interval_id);
        let next = evolving_push.end_push(fabric, snapshot, end, project);
        self.evolving_pushes.push(next);
    }

    fn join(&mut self, fabric: &mut Fabric) {
        let index_a = self.random_push();
        let mut index_b = self.random_push();
        while index_b == index_a {
            index_b = self.random_push()
        }
        let push_a = (
            index_a,
            fabric.interval_snapshot(self.evolving_pushes[index_a].interval_id),
        );
        let push_b = (
            index_b,
            fabric.interval_snapshot(self.evolving_pushes[index_b].interval_id),
        );
        EvolvingPush::join_pushes(fabric, &mut self.evolving_pushes, push_a, push_b);
    }

    fn random_bool(&mut self) -> bool {
        self.random.next_u32() % 2 == 1
    }

    fn random_push(&mut self) -> usize {
        self.random.gen_range(0..self.evolving_pushes.len())
    }

    fn random_unit(&mut self) -> Vector3<f32> {
        Vector3::new(self.random_f32(), self.random_f32(), self.random_f32()).normalize()
    }

    fn random_f32(&mut self) -> f32 {
        self.random.gen_range(-1.0..1.0)
    }
}
