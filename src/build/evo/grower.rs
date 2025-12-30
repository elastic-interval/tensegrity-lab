use crate::fabric::interval::Role;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, JointKey};
use crate::units::{Meters, Seconds, Unit};
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

        // Starting elevation - high enough to fall and settle
        let base_height = push_length * 2.0;

        // Create N push intervals with random orientations around a central point
        for _ in 0..self.config.seed_push_count {
            let direction = self.random_direction();

            // Spread pushes in a small area at the starting height
            let spread = push_length * 0.3;
            let center = Vec3::new(
                self.rng.random_range(-spread..spread),
                base_height + self.rng.random_range(0.0..push_length),
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

    /// Mutate a fabric by adding one push interval that falls from above.
    /// The new push starts above the highest point of the structure and connects
    /// to nearby joints as it settles.
    /// Returns the new push count.
    pub fn mutate(&mut self, fabric: &mut Fabric, current_push_count: usize) -> usize {
        let push_length = self.config.push_length.f32();

        // Collect existing joints
        let joints: Vec<JointKey> = fabric.joints.keys().collect();
        if joints.is_empty() {
            return current_push_count;
        }

        // Find the highest point and center of the structure
        let (min_pos, max_pos) = self.find_bounds(fabric);
        let center_x = (min_pos.x + max_pos.x) / 2.0;
        let center_z = (min_pos.z + max_pos.z) / 2.0;
        let highest_y = max_pos.y;

        // Create new push above the structure
        // Random horizontal position within the structure's footprint
        let spread = (max_pos.x - min_pos.x).max(push_length);
        let new_center = Vec3::new(
            center_x + self.rng.random_range(-spread * 0.5..spread * 0.5),
            highest_y + push_length, // Start 1 push length above so pulls can reach
            center_z + self.rng.random_range(-spread * 0.5..spread * 0.5),
        );

        // Random direction for the push
        let direction = self.random_direction();
        let half = direction * push_length / 2.0;
        let new_alpha = fabric.create_joint(new_center - half);
        let new_omega = fabric.create_joint(new_center + half);
        fabric.create_slack_interval(new_alpha, new_omega, Role::Pushing);

        // Connect new push endpoints to nearby existing joints
        // They will be pulled down toward the structure
        self.connect_new_push(fabric, new_alpha, new_omega, &joints);

        current_push_count + 1
    }

    /// Mutate by shortening a random pull interval.
    /// This can pull the structure tighter and potentially increase height.
    /// Returns true if a mutation was applied.
    pub fn shorten_random_pull(&mut self, fabric: &mut Fabric) -> bool {
        use crate::fabric::IntervalKey;

        // Find all pull intervals
        let pull_keys: Vec<IntervalKey> = fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| {
                if interval.role == Role::Pulling {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        if pull_keys.is_empty() {
            return false;
        }

        // Pick a random pull interval
        let idx = self.rng.random_range(0..pull_keys.len());
        let pull_key = pull_keys[idx];

        // Get current ideal length and shorten it
        if let Some(interval) = fabric.intervals.get(pull_key) {
            let current_ideal = interval.ideal();
            // Shorten by 5-15%
            let shrink_factor = self.rng.random_range(0.85..0.95);
            let new_target = Meters(current_ideal.f32() * shrink_factor);

            // Use extend_interval to set new approaching target
            fabric.extend_interval(pull_key, new_target, self.config.pull_approach_seconds);
            return true;
        }

        false
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
        // Each iteration = 50 microseconds = 0.00005 seconds
        let iterations = (seconds / 0.00005) as usize;
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

    /// Connect a new push to existing structure using V patterns.
    /// Each new endpoint connects to BOTH ends of nearby existing pushes.
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

        // Find push intervals in the existing structure
        // A push connects two joints - we want to connect to BOTH ends
        let push_pairs = self.find_push_pairs(fabric, existing_joints);

        // Sort push pairs by distance to new push center
        let new_center = (new_alpha_loc + new_omega_loc) / 2.0;
        let mut pairs_by_dist: Vec<_> = push_pairs
            .iter()
            .map(|&(a, b)| {
                let pair_center = (fabric.location(a) + fabric.location(b)) / 2.0;
                let dist = pair_center.distance(new_center);
                (a, b, dist)
            })
            .collect();
        pairs_by_dist.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

        // Connect new alpha and omega to BOTH ends of nearest push pairs
        let max_v_connections = 2; // Connect to 2 existing pushes (4 joints each end)
        let mut v_connections = 0;

        for (existing_alpha, existing_omega, _dist) in pairs_by_dist {
            if v_connections >= max_v_connections {
                break;
            }

            // Check if connections are within range
            let alpha_to_ea = fabric.location(existing_alpha).distance(new_alpha_loc);
            let alpha_to_eo = fabric.location(existing_omega).distance(new_alpha_loc);
            let omega_to_ea = fabric.location(existing_alpha).distance(new_omega_loc);
            let omega_to_eo = fabric.location(existing_omega).distance(new_omega_loc);

            // Only connect if at least some connections are in range
            let in_range = alpha_to_ea <= max_pull_length
                || alpha_to_eo <= max_pull_length
                || omega_to_ea <= max_pull_length
                || omega_to_eo <= max_pull_length;

            if in_range {
                // Connect new_alpha to both ends of existing push (V pattern)
                if alpha_to_ea <= max_pull_length {
                    self.force_connect(fabric, new_alpha, existing_alpha);
                }
                if alpha_to_eo <= max_pull_length {
                    self.force_connect(fabric, new_alpha, existing_omega);
                }
                // Connect new_omega to both ends of existing push (V pattern)
                if omega_to_ea <= max_pull_length {
                    self.force_connect(fabric, new_omega, existing_alpha);
                }
                if omega_to_eo <= max_pull_length {
                    self.force_connect(fabric, new_omega, existing_omega);
                }
                v_connections += 1;
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
