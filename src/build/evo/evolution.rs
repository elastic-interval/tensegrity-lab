use crate::build::evo::evolving_push::EvolvingPush;
use crate::fabric::physics::presets::PROTOTYPE_FORMATION;
use crate::fabric::{Fabric, UniqueId};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

const DELAY: usize = 1000;

#[derive()]
pub struct Evolution {
    random: ChaCha8Rng,
    countdown: usize,
    intervals: HashMap<UniqueId, EvolvingPush>,
}

impl Evolution {
    pub fn new(seed: u64) -> Self {
        Self {
            random: ChaCha8Rng::seed_from_u64(seed),
            countdown: DELAY,
            intervals: Default::default(),
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) {
        if self.countdown > 0 {
            fabric.iterate(&PROTOTYPE_FORMATION);
            self.countdown -= 1;
        } else {
            self.countdown = DELAY;
            self.step(fabric);
        }
    }

    fn step(&mut self, _fabric: &mut Fabric) {
    }
}
