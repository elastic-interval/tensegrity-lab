use std::fmt::{Display, Formatter};

pub mod application;
pub mod build;
pub mod camera;
#[cfg(not(target_arch = "wasm32"))]
pub mod cord_machine;
pub mod crucible;
pub mod fabric;
pub mod keyboard;
pub mod messages;
pub mod scene;
pub mod test;
pub mod wgpu;


#[derive(Debug, Clone, Copy)]
pub struct Age(f64);

impl Display for Age {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}s", self.0/1_000_000.0)
    }
}

impl Default for Age {
    fn default() -> Self {
        Self(0.0)
    }
}

const TICK_MICROSECONDS: f64 = 200.0;

impl Age {
    pub fn tick(&mut self) -> f32 {
        self.0 += TICK_MICROSECONDS;
        TICK_MICROSECONDS as f32
    }

    pub fn advanced(&self, ticks: usize) -> Self {
        Self(self.0 + TICK_MICROSECONDS * (ticks as f64))
    }

    pub fn brick_baked(&self) -> bool {
        self.0 > 20000.0 * TICK_MICROSECONDS
    }

    pub fn within(&self, limit: &Self) -> bool {
        self.0 < limit.0
    }
}
