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
    /// Fitness = height / sqrt(push_count)
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

        // Cost-adjusted fitness: height / sqrt(push_count)
        // This rewards efficient structures that are tall with fewer pushes
        let cost = (push_count as f32).sqrt();
        let fitness = height / cost;

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

    /// Get detailed fitness breakdown for debugging/display.
    pub fn evaluate_detailed(&self, fabric: &Fabric, push_count: usize) -> FitnessDetails {
        let height = self.calculate_height(fabric);
        let max_strain = self.calculate_max_strain(fabric);
        let fitness = self.evaluate(fabric, push_count);

        FitnessDetails {
            height,
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
    /// Maximum strain in any interval
    pub max_strain: f32,
    /// Number of push intervals
    pub push_count: usize,
    /// Final fitness score
    pub fitness: f32,
    /// Whether the structure is valid (non-zero fitness)
    pub is_valid: bool,
}
