use crate::units::{Meters, Seconds};

/// Evolvable parameters governing tensegrity growth behavior.
///
/// Each Push interval acts as an autonomous agent with behavior determined by this DNA.
/// The DNA remains fixed throughout the Push's lifetime, like genetic code.
///
/// All timing parameters use fabric time (Seconds), not iteration counts,
/// ensuring frame-rate independent behavior.
#[derive(Clone, Debug)]
pub struct GrowthDna {
    // Geometry
    /// Length of spawned Push intervals
    pub push_length: Meters,
    /// Ratio for short pull (default 1/3)
    pub pull_short_ratio: f32,
    /// Ratio for long pull (default 2/3)
    pub pull_long_ratio: f32,

    // Sensing
    /// How far endpoints can detect other endpoints (in fabric units)
    pub sensing_radius: f32,
    /// Time between sensing attempts (fabric time)
    pub sensing_interval: Seconds,

    // Connection
    /// Probability of connecting when a valid target is sensed [0-1]
    pub connection_eagerness: f32,
    /// Minimum number of secondary connections needed to become Anchored
    pub min_anchor_connections: usize,

    // Spawning
    /// Probability of spawning per second of fabric time when eligible [0-1]
    pub spawn_rate: f32,
    /// Minimum time after anchoring before spawning is allowed (fabric time)
    pub spawn_delay: Seconds,

    // Survival
    /// Maximum time in Pivoting state before dying (fabric time)
    pub pivot_timeout: Seconds,

    // Animation
    /// Duration for new pull connections to reach ideal length (fabric time)
    pub connection_duration: Seconds,
}

impl Default for GrowthDna {
    fn default() -> Self {
        Self {
            // Geometry - based on tensegrity sphere algorithm
            push_length: Meters(1.0),
            pull_short_ratio: 1.0 / 3.0,
            pull_long_ratio: 2.0 / 3.0,

            // Sensing - scan nearby region
            sensing_radius: 2.0,              // 2 fabric units
            sensing_interval: Seconds(0.05),  // Sense every 50ms of fabric time

            // Connection - fairly eager to connect
            connection_eagerness: 0.8,
            min_anchor_connections: 2,        // Need 2 secondary connections to anchor

            // Spawning - moderate spawn rate
            spawn_rate: 2.0,                  // ~2 spawns per second when eligible
            spawn_delay: Seconds(0.5),        // Wait 0.5s fabric time after anchoring

            // Survival - generous timeout
            pivot_timeout: Seconds(5.0),      // 5 seconds fabric time to find connections

            // Animation - smooth 0.5 second transition
            connection_duration: Seconds(0.5),
        }
    }
}

impl GrowthDna {
    /// Calculate short pull ideal length from push length
    pub fn short_pull_length(&self) -> Meters {
        self.push_length * self.pull_short_ratio
    }

    /// Calculate long pull ideal length from push length
    pub fn long_pull_length(&self) -> Meters {
        self.push_length * self.pull_long_ratio
    }
}
