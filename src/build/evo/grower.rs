use crate::fabric::interval::Role;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Seconds, Unit};
use crate::ITERATION_DURATION;
use glam::Vec3;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Configuration for structure growth.
#[derive(Clone, Debug)]
pub struct GrowthConfig {
    /// Length of push intervals
    pub push_length: Meters,
    /// Maximum pull length as ratio of push length
    pub max_pull_ratio: f32,
    /// Target pull length as ratio of current distance (< 1.0 means pulls will contract)
    pub pull_target_ratio: f32,
    /// Duration for pulls to approach their target length
    pub pull_approach_seconds: Seconds,
    /// Seconds of fabric time to settle initial seed
    pub seed_settle_seconds: f32,
    /// Seconds of fabric time to settle after each mutation
    pub mutation_settle_seconds: f32,
    /// Number of push intervals in initial seed
    pub seed_push_count: usize,
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self {
            push_length: Meters(1.0),
            max_pull_ratio: 3.0, // Allow longer pulls to connect falling structures
            pull_target_ratio: 0.7, // Pulls contract to 70% of initial distance
            pull_approach_seconds: Seconds(1.0), // 1 second to reach target
            seed_settle_seconds: 5.0,
            mutation_settle_seconds: 2.0,
            seed_push_count: 3,
        }
    }
}

/// Creates and mutates tensegrity structures.
pub struct Grower {
    /// RNG for random decisions
    rng: ChaCha8Rng,
    /// Growth configuration
    config: GrowthConfig,
}

