/*
 * Growth Genome - A simple genome that grows tensegrity structures
 *
 * This genome represents a tensegrity structure as:
 * - A list of push intervals (bars)
 * - A list of pull connections between push endpoints
 *
 * The adjacent possible includes:
 * - Sprouting: Add a new push from an existing endpoint
 * - Joining: Connect two endpoints with a pull
 * - Pruning: Remove a push (if structure remains connected)
 */

use crate::build::evo::traits::{ExpressionContext, Genome, GenomeId};
use crate::fabric::interval::Role;
use crate::fabric::Fabric;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::PI;

/// Which end of a push interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PushEnd {
    Alpha,
    Omega,
}

/// A gene representing a single push interval (bar).
#[derive(Clone, Debug)]
pub struct PushGene {
    /// Direction of the push (unit vector).
    pub direction: Vec3,
    /// Length of the push.
    pub length: f32,
    /// If Some, this push sprouts from another push's endpoint.
    pub parent: Option<(usize, PushEnd)>,
}

/// A gene representing a pull connection between two push endpoints.
#[derive(Clone, Debug)]
pub struct PullGene {
    /// First push index and endpoint.
    pub from: (usize, PushEnd),
    /// Second push index and endpoint.
    pub to: (usize, PushEnd),
}

#[derive(Clone, Debug)]
pub struct GrowthGenome {
    id: GenomeId,
    pushes: Vec<PushGene>,
    pulls: Vec<PullGene>,
}

impl GrowthGenome {
    pub fn new(_seed: u64) -> Self {
        Self {
            id: GenomeId::new(),
            pushes: vec![PushGene {
                direction: Vec3::Y,
                length: 1.0,
                parent: None,
            }],
            pulls: vec![],
        }
    }

    /// Create a random push direction perpendicular to the parent push.
    fn random_perpendicular(&self, parent_dir: Vec3, rng: &mut impl Rng) -> Vec3 {
        // Find a vector not parallel to parent
        let not_parallel = if parent_dir.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };

        // Get perpendicular plane basis vectors
        let perp1 = parent_dir.cross(not_parallel).normalize();
        let perp2 = parent_dir.cross(perp1).normalize();

        // Random angle in perpendicular plane
        let angle = rng.random_range(0.0..2.0 * PI);
        (perp1 * angle.cos() + perp2 * angle.sin()).normalize()
    }

    /// Get endpoint position of a push in the expressed structure.
    fn endpoint_position(&self, push_idx: usize, end: PushEnd) -> Vec3 {
        // Recursively calculate position from root
        self.calculate_position(push_idx, end, &mut std::collections::HashSet::new())
    }

    fn calculate_position(
        &self,
        push_idx: usize,
        end: PushEnd,
        visited: &mut std::collections::HashSet<usize>,
    ) -> Vec3 {
        if visited.contains(&push_idx) {
            return Vec3::ZERO; // Cycle detected
        }
        visited.insert(push_idx);

        let push = &self.pushes[push_idx];
        let half_length = push.length / 2.0;

        let center = match &push.parent {
            None => Vec3::ZERO, // Root push centered at origin
            Some((parent_idx, parent_end)) => {
                let parent_pos = self.calculate_position(*parent_idx, *parent_end, visited);
                parent_pos
            }
        };

        match end {
            PushEnd::Alpha => center - push.direction * half_length,
            PushEnd::Omega => center + push.direction * half_length,
        }
    }

    /// Try to generate a sprout mutation.
    fn try_sprout(&self, rng: &mut impl Rng) -> Option<Self> {
        if self.pushes.is_empty() {
            return None;
        }

        // Pick a random push to sprout from
        let parent_idx = rng.random_range(0..self.pushes.len());
        let parent_end = if rng.random_bool(0.5) {
            PushEnd::Alpha
        } else {
            PushEnd::Omega
        };

        let parent_push = &self.pushes[parent_idx];

        // Create new push perpendicular to parent
        let direction = self.random_perpendicular(parent_push.direction, rng);
        let length = rng.random_range(0.5..1.5);

        let mut new_genome = self.clone();
        new_genome.id = GenomeId::new();
        new_genome.pushes.push(PushGene {
            direction,
            length,
            parent: Some((parent_idx, parent_end)),
        });

        Some(new_genome)
    }

    /// Try to generate a join mutation (connect two endpoints with a pull).
    fn try_join(&self, rng: &mut impl Rng) -> Option<Self> {
        if self.pushes.len() < 2 {
            return None;
        }

        // Collect all endpoints
        let mut endpoints: Vec<(usize, PushEnd)> = vec![];
        for i in 0..self.pushes.len() {
            endpoints.push((i, PushEnd::Alpha));
            endpoints.push((i, PushEnd::Omega));
        }

        // Pick two different endpoints
        let idx1 = rng.random_range(0..endpoints.len());
        let idx2 = loop {
            let idx = rng.random_range(0..endpoints.len());
            if idx != idx1 {
                break idx;
            }
        };

        let from = endpoints[idx1];
        let to = endpoints[idx2];

        // Check if this connection already exists
        for pull in &self.pulls {
            if (pull.from == from && pull.to == to) || (pull.from == to && pull.to == from) {
                return None;
            }
        }

        let mut new_genome = self.clone();
        new_genome.id = GenomeId::new();
        new_genome.pulls.push(PullGene { from, to });

        Some(new_genome)
    }

    /// Number of push intervals.
    pub fn push_count(&self) -> usize {
        self.pushes.len()
    }

    /// Number of pull connections.
    pub fn pull_count(&self) -> usize {
        self.pulls.len()
    }
}

