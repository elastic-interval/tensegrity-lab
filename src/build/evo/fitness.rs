use crate::fabric::Fabric;

/// Evaluates the fitness of structures based on suspended joints.
///
/// Fitness = number of joints above the surface
/// This rewards structures for holding joints in the air, regardless of how tall they are.
/// A structure that falls over but keeps many joints elevated is still valuable.
pub struct FitnessEvaluator {
    /// Maximum allowable strain before structure is considered failed
    pub max_strain_threshold: f32,
    /// Height threshold for a joint to be considered "suspended"
    pub suspension_threshold: f32,
}

impl Default for FitnessEvaluator {
    fn default() -> Self {
        Self {
            max_strain_threshold: 0.5,    // 50% strain is failure
            suspension_threshold: 0.05,   // 5cm above surface = suspended
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
    /// Fitness = sum of suspended joint heights / sqrt(interval_count)
    /// This rewards holding joints high while penalizing excessive material use.
    /// Returns 0.0 for collapsed or overstrained structures.
    pub fn evaluate(&self, fabric: &Fabric, push_count: usize) -> f32 {
        // Check for empty or trivial structures
        if fabric.joints.len() < 2 || push_count == 0 {
            return 0.0;
        }

        // Check stability (max strain) - broken structures get zero fitness
        let max_strain = self.calculate_max_strain(fabric);
        if max_strain > self.max_strain_threshold {
            return 0.0;
        }

        // Sum of heights of all suspended joints
        let suspended_height = self.sum_suspended_heights(fabric);

        // Cost = sqrt(interval count) - penalizes using too many intervals
        let interval_count = fabric.intervals.len();
        let cost = (interval_count as f32).sqrt();

        suspended_height / cost
    }

    /// Sum the heights of all joints above the surface threshold.
    /// Each joint contributes its height above the threshold.
    fn sum_suspended_heights(&self, fabric: &Fabric) -> f32 {
        fabric.joints.values()
            .map(|j| (j.location.y - self.suspension_threshold).max(0.0))
            .sum()
    }

    /// Count joints above the suspension threshold.
    fn count_suspended_joints(&self, fabric: &Fabric) -> usize {
        fabric.joints.values()
            .filter(|j| j.location.y > self.suspension_threshold)
            .count()
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
        let suspended_joints = self.count_suspended_joints(fabric);
        let interval_count = fabric.intervals.len();
        let max_strain = self.calculate_max_strain(fabric);
        let fitness = self.evaluate(fabric, push_count);

        FitnessDetails {
            height,
            suspended_joints,
            interval_count,
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
    /// Height of the structure (max_y - min_y) - for display
    pub height: f32,
    /// Number of joints above the surface threshold
    pub suspended_joints: usize,
    /// Total number of intervals (cost factor)
    pub interval_count: usize,
    /// Maximum strain in any interval
    pub max_strain: f32,
    /// Number of push intervals
    pub push_count: usize,
    /// Final fitness score (suspended heights / sqrt(intervals))
    pub fitness: f32,
    /// Whether the structure is valid (non-zero fitness)
    pub is_valid: bool,
}

impl std::fmt::Display for FitnessDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fitness={:.3} ({}j suspended, {} intervals, ht={:.2}m)",
            self.fitness,
            self.suspended_joints,
            self.interval_count,
            self.height
        )
    }
}
