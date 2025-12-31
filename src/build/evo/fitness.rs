use crate::fabric::Fabric;

/// Trait for fitness evaluation strategies.
///
/// Different fitness functions can reward different properties of structures:
/// - Suspended joints (keeps things in the air)
/// - Height (maximizes vertical extent)
/// - etc.
pub trait FitnessFunction: Send + Sync {
    /// Name of this fitness function (for display and config).
    fn name(&self) -> &'static str;

    /// Evaluate the fitness of a fabric.
    /// Returns 0.0 for invalid/collapsed structures.
    fn evaluate(&self, fabric: &Fabric, push_count: usize) -> f32;

    /// Get detailed fitness breakdown for display.
    fn evaluate_detailed(&self, fabric: &Fabric, push_count: usize) -> FitnessDetails;
}

/// Get a fitness function by name.
pub fn get_fitness_function(name: &str) -> Option<Box<dyn FitnessFunction>> {
    match name {
        "suspended" | "default" => Some(Box::new(SuspendedJointsFitness::default())),
        "height" => Some(Box::new(HeightFitness::default())),
        _ => None,
    }
}

/// List available fitness function names.
pub fn available_fitness_functions() -> Vec<&'static str> {
    vec!["suspended", "height"]
}

// ============================================================================
// Common utilities for fitness functions
// ============================================================================

/// Calculate the maximum absolute strain in the structure.
fn calculate_max_strain(fabric: &Fabric) -> f32 {
    fabric
        .intervals
        .values()
        .map(|interval| interval.strain.abs())
        .fold(0.0f32, |a, b| a.max(b))
}

/// Calculate the height of the structure (max_y - min_y).
fn calculate_height(fabric: &Fabric) -> f32 {
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

// ============================================================================
// SuspendedJointsFitness - rewards keeping joints in the air
// ============================================================================

/// Fitness function that rewards suspended joints.
///
/// Fitness = sum of suspended joint heights / sqrt(interval_count)
/// This rewards holding joints high while penalizing excessive material use.
#[derive(Clone, Debug)]
pub struct SuspendedJointsFitness {
    /// Maximum allowable strain before structure is considered failed
    pub max_strain_threshold: f32,
    /// Height threshold for a joint to be considered "suspended"
    pub suspension_threshold: f32,
}

impl Default for SuspendedJointsFitness {
    fn default() -> Self {
        Self {
            max_strain_threshold: 0.5,    // 50% strain is failure
            suspension_threshold: 0.05,   // 5cm above surface = suspended
        }
    }
}

impl SuspendedJointsFitness {
    /// Sum the heights of all joints above the surface threshold.
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
}

impl FitnessFunction for SuspendedJointsFitness {
    fn name(&self) -> &'static str {
        "suspended"
    }

    fn evaluate(&self, fabric: &Fabric, push_count: usize) -> f32 {
        // Check for empty or trivial structures
        if fabric.joints.len() < 2 || push_count == 0 {
            return 0.0;
        }

        // Check stability (max strain) - broken structures get zero fitness
        let max_strain = calculate_max_strain(fabric);
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

    fn evaluate_detailed(&self, fabric: &Fabric, push_count: usize) -> FitnessDetails {
        let height = calculate_height(fabric);
        let suspended_joints = self.count_suspended_joints(fabric);
        let interval_count = fabric.intervals.len();
        let max_strain = calculate_max_strain(fabric);
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

// ============================================================================
// HeightFitness - rewards tall structures
// ============================================================================

/// Fitness function that rewards height.
///
/// Fitness = height^2 / sqrt(interval_count)
/// This rewards tall structures while penalizing excessive material use.
#[derive(Clone, Debug)]
pub struct HeightFitness {
    /// Maximum allowable strain before structure is considered failed
    pub max_strain_threshold: f32,
    /// Minimum height to be considered valid
    pub min_height_threshold: f32,
}

impl Default for HeightFitness {
    fn default() -> Self {
        Self {
            max_strain_threshold: 0.5,    // 50% strain is failure
            min_height_threshold: 0.05,   // 5cm minimum height
        }
    }
}

impl HeightFitness {
    /// Count joints above a threshold (for display).
    fn count_elevated_joints(&self, fabric: &Fabric) -> usize {
        fabric.joints.values()
            .filter(|j| j.location.y > self.min_height_threshold)
            .count()
    }
}

impl FitnessFunction for HeightFitness {
    fn name(&self) -> &'static str {
        "height"
    }

    fn evaluate(&self, fabric: &Fabric, push_count: usize) -> f32 {
        // Check for empty or trivial structures
        if fabric.joints.len() < 2 || push_count == 0 {
            return 0.0;
        }

        // Check stability (max strain) - broken structures get zero fitness
        let max_strain = calculate_max_strain(fabric);
        if max_strain > self.max_strain_threshold {
            return 0.0;
        }

        let height = calculate_height(fabric);
        if height < self.min_height_threshold {
            return 0.0;
        }

        // height^2 rewards tall structures more than linearly
        let height_score = height * height;

        // Cost = sqrt(interval count) - penalizes using too many intervals
        let interval_count = fabric.intervals.len();
        let cost = (interval_count as f32).sqrt();

        height_score / cost
    }

    fn evaluate_detailed(&self, fabric: &Fabric, push_count: usize) -> FitnessDetails {
        let height = calculate_height(fabric);
        let suspended_joints = self.count_elevated_joints(fabric);
        let interval_count = fabric.intervals.len();
        let max_strain = calculate_max_strain(fabric);
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

// ============================================================================
// FitnessEvaluator - wrapper for backward compatibility
// ============================================================================

/// Wrapper that provides a simple interface to fitness evaluation.
/// Uses SuspendedJointsFitness by default.
pub struct FitnessEvaluator {
    inner: Box<dyn FitnessFunction>,
}

impl Default for FitnessEvaluator {
    fn default() -> Self {
        Self {
            inner: Box::new(SuspendedJointsFitness::default()),
        }
    }
}

impl FitnessEvaluator {
    /// Create a new fitness evaluator with default (suspended) fitness function.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a fitness evaluator with a specific fitness function.
    pub fn with_function(fitness_function: Box<dyn FitnessFunction>) -> Self {
        Self {
            inner: fitness_function,
        }
    }

    /// Evaluate the fitness of a fabric.
    pub fn evaluate(&self, fabric: &Fabric, push_count: usize) -> f32 {
        self.inner.evaluate(fabric, push_count)
    }

    /// Get detailed fitness breakdown.
    pub fn evaluate_detailed(&self, fabric: &Fabric, push_count: usize) -> FitnessDetails {
        self.inner.evaluate_detailed(fabric, push_count)
    }

    /// Get the name of the current fitness function.
    pub fn name(&self) -> &'static str {
        self.inner.name()
    }
}

// ============================================================================
// FitnessDetails - detailed breakdown for display
// ============================================================================

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
    /// Final fitness score
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