impl Genome for GrowthGenome {
    fn id(&self) -> GenomeId {
        self.id
    }

    fn express(&self, _context: &ExpressionContext) -> Fabric {
        let mut fabric = Fabric::new(format!("Growth-{}", self.id.0));

        // Create joints for each push endpoint
        // Map: (push_idx, end) -> JointKey
        use std::collections::HashMap;
        let mut joint_map: HashMap<(usize, PushEnd), crate::fabric::JointKey> = HashMap::new();

        // First pass: create joints
        for (push_idx, _push) in self.pushes.iter().enumerate() {
            for end in [PushEnd::Alpha, PushEnd::Omega] {
                let pos = self.endpoint_position(push_idx, end);
                let key = fabric.create_joint(pos);
                joint_map.insert((push_idx, end), key);
            }
        }

        // Second pass: merge joints at shared positions (parent/child connections)
        // For simplicity, we'll just use separate joints and let physics handle it
        // A more sophisticated version would share joints at parent connections

        // Create push intervals
        for (push_idx, _push) in self.pushes.iter().enumerate() {
            let alpha = joint_map[&(push_idx, PushEnd::Alpha)];
            let omega = joint_map[&(push_idx, PushEnd::Omega)];
            fabric.create_slack_interval(alpha, omega, Role::Pushing);
        }

        // Create pull intervals
        for pull in &self.pulls {
            let from_joint = joint_map[&pull.from];
            let to_joint = joint_map[&pull.to];
            fabric.create_slack_interval(from_joint, to_joint, Role::Pulling);
        }

        fabric
    }

    fn adjacent_possible(&self, rng: &mut impl Rng) -> Vec<Self> {
        let mut variants = vec![];

        // Try to sprout a new push
        if let Some(sprouted) = self.try_sprout(rng) {
            variants.push(sprouted);
        }

        // Try to join two endpoints
        if let Some(joined) = self.try_join(rng) {
            variants.push(joined);
        }

        // Could also add: remove a push, modify a push length, etc.

        variants
    }

    fn describe(&self) -> String {
        format!(
            "GrowthGenome(id={}, pushes={}, pulls={})",
            self.id.0,
            self.pushes.len(),
            self.pulls.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::physics::presets::CONSTRUCTION;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn test_growth_genome_express() {
        let genome = GrowthGenome::new(42);
        let context = ExpressionContext::new(CONSTRUCTION);
        let fabric = genome.express(&context);

        assert_eq!(fabric.joints.len(), 2); // Two endpoints
        assert_eq!(fabric.intervals.len(), 1); // One push
    }

    #[test]
    fn test_growth_genome_adjacent_possible() {
        let genome = GrowthGenome::new(42);
        let mut rng = ChaCha8Rng::seed_from_u64(42);

        let variants = genome.adjacent_possible(&mut rng);

        // Should have at least one variant (sprout)
        assert!(!variants.is_empty());

        // Check that each variant has a different ID
        for variant in &variants {
            assert_ne!(variant.id(), genome.id());
        }
    }
}
