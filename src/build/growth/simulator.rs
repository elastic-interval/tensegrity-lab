use std::collections::HashMap;

use glam::Vec3;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::Unit;
use crate::Age;

use super::dna::GrowthDna;
use super::push::{GrowingPush, PushState};

/// Seed altitude in meters (how high above ground the seed Push is placed)
const SEED_ALTITUDE: f32 = 0.5;

/// Orchestrates tensegrity growth from a single seed Push.
///
/// The simulator manages the lifecycle of growing Push intervals:
/// 1. Seed Push is created at origin
/// 2. Anchored Pushes can spawn new Pivoting Pushes from their endpoints
/// 3. Pivoting Pushes seek connections to become Anchored
/// 4. Pushes that fail to anchor within timeout die and are removed
///
/// All timing is based on fabric age (time), not iteration counts.
pub struct GrowthSimulator {
    /// The DNA governing growth behavior
    dna: GrowthDna,
    /// Random number generator for probabilistic decisions
    rng: ChaCha8Rng,
    /// All growing Pushes tracked by their interval key
    pushes: HashMap<IntervalKey, GrowingPush>,
    /// Map from JointKey to the Push that owns it (for connection lookup)
    endpoint_owners: HashMap<JointKey, IntervalKey>,

    // Statistics
    pub total_spawned: usize,
    pub total_died: usize,
    pub total_anchored: usize,
}

impl GrowthSimulator {
    /// Create a new GrowthSimulator with a seed Push at the origin.
    pub fn new(dna: GrowthDna, seed: u64, context: &mut CrucibleContext) -> Self {
        let mut simulator = Self {
            dna: dna.clone(),
            rng: ChaCha8Rng::seed_from_u64(seed),
            pushes: HashMap::new(),
            endpoint_owners: HashMap::new(),
            total_spawned: 0,
            total_died: 0,
            total_anchored: 0,
        };

        // Create the seed Push at origin
        simulator.create_seed_push(context);

        simulator
    }

    /// Create the initial seed Push centered at origin.
    fn create_seed_push(&mut self, context: &mut CrucibleContext) {
        let half_length = self.dna.push_length.f32() / 2.0;

        // Create joints at origin, horizontal, at seed altitude
        let alpha_pos = Vec3::new(-half_length, SEED_ALTITUDE, 0.0);
        let omega_pos = Vec3::new(half_length, SEED_ALTITUDE, 0.0);

        let alpha_key = context.fabric.create_joint(alpha_pos);
        let omega_key = context.fabric.create_joint(omega_pos);

        // Create the Push interval at slack length (no immediate strain)
        let interval_key = context.fabric.create_slack_interval(
            alpha_key,
            omega_key,
            Role::Pushing,
        );

        // Track as anchored (seed starts ready to spawn)
        let current_age = context.fabric.age;
        let push = GrowingPush::new_anchored(interval_key, alpha_key, omega_key, current_age);
        self.endpoint_owners.insert(alpha_key, interval_key);
        self.endpoint_owners.insert(omega_key, interval_key);
        self.pushes.insert(interval_key, push);
        self.total_spawned = 1;
        self.total_anchored = 1;
    }

    /// Main iteration loop - called once per frame by Crucible.
    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        // Get the time delta per iteration for probability calculations
        let dt = Age::iteration_duration();