impl Grower {
    /// Create a new grower with the given seed.
    pub fn new(seed: u64, config: GrowthConfig) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            config,
        }
    }

    /// Create the initial seed structure with N random pushes connected by pulls.
    /// Returns (fabric, push_count).
    /// Structure starts elevated above the floor so it can fall and settle.
    pub fn create_seed(&mut self) -> (Fabric, usize) {
        let mut fabric = Fabric::new("evo-seed".to_string());
        let push_length = self.config.push_length.f32();
        let mut push_joints: Vec<(JointKey, JointKey)> = Vec::new();

        // Starting elevation - enough room to form before hitting ground
        let base_height = push_length * 1.0;

        // Create N push intervals with random orientations around a central point
        for _ in 0..self.config.seed_push_count {
            let direction = self.random_direction();

            // Spread pushes in a small area at the starting height
            let spread = push_length * 0.3;
            let center = Vec3::new(
                self.rng.random_range(-spread..spread),
                base_height + self.rng.random_range(0.0..push_length * 0.5),
                self.rng.random_range(-spread..spread),
            );

            let half = direction * push_length / 2.0;
            let alpha = fabric.create_joint(center - half);
            let omega = fabric.create_joint(center + half);
            fabric.create_slack_interval(alpha, omega, Role::Pushing);
            push_joints.push((alpha, omega));
        }

        // Connect pushes with pulls
        self.connect_with_pulls(&mut fabric, &push_joints);

        (fabric, self.config.seed_push_count)
    }

    /// Mutate a fabric by adding one push interval near the current height.
    /// The new push's midpoint is placed near the top of the structure to encourage building higher.
    /// Returns the new push count.
    pub fn mutate(&mut self, fabric: &mut Fabric, current_push_count: usize) -> usize {
        let push_length = self.config.push_length.f32();

        // Collect existing joints
        let joints: Vec<JointKey> = fabric.joints.keys().collect();
        if joints.is_empty() {
            return current_push_count;
        }

        // Find the bounds of the structure
        let (min_pos, max_pos) = self.find_bounds(fabric);

        // Place new push with midpoint near the current max height
        // This encourages structures to build higher rather than wider
        let height_variation = push_length * 0.3; // Small variation around max height
        let target_y = max_pos.y + self.rng.random_range(-height_variation..height_variation);
        let new_center = Vec3::new(
            self.rng.random_range(min_pos.x..max_pos.x),
            target_y.max(0.1), // Stay above ground
            self.rng.random_range(min_pos.z..max_pos.z),
        );

        // Random direction for the push
        let direction = self.random_direction();
        let half = direction * push_length / 2.0;
        let new_alpha = fabric.create_joint(new_center - half);
        let new_omega = fabric.create_joint(new_center + half);
        fabric.create_slack_interval(new_alpha, new_omega, Role::Pushing);

        // Connect new push endpoints to nearby existing joints
        // Favor less-connected joints
        self.connect_new_push(fabric, new_alpha, new_omega, &joints);

        current_push_count + 1
    }

    /// Get all pull interval keys from the fabric.
    fn get_pull_keys(&self, fabric: &Fabric) -> Vec<IntervalKey> {
        fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.role == Role::Pulling {
                    Some(key)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Mutate by shortening a random pull interval.
    /// This can pull the structure tighter and potentially increase height.
    pub fn shorten_random_pull(&mut self, fabric: &mut Fabric) -> bool {
        self.adjust_random_pull(fabric, 0.97..0.99)
    }

    /// Mutate by lengthening a random pull interval.
    /// This can loosen the structure and allow it to expand.
    pub fn lengthen_random_pull(&mut self, fabric: &mut Fabric) -> bool {
        self.adjust_random_pull(fabric, 1.01..1.03)
    }

    /// Adjust a random pull interval by a factor in the given range.
    fn adjust_random_pull(&mut self, fabric: &mut Fabric, factor_range: std::ops::Range<f32>) -> bool {
        let pull_keys = self.get_pull_keys(fabric);
        if pull_keys.is_empty() {
            return false;
        }

        let idx = self.rng.random_range(0..pull_keys.len());
        let pull_key = pull_keys[idx];

        if let Some(interval) = fabric.intervals.get(pull_key) {
            let current_ideal = interval.ideal();
            let factor = self.rng.random_range(factor_range);
            let new_target = Meters(current_ideal.f32() * factor);
            fabric.extend_interval(pull_key, new_target, self.config.pull_approach_seconds);
            return true;
        }

        false
    }

    /// Mutate by removing a random pull interval.
    /// This reduces complexity and cost, potentially improving fitness.
    /// Only removes pulls where both joints would still have at least 3 pull connections.
    /// Returns true if a pull was removed.
    pub fn remove_random_pull(&mut self, fabric: &mut Fabric) -> bool {
        use std::collections::HashMap;

        // Count pull connections per joint
        let mut pull_counts: HashMap<JointKey, usize> = HashMap::new();
        for interval in fabric.intervals.values() {
            if interval.role == Role::Pulling {
                *pull_counts.entry(interval.alpha_key).or_insert(0) += 1;
                *pull_counts.entry(interval.omega_key).or_insert(0) += 1;
            }
        }

        // Find pulls that are safe to remove (both joints would still have >= 3 pulls)
        let safe_pull_keys: Vec<IntervalKey> = fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.role == Role::Pulling {
                    let alpha_count = pull_counts.get(&interval.alpha_key).copied().unwrap_or(0);
                    let omega_count = pull_counts.get(&interval.omega_key).copied().unwrap_or(0);
                    // Both joints must have > 3 pulls (so after removal they have >= 3)
                    if alpha_count > 3 && omega_count > 3 {
                        Some(key)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // No safe pulls to remove
        if safe_pull_keys.is_empty() {
            return false;
        }

        // Pick a random safe pull interval and remove it
        let idx = self.rng.random_range(0..safe_pull_keys.len());
        let pull_key = safe_pull_keys[idx];
        fabric.intervals.remove(pull_key);
        true
    }

    /// Mutate by removing a random push interval and its orphaned joints.
    /// This simplifies the structure significantly.
    /// Returns true if a push was removed.
    pub fn remove_random_push(&mut self, fabric: &mut Fabric) -> bool {
        // Find all push intervals
        let push_keys: Vec<IntervalKey> = fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.role == Role::Pushing {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        // Need at least 2 pushes to maintain any structure
        if push_keys.len() <= 2 {
            return false;
        }

        // Pick a random push interval
        let idx = self.rng.random_range(0..push_keys.len());
        let push_key = push_keys[idx];

        // Get the joints before removing
        let (alpha_key, omega_key) = {
            let interval = fabric.intervals.get(push_key).unwrap();
            (interval.alpha_key, interval.omega_key)
        };

        // Remove the push interval
        fabric.intervals.remove(push_key);

        // Remove any pulls connected to these joints
        let pulls_to_remove: Vec<IntervalKey> = fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.alpha_key == alpha_key
                    || interval.omega_key == alpha_key
                    || interval.alpha_key == omega_key
                    || interval.omega_key == omega_key
                {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        for key in pulls_to_remove {
            fabric.intervals.remove(key);
        }

        // Remove the orphaned joints
        fabric.joints.remove(alpha_key);
        fabric.joints.remove(omega_key);

        true
    }

    /// Find the bounding box of the fabric.
    fn find_bounds(&self, fabric: &Fabric) -> (Vec3, Vec3) {
        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for joint in fabric.joints.values() {
            min = min.min(joint.location);
            max = max.max(joint.location);
        }

        if min.x == f32::MAX {
            // Empty fabric, return origin
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            (min, max)
        }
    }

    /// Try to add more pull connections between existing joints using V patterns.
    /// Focuses on connecting joints from different pushes to create volume.
    /// Returns true if any new connections were made.
    pub fn add_more_connections(&mut self, fabric: &mut Fabric) -> bool {
        let max_pull_length = self.config.push_length.f32() * self.config.max_pull_ratio;
        let joints: Vec<JointKey> = fabric.joints.keys().collect();
        let push_pairs = self.find_push_pairs(fabric, &joints);
        let mut added = false;

        // For each pair of pushes, ensure V patterns exist
        for i in 0..push_pairs.len() {
            for j in (i + 1)..push_pairs.len() {
                let (a_alpha, a_omega) = push_pairs[i];
                let (b_alpha, b_omega) = push_pairs[j];

                // Try to create all four V-pattern connections
                let pairs = [
                    (a_alpha, b_alpha),
                    (a_alpha, b_omega),
                    (a_omega, b_alpha),
                    (a_omega, b_omega),
                ];

                for (p, q) in pairs {
                    if fabric.interval_between(p, q).is_some() {
                        continue;
                    }
                    let dist = fabric.location(p).distance(fabric.location(q));
                    if dist <= max_pull_length && dist > 0.01 {
                        // Use force_connect for approaching intervals
                        self.force_connect(fabric, p, q);
                        added = true;
                    }
                }
            }
        }

        added
    }

    /// Settle the fabric with physics for the specified number of seconds.
    pub fn settle(&self, fabric: &mut Fabric, physics: &Physics, seconds: f32) {
        let iterations = (seconds / ITERATION_DURATION.secs) as usize;
        for _ in 0..iterations {
            fabric.iterate(physics);
        }
    }

    /// Settle the seed structure.
    pub fn settle_seed(&self, fabric: &mut Fabric, physics: &Physics) {
        self.settle(fabric, physics, self.config.seed_settle_seconds);
    }

    /// Settle after a mutation.
    pub fn settle_mutation(&self, fabric: &mut Fabric, physics: &Physics) {
        self.settle(fabric, physics, self.config.mutation_settle_seconds);
    }

    /// Connect existing push endpoints with pulls using "V" patterns.
    /// Each endpoint connects to BOTH ends of other pushes to create tent-like structures.
    fn connect_with_pulls(&mut self, fabric: &mut Fabric, push_joints: &[(JointKey, JointKey)]) {
        // For each push, connect both its ends to both ends of every other push
        // This creates "V" patterns that can generate volume
        for i in 0..push_joints.len() {
            for j in (i + 1)..push_joints.len() {
                let (a_alpha, a_omega) = push_joints[i];
                let (b_alpha, b_omega) = push_joints[j];

                // Connect all four combinations to maximize triangulation
                // a_alpha connects to both b_alpha and b_omega (V pattern)
                self.force_connect(fabric, a_alpha, b_alpha);
                self.force_connect(fabric, a_alpha, b_omega);
                // a_omega connects to both b_alpha and b_omega (V pattern)
                self.force_connect(fabric, a_omega, b_alpha);
                self.force_connect(fabric, a_omega, b_omega);
            }
        }
    }

    /// Force a connection between two joints with an approaching pull interval.
    /// The pull will contract to pull_target_ratio of current distance.
    fn force_connect(&mut self, fabric: &mut Fabric, a: JointKey, b: JointKey) {
        if fabric.interval_between(a, b).is_none() {
            let current_dist = fabric.distance(a, b);
            let target_length = Meters(current_dist.f32() * self.config.pull_target_ratio);
            fabric.create_approaching_interval(
                a,
                b,
                target_length,
                Role::Pulling,
                self.config.pull_approach_seconds,
            );
        }
    }

    /// Count how many intervals connect to a joint.
    fn connection_count(&self, fabric: &Fabric, joint: JointKey) -> usize {
        fabric.intervals.values()
            .filter(|i| i.alpha_key == joint || i.omega_key == joint)
            .count()
    }

    /// Connect a new push to existing structure, favoring less-connected joints.
    fn connect_new_push(
        &mut self,
        fabric: &mut Fabric,
        new_alpha: JointKey,
        new_omega: JointKey,
        existing_joints: &[JointKey],
    ) {
        let max_pull_length = self.config.push_length.f32() * self.config.max_pull_ratio;
        let new_alpha_loc = fabric.location(new_alpha);
        let new_omega_loc = fabric.location(new_omega);
        let new_center = (new_alpha_loc + new_omega_loc) / 2.0;

        // Score each existing joint: prefer close AND less-connected
        // Score = distance + connection_count * distance_penalty
        let distance_penalty = 0.5; // Each connection adds this much "virtual distance"

        let mut scored_joints: Vec<_> = existing_joints
            .iter()
            .filter(|&&j| j != new_alpha && j != new_omega)
            .map(|&j| {
                let dist = fabric.location(j).distance(new_center);
                let connections = self.connection_count(fabric, j);
                let score = dist + connections as f32 * distance_penalty;
                (j, dist, score)
            })
            .filter(|(_, dist, _)| *dist <= max_pull_length)
            .collect();

        // Sort by score (lower is better: closer AND less connected)
        scored_joints.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        // Connect to best-scored joints (up to 6 connections total)
        let max_connections = 6;
        let mut connections_made = 0;

        for (existing_joint, _dist, _score) in scored_joints {
            if connections_made >= max_connections {
                break;
            }

            // Connect both new endpoints to this joint if in range
            let alpha_dist = fabric.location(existing_joint).distance(new_alpha_loc);
            let omega_dist = fabric.location(existing_joint).distance(new_omega_loc);

            if alpha_dist <= max_pull_length {
                self.force_connect(fabric, new_alpha, existing_joint);
                connections_made += 1;
            }
            if omega_dist <= max_pull_length && connections_made < max_connections {
                self.force_connect(fabric, new_omega, existing_joint);
                connections_made += 1;
            }
        }
    }

    /// Find pairs of joints that are connected by push intervals.
    fn find_push_pairs(&self, fabric: &Fabric, joints: &[JointKey]) -> Vec<(JointKey, JointKey)> {
        let mut pairs = Vec::new();
        for interval in fabric.intervals.values() {
            if interval.role == Role::Pushing {
                // Only include if both joints are in our list
                if joints.contains(&interval.alpha_key) && joints.contains(&interval.omega_key) {
                    pairs.push((interval.alpha_key, interval.omega_key));
                }
            }
        }
        pairs
    }

    /// Generate a random unit direction vector.
    fn random_direction(&mut self) -> Vec3 {
        let x = self.rng.random_range(-1.0..1.0);
        let y = self.rng.random_range(-1.0..1.0);
        let z = self.rng.random_range(-1.0..1.0);
        Vec3::new(x, y, z).normalize_or_zero()
    }
}
