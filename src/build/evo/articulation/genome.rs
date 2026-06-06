use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::Spin;
use crate::build::evo::articulation::structure::{Actuator, BrickFaceSpec, BrickStructure};
use crate::build::evo::traits::{ExpressionContext, Genome, GenomeId};
use crate::fabric::Fabric;
use glam::Vec3;
use rand::seq::SliceRandom;
use rand::Rng;

const JOINT_SHIFT: f32 = 0.06;
/// Free multiplicative range for a member's rest length — significant, so
/// the search can actually reshape the structure toward a compliant pose.
const RETUNE_LO: f32 = 0.75;
const RETUNE_HI: f32 = 1.33;
/// Additive step on a face's radial pretension, clamped to a load-bearing
/// band (never slack, never absurdly tight).
const STRAIN_STEP: f32 = 0.04;
const STRAIN_MIN: f32 = 0.01;
const STRAIN_MAX: f32 = 0.4;
const DEFAULT_CONTRACTION: f32 = 0.7;
const FACE_STRAIN_DEFAULT: f32 = 0.1;

#[derive(Clone, Debug)]
pub struct ArticulationGenome {
    id: GenomeId,
    structure: BrickStructure,
}

impl ArticulationGenome {
    pub fn seed() -> Self {
        // Omni is the purest face-centric brick (pushes + 8 faces, zero
        // internal pulls) and articulates ~2.4x more than SingleTwist for
        // the same actuator effort. See the diagnostic test.
        Self::from_brick(BrickName::OmniSymmetrical)
    }

    pub fn from_brick(brick_name: BrickName) -> Self {
        Self {
            id: GenomeId::new(),
            structure: BrickStructure::from_baked(brick_name),
        }
    }

    pub fn structure(&self) -> &BrickStructure {
        &self.structure
    }

    fn child(&self) -> Self {
        Self {
            id: GenomeId::new(),
            structure: self.structure.clone(),
        }
    }

    fn shift_joint(&self, rng: &mut impl Rng) -> Option<Self> {
        if self.structure.joints.is_empty() {
            return None;
        }
        let idx = rng.gen_range(0..self.structure.joints.len());
        let axis = match rng.gen_range(0..3) {
            0 => Vec3::X,
            1 => Vec3::Y,
            _ => Vec3::Z,
        };
        let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        let mut child = self.child();
        child.structure.joints[idx] += axis * sign * JOINT_SHIFT;
        Some(child)
    }

    /// Freely retune one push/pull rest length by a significant factor.
    fn retune_member(&self, rng: &mut impl Rng) -> Option<Self> {
        let n_push = self.structure.pushes.len();
        let n_pull = self.structure.pulls.len();
        let total = n_push + n_pull;
        if total == 0 {
            return None;
        }
        let idx = rng.gen_range(0..total);
        let factor = rng.gen_range(RETUNE_LO..RETUNE_HI);
        let mut child = self.child();
        if idx < n_push {
            child.structure.pushes[idx].ideal *= factor;
        } else {
            child.structure.pulls[idx - n_push].ideal *= factor;
        }
        Some(child)
    }

    /// Soften or stiffen one face's radial pretension — the tension
    /// network's local stiffness, and the most direct lever for a
    /// compliant joint. Omni carries no internal pulls, so this is the
    /// main way to retune its rest tensions.
    fn retune_face(&self, rng: &mut impl Rng) -> Option<Self> {
        if self.structure.faces.is_empty() {
            return None;
        }
        let idx = rng.gen_range(0..self.structure.faces.len());
        let sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        let mut child = self.child();
        let strain = &mut child.structure.faces[idx].radial_strain;
        *strain = (*strain + sign * STRAIN_STEP).clamp(STRAIN_MIN, STRAIN_MAX);
        if *strain == self.structure.faces[idx].radial_strain {
            return None;
        }
        Some(child)
    }

    /// Repoint one actuator at a different pair of faces.
    fn move_actuator(&self, rng: &mut impl Rng) -> Option<Self> {
        let n_face = self.structure.faces.len();
        if n_face < 2 || self.structure.actuators.is_empty() {
            return None;
        }
        let a_idx = rng.gen_range(0..self.structure.actuators.len());
        let alpha = rng.gen_range(0..n_face);
        let mut omega = rng.gen_range(0..n_face);
        if omega == alpha {
            omega = (omega + 1) % n_face;
        }
        let current = &self.structure.actuators[a_idx];
        if current.alpha_face == alpha && current.omega_face == omega {
            return None;
        }
        let mut child = self.child();
        child.structure.actuators[a_idx].alpha_face = alpha;
        child.structure.actuators[a_idx].omega_face = omega;
        Some(child)
    }