        for _ in 0..iterations_per_frame {
            let current_age = context.fabric.age;

            // 1. Check for pivot timeouts (based on fabric age)
            self.check_pivot_timeouts(current_age);

            // 2. Remove dead Pushes and their connections
            self.remove_dead_pushes(context);

            // 3. Pivoting Pushes sense and try to connect (based on fabric age)
            self.seek_connections(context, current_age);

            // 4. Check if Pivoting Pushes can become Anchored
            self.check_anchoring(current_age);

            // 5. Anchored Pushes may spawn children (probability per unit time)
            self.try_spawning(context, current_age, dt);

            // 6. Physics step
            context.fabric.iterate(context.physics);
        }
    }

    /// Check if any pivoting Pushes have exceeded their timeout.
    fn check_pivot_timeouts(&mut self, current_age: crate::Age) {
        let timeout = self.dna.pivot_timeout;

        for push in self.pushes.values_mut() {
            if let PushState::Pivoting { start_age, .. } = &push.state {
                let elapsed = current_age.elapsed_since(*start_age);
                if elapsed.0 >= timeout.0 {
                    push.state = PushState::Dying;
                }
            }
        }
    }

    /// Remove dead Pushes and all their associated intervals.
    fn remove_dead_pushes(&mut self, context: &mut CrucibleContext) {
        let dying_keys: Vec<IntervalKey> = self
            .pushes
            .iter()
            .filter(|(_, push)| push.is_dying())
            .map(|(key, _)| *key)
            .collect();

        for key in dying_keys {
            if let Some(push) = self.pushes.remove(&key) {
                // Remove from endpoint owners
                self.endpoint_owners.remove(&push.alpha_key);
                self.endpoint_owners.remove(&push.omega_key);

                // Remove all intervals (push + parent pulls + anchor pulls)
                for interval_key in push.all_interval_keys() {
                    context.fabric.remove_interval(interval_key);
                }

                // Remove joints
                context.fabric.remove_joint(push.alpha_key);
                context.fabric.remove_joint(push.omega_key);

                self.total_died += 1;
            }
        }
    }

    /// Pivoting Pushes sense nearby endpoints and try to connect.
    fn seek_connections(&mut self, context: &mut CrucibleContext, current_age: crate::Age) {
        let sensing_interval = self.dna.sensing_interval;

        // Collect Pivoting pushes that are ready to sense (based on time since last sense)
        let ready_to_sense: Vec<IntervalKey> = self
            .pushes
            .iter()
            .filter(|(_, push)| {
                if let PushState::Pivoting { last_sense_age, .. } = &push.state {
                    let elapsed = current_age.elapsed_since(*last_sense_age);
                    elapsed.0 >= sensing_interval.0
                } else {
                    false
                }
            })
            .map(|(key, _)| *key)
            .collect();

        for push_key in ready_to_sense {
            // Get the push's endpoints
            let (alpha_key, omega_key, parent_endpoint) = {
                let push = &self.pushes[&push_key];
                let parent = match &push.state {
                    PushState::Pivoting { parent_endpoint, .. } => *parent_endpoint,
                    _ => continue,
                };
                (push.alpha_key, push.omega_key, parent)
            };

            // Determine which endpoint is the "free" one (not connected to parent)
            let free_endpoint = if alpha_key == parent_endpoint {
                omega_key
            } else {
                alpha_key
            };
            let free_pos = context.fabric.location(free_endpoint);

            // Find nearby endpoints to connect to
            let candidates = self.find_connection_candidates(
                context.fabric,
                free_endpoint,
                free_pos,
                push_key,
            );

            // Try to connect to a candidate
            for candidate_endpoint in candidates {
                if self.rng.random_range(0.0..1.0) < self.dna.connection_eagerness {
                    // Create the connection pulls
                    self.create_connection(context, push_key, free_endpoint, candidate_endpoint);
                    break; // One connection per sensing attempt
                }
            }

            // Update last sense time
            if let Some(push) = self.pushes.get_mut(&push_key) {
                if let PushState::Pivoting { last_sense_age, .. } = &mut push.state {
                    *last_sense_age = current_age;
                }
            }
        }
    }

    /// Find endpoints within sensing radius that could be connected to.
    fn find_connection_candidates(
        &self,
        fabric: &Fabric,
        from_endpoint: JointKey,
        from_pos: Vec3,
        exclude_push: IntervalKey,
    ) -> Vec<JointKey> {
        let mut candidates = Vec::new();
        let radius_sq = self.dna.sensing_radius * self.dna.sensing_radius;

        for (&endpoint, &owner_push) in &self.endpoint_owners {
            // Skip self
            if owner_push == exclude_push {
                continue;
            }
            // Skip if already the from_endpoint
            if endpoint == from_endpoint {
                continue;
            }

            // Check distance
            let pos = fabric.location(endpoint);
            let dist_sq = (pos - from_pos).length_squared();
            if dist_sq <= radius_sq {
                candidates.push(endpoint);
            }
        }

        candidates
    }

    /// Create a pull connection from a Pivoting Push's free endpoint to another endpoint.
    fn create_connection(
        &mut self,
        context: &mut CrucibleContext,
        push_key: IntervalKey,
        from_endpoint: JointKey,
        to_endpoint: JointKey,
    ) {
        // Create an approaching pull interval
        let pull_length = self.dna.short_pull_length(); // Use short ratio for anchor connections
        let pull_key = context.fabric.create_approaching_interval(
            from_endpoint,
            to_endpoint,
            pull_length,
            Role::Pulling,
            self.dna.connection_duration,
        );

        // Record this connection on the Push
        if let Some(push) = self.pushes.get_mut(&push_key) {
            push.add_anchor_pull(pull_key);
        }
    }

    /// Check if Pivoting Pushes have enough connections to become Anchored.
    fn check_anchoring(&mut self, current_age: crate::Age) {
        let min_connections = self.dna.min_anchor_connections;

        for push in self.pushes.values_mut() {
            if push.is_pivoting() && push.anchor_pull_count() >= min_connections {
                push.anchor(current_age);
                self.total_anchored += 1;
            }
        }
    }

    /// Anchored Pushes may spawn new children from their endpoints.
    /// Uses spawn_rate (per second) converted to per-iteration probability.
    fn try_spawning(&mut self, context: &mut CrucibleContext, current_age: crate::Age, dt: f32) {
        // Convert spawn rate (per second) to probability per iteration
        // P(spawn in dt) ≈ rate * dt for small dt
        let spawn_prob_per_iter = self.dna.spawn_rate * dt;

        // Collect spawn candidates: (push_key, endpoint_to_spawn_from)
        let spawn_candidates: Vec<(IntervalKey, JointKey)> = self
            .pushes
            .iter()
            .filter_map(|(key, push)| {
                if !push.is_anchored() {
                    return None;
                }

                // Check spawn probability (per iteration, derived from rate)
                if self.rng.random_range(0.0..1.0) >= spawn_prob_per_iter {
                    return None;
                }

                // Try alpha endpoint
                if push.can_spawn(push.alpha_key, current_age, self.dna.spawn_delay) {
                    return Some((*key, push.alpha_key));
                }
                // Try omega endpoint
                if push.can_spawn(push.omega_key, current_age, self.dna.spawn_delay) {
                    return Some((*key, push.omega_key));
                }

                None
            })
            .collect();

        // Perform spawns
        for (parent_key, parent_endpoint) in spawn_candidates {
            self.spawn_push(context, parent_key, parent_endpoint);
        }
    }

    /// Spawn a new Push from a parent endpoint.
    fn spawn_push(
        &mut self,
        context: &mut CrucibleContext,
        parent_key: IntervalKey,
        parent_endpoint: JointKey,
    ) {
        let parent_pos = context.fabric.location(parent_endpoint);
        let current_age = context.fabric.age;

        // Generate a random direction for the new Push
        let theta: f32 = self.rng.random_range(0.0..std::f32::consts::TAU);
        let phi: f32 = self.rng.random_range(-0.5..0.5); // Slight vertical variation
        let direction = Vec3::new(theta.cos() * phi.cos(), phi.sin(), theta.sin() * phi.cos()).normalize();

        let short_pull_len = self.dna.short_pull_length().f32();
        let long_pull_len = self.dna.long_pull_length().f32();

        // Position new Push so pulls have correct lengths from the start
        // Alpha connects to parent with short pull (1/3), omega with long pull (2/3)
        let alpha_pos = parent_pos + direction * short_pull_len;
        let omega_pos = parent_pos + direction * long_pull_len;

        // Create joints
        let alpha_key = context.fabric.create_joint(alpha_pos);
        let omega_key = context.fabric.create_joint(omega_pos);

        // Create Push interval at slack length (no immediate strain)
        let interval_key = context.fabric.create_slack_interval(
            alpha_key,
            omega_key,
            Role::Pushing,
        );

        // Create the 1/3 and 2/3 pull connections to parent
        let short_pull = context.fabric.create_approaching_interval(
            alpha_key,
            parent_endpoint,
            self.dna.short_pull_length(),
            Role::Pulling,
            self.dna.connection_duration,
        );
        let long_pull = context.fabric.create_approaching_interval(
            omega_key,
            parent_endpoint,
            self.dna.long_pull_length(),
            Role::Pulling,
            self.dna.connection_duration,
        );

        // Create the GrowingPush in Pivoting state
        let push = GrowingPush::new_pivoting(
            interval_key,
            alpha_key,
            omega_key,
            parent_endpoint,
            vec![short_pull, long_pull],
            current_age,
        );

        // Track endpoints
        self.endpoint_owners.insert(alpha_key, interval_key);
        self.endpoint_owners.insert(omega_key, interval_key);
        self.pushes.insert(interval_key, push);

        // Mark parent endpoint as having spawned
        if let Some(parent) = self.pushes.get_mut(&parent_key) {
            parent.mark_spawned(parent_endpoint);
        }

        self.total_spawned += 1;
    }

    /// Get the current count of active Pushes.
    pub fn push_count(&self) -> usize {
        self.pushes.len()
    }

    /// Get count of currently pivoting Pushes.
    pub fn pivoting_count(&self) -> usize {
        self.pushes.values().filter(|p| p.is_pivoting()).count()
    }

    /// Get count of currently anchored Pushes.
    pub fn anchored_count(&self) -> usize {
        self.pushes.values().filter(|p| p.is_anchored()).count()
    }
}
