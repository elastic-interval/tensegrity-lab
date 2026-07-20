use crate::build::dsl::fabric_library::FabricName;
use std::fmt::{Display, Formatter};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub mod animation_export;
pub mod application;
pub mod build;
pub mod caliper;
pub mod camera;
pub mod connector;
pub mod control;
pub mod crucible;
pub mod crucible_context;
pub mod events;
pub mod fabric;
pub mod keyboard;
#[cfg(not(target_arch = "wasm32"))]
pub mod physics_gpu;
pub mod pointer;
pub mod scene;
pub mod units;
pub mod wgpu;

#[cfg(test)]
mod fabric_smoke_test;
#[cfg(test)]
mod open_claw_symmetry;
mod open_claw_test;
mod propeller_blender_export;

// Re-export every public name from `control` and `events` at the crate root
// so existing import paths (`use crate::ControlState;` etc.) keep working.
pub use control::*;
pub use events::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Age(Duration);

impl Display for Age {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let total_secs = self.0.as_secs_f64();

        if total_secs < 1.0 {
            // Less than a second: show hundredths
            write!(f, "{:.2}s", total_secs)
        } else if total_secs < 60.0 {
            // Less than a minute: show as whole seconds
            write!(f, "{}s", total_secs as u64)
        } else {
            // 60 seconds or more: show as minutes:seconds
            let minutes = (total_secs / 60.0).floor() as u64;
            let seconds = (total_secs % 60.0) as u64;
            write!(f, "{}:{:02}", minutes, seconds)
        }
    }
}

impl Default for Age {
    fn default() -> Self {
        Self(Duration::ZERO)
    }
}

/// Duration of each physics iteration tick (50 microseconds)
const TICK_DURATION: Duration = Duration::from_micros(50);
const TICK_SECS: f32 = 50.0 / 1_000_000.0;

impl Age {
    /// Duration of a single physics iteration in seconds
    pub fn iteration_duration() -> f32 {
        TICK_SECS
    }

    /// Number of iterations per second (inverse of iteration_duration)
    pub fn iterations_per_second() -> f32 {
        1.0 / TICK_SECS
    }

    pub fn tick(&mut self) -> Duration {
        self.0 += TICK_DURATION;
        TICK_DURATION
    }

    pub fn advanced(&self, ticks: usize) -> Self {
        Self(self.0 + TICK_DURATION * ticks as u32)
    }

    pub fn within(&self, limit: &Self) -> bool {
        self.0 < limit.0
    }

    pub fn as_duration(&self) -> Duration {
        self.0
    }

    pub fn elapsed_since(&self, earlier: Age) -> units::Seconds {
        let elapsed = self.0.saturating_sub(earlier.0);
        units::Seconds(elapsed.as_secs_f32())
    }
}

#[derive(Debug, Clone)]
pub enum RunStyle {
    Unknown,
    Fabric {
        fabric_name: FabricName,
        /// Record animation for this duration from start
        record: Option<units::Seconds>,
        /// FPS for animation export (default 100)
        export_fps: f64,
    },
    /// Algorithmic tensegrity sphere (geodesic)
    Sphere {
        frequency: usize,
        radius: f32,
    },
    /// Algorithmic Möbius strip
    Mobius {
        segments: usize,
    },
    /// Algorithmic Klein bottle tensegrity (width=even, height=odd)
    Klein {
        width: usize,
        height: usize,
        shift: usize,
    },
    BakeBricks,
    Evolution(u64),
}
