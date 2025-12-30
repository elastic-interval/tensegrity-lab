use crate::fabric::Fabric;

/// Evaluates the fitness of grown structures.
///
/// Fitness is primarily based on height (taller = better), with penalties
/// for instability or collapse.
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
            min_height_threshold: 0.1,   // Must be at least 10cm tall
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
    /// Returns a fitness score where higher is better.
    /// Returns 0.0 for collapsed or invalid structures.
    pub fn evaluate(&self, fabric: &Fabric) -> f32 {
        // Check for empty or trivial structures
        if fabric.joints.len() < 2 {
            return 0.0;
        }

        // Calculate height (y-extent)
        let height = self.calculate_height(fabric);
        if height < self.min_height_threshold {
            return 0.0;
        }

        // Check stability (max strain)
        let max_strain = self.calculate_max_strain(fabric);
        if max_strain > self.max_strain_threshold {
            return 0.0;
        }

        // Base fitness is height
        let mut fitness = height;

        // Bonus for stability (lower strain is better)
        let stability_bonus = 1.0 - (max_strain / self.max_strain_threshold);
        fitness *= 1.0 + stability_bonus * 0.2; // Up to 20% bonus

        fitness
    }

    /// Calculate the height (y-extent) of the structure.
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
    pub fn evaluate_detailed(&self, fabric: &Fabric) -> FitnessDetails {
        let height = self.calculate_height(fabric);
        let max_strain = self.calculate_max_strain(fabric);
        let fitness = self.evaluate(fabric);

        FitnessDetails {
            height,
            max_strain,
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
    /// Final fitness score
    pub fitness: f32,
    /// Whether the structure is valid (non-zero fitness)
    pub is_valid: bool,
}