    /// Topological growth: add a new face spanning three structural
    /// joints, growing the tension network and creating a new actuator
    /// target. Most such faces won't yield a valid equilibrium — the rest
    /// gate filters those — but the occasional good one enriches the brick.
    fn add_face(&self, rng: &mut impl Rng) -> Option<Self> {
        let n = self.structure.joints.len();
        if n < 3 {
            return None;
        }
        let a = rng.gen_range(0..n);
        let b = (a + 1 + rng.gen_range(0..n - 1)) % n;
        let c = loop {
            let c = rng.gen_range(0..n);
            if c != a && c != b {
                break c;
            }
        };
        let mut sorted = [a, b, c];
        sorted.sort_unstable();
        let duplicate = self.structure.faces.iter().any(|f| {
            let mut s = f.joints;
            s.sort_unstable();
            s == sorted
        });
        if duplicate {
            return None;
        }
        let scale = self
            .structure
            .faces
            .first()
            .map(|f| f.scale)
            .unwrap_or(1.0);
        let mut child = self.child();
        child.structure.faces.push(BrickFaceSpec {
            joints: [a, b, c],
            spin: Spin::Left,
            scale,
            aliases: vec![],
            radial_strain: FACE_STRAIN_DEFAULT,
        });
        Some(child)
    }

    /// Topological pruning: remove a face (keeping at least two for the
    /// actuators), remapping the actuator face indices.
    fn remove_face(&self, rng: &mut impl Rng) -> Option<Self> {
        if self.structure.faces.len() <= 2 {
            return None;
        }
        let r = rng.gen_range(0..self.structure.faces.len());
        let remap = |x: usize| if x > r { x - 1 } else { x };
        let actuators: Vec<Actuator> = self
            .structure
            .actuators
            .iter()
            .filter(|a| a.alpha_face != r && a.omega_face != r)
            .map(|a| Actuator {
                alpha_face: remap(a.alpha_face),
                omega_face: remap(a.omega_face),
                contraction: a.contraction,
            })
            .collect();
        if actuators.is_empty() {
            return None;
        }
        let mut child = self.child();
        child.structure.faces.remove(r);
        child.structure.actuators = actuators;
        Some(child)
    }

    /// Grow a moving part: add an actuator across a face pair not already
    /// actuated. Needs enough faces to find a fresh pair.
    fn add_actuator(&self, rng: &mut impl Rng) -> Option<Self> {
        let n_face = self.structure.faces.len();
        if n_face < 2 {
            return None;
        }
        let used = |a: usize, b: usize| {
            self.structure.actuators.iter().any(|act| {
                (act.alpha_face == a && act.omega_face == b)
                    || (act.alpha_face == b && act.omega_face == a)
            })
        };
        let alpha = rng.gen_range(0..n_face);
        let mut omega = rng.gen_range(0..n_face);
        if omega == alpha {
            omega = (omega + 1) % n_face;
        }
        if used(alpha, omega) {
            return None;
        }
        let mut child = self.child();
        child.structure.actuators.push(Actuator {
            alpha_face: alpha,
            omega_face: omega,
            contraction: DEFAULT_CONTRACTION,
        });
        Some(child)
    }
}

impl Genome for ArticulationGenome {
    fn id(&self) -> GenomeId {
        self.id
    }

    fn express(&self, _context: &ExpressionContext) -> Fabric {
        self.structure
            .express(format!("Articulation-{}", self.id.0))
            .fabric
    }

    fn adjacent_possible(&self, rng: &mut impl Rng) -> Vec<Self> {
        // Shuffle so the population strategy (which applies the first
        // variant) draws a *random* mutation each time, not always the
        // first operator.
        let mut variants: Vec<Self> = [
            self.shift_joint(rng),
            self.retune_member(rng),
            self.retune_face(rng),
            self.move_actuator(rng),
            self.add_actuator(rng),
            self.add_face(rng),
            self.remove_face(rng),
        ]
        .into_iter()
        .flatten()
        .collect();
        variants.shuffle(rng);
        variants
    }

    fn describe(&self) -> String {
        format!(
            "ArticulationGenome(id={}, joints={}, pushes={}, pulls={}, faces={}, actuators={})",
            self.id.0,
            self.structure.joints.len(),
            self.structure.pushes.len(),
            self.structure.pulls.len(),
            self.structure.faces.len(),
            self.structure.actuators.len(),
        )
    }
}
