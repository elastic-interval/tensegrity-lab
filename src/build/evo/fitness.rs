use crate::fabric::Fabric;

/// Evaluates the fitness of structures with cost adjustment.
///
/// Fitness = height / sqrt(push_count)
/// This rewards tall structures but penalizes excessive material use.
pub struct FitnessEvaluator {
    /// Maximum allowable strain before structure is considered failed
    pub max_strain_threshold: f32,
    /// Minimum height to be considered a valid structure
    pub min_height_threshold: f32,
}

impl Default for FitnessEvaluator {
    fn default() -> Self {
        Self {
            max_strain_threshold: 0.5,   // 50% strain is failure
            min_height_threshold: 0.01,  // Must have some height
        }
    }
}

impl FitnessEvaluator {
    /// Create a new fitness evaluator with default thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluate the fitness of a fabric.
    ///
    /// Fitness = perceived_height / cost
    /// where perceived_height = height + suspension_bonus
    /// and suspension_bonus = lowest joint of the highest interval (rewards floating structures)
    /// Push intervals cost 4x as much as pull intervals.
    /// Returns 0.0 for collapsed or invalid structures.
    pub fn evaluate(&self, fabric: &Fabric, push_count: usize) -> f32 {
        // Check for empty or trivial structures
        if fabric.joints.len() < 2 || push_count == 0 {
            return 0.0;
        }

        // Calculate height (y-extent from floor)
        let height = self.calculate_height(fabric);
        if height < self.min_height_threshold {
            return 0.0;
        }

        // Check stability (max strain)
        let max_strain = self.calculate_max_strain(fabric);
        if max_strain > self.max_strain_threshold {
            return 0.0;
        }

        // Calculate suspension bonus: lowest joint of the highest interval
        // This rewards structures where even the top intervals are floating
        let suspension_bonus = self.calculate_suspension_bonus(fabric);

        // Perceived height includes suspension bonus
        let perceived_height = height + suspension_bonus;

        // Count pull intervals
        let pull_count = fabric.intervals.values()
            .filter(|i| i.role == crate::fabric::interval::Role::Pulling)
            .count();

        // Cost: push costs 4x as much as pull (linear, no sqrt)
        let cost = (push_count * 4 + pull_count) as f32;
        let fitness = perceived_height / cost;

        // Small bonus for stability (lower strain is better)
        let stability_bonus = 1.0 - (max_strain / self.max_strain_threshold);
        fitness * (1.0 + stability_bonus * 0.1) // Up to 10% bonus for low strain
    }

    /// Calculate the height of the structure (max_y - min_y, but from floor).
    /// For structures on a floor at y=0, this is just max_y.
    fn calculate_height(&self, fabric: &Fabric) -> f32 {
        if fabric.joints.is_empty() {
            return 0.0;
        }

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for joint in fabric.joints.values() {
            let y = joint.location.y;
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        if min_y == f32::MAX || max_y == f32::MIN {
            return 0.0;
        }

        // Height from lowest point (floor) to highest
        max_y - min_y
    }

    /// Calculate the maximum absolute strain in the structure.
    fn calculate_max_strain(&self, fabric: &Fabric) -> f32 {
        fabric
            .intervals
            .values()
            .map(|interval| interval.strain.abs())
            .fold(0.0f32, |a, b| a.max(b))
    }

    /// Calculate suspension bonus: the lowest joint of the highest interval.
    /// This rewards structures where intervals are floating in the air,
    /// not just tall with parts resting on the ground.
    fn calculate_suspension_bonus(&self, fabric: &Fabric) -> f32 {
        let mut highest_midpoint = f32::MIN;
        let mut lowest_joint_of_highest = 0.0f32;

        for interval in fabric.intervals.values() {
            let alpha_y = fabric.joints.get(interval.alpha_key)
                .map(|j| j.location.y)
                .unwrap_or(0.0);
            let omega_y = fabric.joints.get(interval.omega_key)
                .map(|j| j.location.y)
                .unwrap_or(0.0);

            let midpoint_y = (alpha_y + omega_y) / 2.0;
            let min_y = alpha_y.min(omega_y);

            if midpoint_y > highest_midpoint {
                highest_midpoint = midpoint_y;
                lowest_joint_of_highest = min_y;
            }
        }

        // Only give bonus if the lowest joint is above ground
        lowest_joint_of_highest.max(0.0)
    }

    /// Get detailed fitness breakdown for debugging/display.
    pub fn evaluate_detailed(&self, fabric: &Fabric, push_count: usize) -> FitnessDetails {
        let height = self.calculate_height(fabric);
        let suspension_bonus = self.calculate_suspension_bonus(fabric);
        let max_strain = self.calculate_max_strain(fabric);
        let fitness = self.evaluate(fabric, push_count);

        FitnessDetails {
            height,
            suspension_bonus,
            max_strain,
            push_count,
            fitness,
            is_valid: fitness > 0.0,
        }
    }
}

/// Detailed fitness breakdown for analysis.
#[derive(Debug, Clone)]
pub struct FitnessDetails {
    /// Height of the structure (y-extent)
    pub height: f32,
    /// Suspension bonus (lowest joint of highest interval)
    pub suspension_bonus: f32,
    /// Maximum strain in any interval
    pub max_strain: f32,
    /// Number of push intervals
    pub push_count: usize,
    /// Final fitness score
    pub fitness: f32,
    /// Whether the structure is valid (non-zero fitness)
    pub is_valid: bool,
}
