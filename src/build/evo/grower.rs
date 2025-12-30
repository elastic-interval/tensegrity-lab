use crate::build::evo::cell::Cell;
use crate::build::evo::decision_maker::DecisionMaker;
use crate::build::evo::genome::Genome;
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalEnd};
use crate::units::{Meters, Unit};
use glam::Vec3;

/// Configuration for structure growth.
#[derive(Clone, Debug)]
pub struct GrowthConfig {
    /// Length of push intervals (default: 1.0m)
    pub push_length: Meters,
    /// How far a cell can "see" to find connection targets
    pub perception_radius: f32,
    /// Maximum pull length as ratio of push length
    pub max_pull_ratio: f32,
    /// Number of physics iterations for settling
    pub settle_iterations: usize,
    /// Maximum growth steps
    pub max_steps: usize,
}

impl Default for GrowthConfig {
    fn default() -> Self {
        Self {
            push_length: Meters(1.0),
            perception_radius: 2.0,   // 2x push length
            max_pull_ratio: 1.0,      // pull ≤ push
            settle_iterations: 5000,
            max_steps: 50,
        }
    }
}

/// Result of a growth step.
pub enum GrowthResult {
    /// Growth can continue
    Continue,
    /// Growth is complete (max steps reached)
    Complete,
    /// Growth failed (structure collapsed or became invalid)
    Failed(String),
}

/// Manages the growth of a tensegrity structure from a genome.
pub struct Grower {
    /// The fabric being grown
    pub fabric: Fabric,
    /// Cells (push intervals) in the structure
    pub cells: Vec<Cell>,
    /// Decision maker with seeded RNG and genome
    pub decision_maker: DecisionMaker,
    /// Growth configuration
    pub config: GrowthConfig,
    /// Current growth step
    pub growth_step: usize,
    /// The seed used for this growth
    pub seed: u64,
}

impl Grower {
    /// Create a new grower with the given seed and genome.
    pub fn new(seed: u64, genome: Genome, config: GrowthConfig) -> Self {
        Self {
            fabric: Fabric::new(format!("Evo-{}", seed)),
            cells: Vec::new(),
            decision_maker: DecisionMaker::new(seed, genome),
            config,
            growth_step: 0,
            seed,
        }
    }

    /// Grow the structure to completion (all steps).
    pub fn grow_complete(&mut self) -> GrowthResult {
        while self.growth_step < self.config.max_steps {
            match self.grow_step() {
                GrowthResult::Continue => continue,
                result => return result,
            }
        }
        GrowthResult::Complete
    }

    /// Perform one growth step.
    pub fn grow_step(&mut self) -> GrowthResult {
        // First step: create initial cell
        if self.cells.is_empty() {
            self.spawn_first_cell();
            self.growth_step += 1;
            return GrowthResult::Continue;
        }

        // Pick a random cell
        let cell_idx = self.decision_maker.choose(self.cells.len());

        // Pick an end (alpha or omega)
        let end = if self.decision_maker.decide() {
            IntervalEnd::Alpha
        } else {
            IntervalEnd::Omega
        };

        // Find nearby endpoints from other cells
        let nearby = self.find_nearby_endpoints(cell_idx, end);

        if !nearby.is_empty() && self.decision_maker.decide() {
            // Try to connect to an existing endpoint
            self.try_connect(cell_idx, end, &nearby);
        } else if self.cells.len() < 100 {
            // Spawn a new cell nearby (limit total cells)
            self.spawn_cell_near(cell_idx, end);
        }

        self.growth_step += 1;

        if self.growth_step >= self.config.max_steps {
            GrowthResult::Complete
        } else {
            GrowthResult::Continue
        }
    }

    /// Create the first cell at the origin, pointing up.
    fn spawn_first_cell(&mut self) {
        let cell = Cell::new(
            &mut self.fabric,
            Vec3::ZERO,
            Vec3::Y,
            self.config.push_length.f32(),
        );
        self.cells.push(cell);
    }

    /// Find endpoints of other cells within perception radius.
    /// Returns vec of (cell_index, end, distance).
    fn find_nearby_endpoints(
        &self,
        from_cell_idx: usize,
        from_end: IntervalEnd,
    ) -> Vec<(usize, IntervalEnd, f32)> {
        let from_joint = self.cells[from_cell_idx].joint_at(from_end);
        let from_location = self.fabric.location(from_joint);
        let max_dist = self.config.push_length.f32() * self.config.max_pull_ratio;

        let mut nearby = Vec::new();

        for (idx, cell) in self.cells.iter().enumerate() {
            if idx == from_cell_idx {
                continue; // Don't connect to self
            }

            for (joint, end) in cell.endpoints() {
                let location = self.fabric.location(joint);
                let dist = from_location.distance(location);

                // Within range and not too close (avoid zero-length pulls)
                if dist <= max_dist && dist > 0.01 {
                    // Check not already connected
                    if self.fabric.interval_between(from_joint, joint).is_none() {
                        nearby.push((idx, end, dist));
                    }
                }
            }
        }

        // Sort by distance (closest first)
        nearby.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        nearby
    }

    /// Try to connect two endpoints with a pull interval.
    fn try_connect(
        &mut self,
        from_cell_idx: usize,
        from_end: IntervalEnd,
        nearby: &[(usize, IntervalEnd, f32)],
    ) {
        // Pick from nearby (biased toward closer)
        let pick_count = nearby.len().min(3);
        if pick_count == 0 {
            return;
        }
        let target_idx = self.decision_maker.choose(pick_count);
        let (to_cell_idx, to_end, _dist) = nearby[target_idx];

        // Check both ends can accept more pulls
        if !self.cells[from_cell_idx].can_accept_pull(from_end) {
            return;
        }
        if !self.cells[to_cell_idx].can_accept_pull(to_end) {
            return;
        }

        // Get joint keys
        let from_joint = self.cells[from_cell_idx].joint_at(from_end);
        let to_joint = self.cells[to_cell_idx].joint_at(to_end);

        // Create pull interval
        let pull_key = self.fabric.create_slack_interval(from_joint, to_joint, Role::Pulling);

        // Track in both cells
        self.cells[from_cell_idx].add_pull(from_end, pull_key);
        self.cells[to_cell_idx].add_pull(to_end, pull_key);
    }

    /// Spawn a new cell near an existing cell's endpoint.
    fn spawn_cell_near(&mut self, parent_idx: usize, parent_end: IntervalEnd) {
        let parent_joint = self.cells[parent_idx].joint_at(parent_end);
        let parent_loc = self.fabric.location(parent_joint);

        // Random direction for new cell
        let direction = self.decision_maker.random_direction();
        let push_length = self.config.push_length.f32();

        // Place new cell nearby (offset in the random direction)
        let offset = direction * push_length * 0.6;
        let new_cell = Cell::new(&mut self.fabric, parent_loc + offset, direction, push_length);

        // Connect parent to the closer end of the new cell
        let alpha_dist = self.fabric.location(new_cell.alpha_joint).distance(parent_loc);
        let omega_dist = self.fabric.location(new_cell.omega_joint).distance(parent_loc);

        let (near_end, near_joint) = if alpha_dist < omega_dist {
            (IntervalEnd::Alpha, new_cell.alpha_joint)
        } else {
            (IntervalEnd::Omega, new_cell.omega_joint)
        };

        // Create pull from parent to new cell
        let pull_key = self.fabric.create_slack_interval(parent_joint, near_joint, Role::Pulling);

        // Track the new cell
        let new_cell_idx = self.cells.len();
        self.cells.push(new_cell);

        // Add pull to both cells
        self.cells[parent_idx].add_pull(parent_end, pull_key);
        self.cells[new_cell_idx].add_pull(near_end, pull_key);
    }

    /// Get a reference to the genome.
    pub fn genome(&self) -> &Genome {
        self.decision_maker.genome()
    }

    /// Clone the genome for creating offspring.
    pub fn clone_genome(&self) -> Genome {
        self.decision_maker.clone_genome()
    }

    /// Get the current virtual position (for mutation targeting).
    pub fn decision_position(&self) -> usize {
        self.decision_maker.virtual_position()
    }
}
