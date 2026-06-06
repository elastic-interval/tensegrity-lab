//! `BrickStructure` — direct joint+interval+face representation that the
//! articulation genome mutates. Kept separate from `BakedBrick` so the
//! genome can shift joints, rescale ideal lengths, move actuators, and
//! grow extra parts without going through the DSL build pipeline on
//! every individual.
//!
//! The bricks in this codebase are *face-centric*: their tension network
//! is the face radials (Omni has zero internal pulls — its 8 faces' 24
//! radials are the whole tensegrity). An articulating brick keeps that
//! self-tensioning face structure and adds one or more **actuators** —
//! contracting pulls between two faces' centres — as its moving parts.

use crate::build::dsl::brick::BakedBrick;
use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::brick_library::baked_bricks::get_baked_brick;
use crate::build::dsl::{FaceAlias, Spin};
use crate::fabric::interval::Role;
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::Meters;
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct BrickInterval {
    pub alpha: usize,
    pub omega: usize,
    pub ideal: f32,
}

/// A face the brick exposes: three structural joints, plus the spin/scale
/// and aliases needed to recreate it headlessly the way `attach_brick`
/// does (`src/fabric/brick.rs`).
#[derive(Clone, Debug)]
pub struct BrickFaceSpec {
    pub joints: [usize; 3],
    pub spin: Spin,
    pub scale: f32,
    pub aliases: Vec<FaceAlias>,
    /// Rest strain of this face's three radial intervals — the local
    /// pretension of the tension network. Lowering it softens the face
    /// (a candidate compliant joint); raising it stiffens. Evolvable.
    pub radial_strain: f32,
}

/// A muscle: a contracting pull between the centres of two of the brick's
/// faces. `contraction` is the fraction of rest length it pulls down to
/// when active (e.g. 0.7 = contracts to 70 %). Actuators never push.
#[derive(Clone, Debug)]
pub struct Actuator {
    pub alpha_face: usize,
    pub omega_face: usize,
    pub contraction: f32,
}

/// Result of materialising a `BrickStructure` into a `Fabric`, carrying
/// the references the trial needs (which intervals are actuators, which
/// joints are the brick's own structural joints).
pub struct Expressed {
    pub fabric: Fabric,
    pub actuators: Vec<IntervalKey>,
    pub structural_joints: Vec<JointKey>,
}

#[derive(Clone, Debug)]
pub struct BrickStructure {
    pub joints: Vec<Vec3>,
    pub pushes: Vec<BrickInterval>,
    pub pulls: Vec<BrickInterval>,
    pub faces: Vec<BrickFaceSpec>,
    pub actuators: Vec<Actuator>,
}

const DEFAULT_CONTRACTION: f32 = 0.7;

impl BrickStructure {
    pub fn from_single_twist_left() -> Self {
        Self::from_baked(BrickName::SingleTwistLeft)
    }

    /// Seed a structure from a baked static brick, promoting a pair of
    /// faces to an actuator. `from_brick` lets us compare seeds (e.g.
    /// SingleTwistLeft vs OmniSymmetrical).
    pub fn from_baked(brick_name: BrickName) -> Self {
        let baked = get_baked_brick(brick_name);
        let joints: Vec<Vec3> = baked.joints.iter().map(|j| j.location).collect();
        let mut pushes = Vec::new();
        let mut pulls = Vec::new();
        for iv in &baked.intervals {
            // ideal = actual / (1 + strain), inverting the strain captured at bake
            let actual = (joints[iv.alpha_index] - joints[iv.omega_index]).length();
            let ideal = actual / (1.0 + iv.strain);
            let entry = BrickInterval {
                alpha: iv.alpha_index,
                omega: iv.omega_index,
                ideal,
            };
            match iv.material_name.as_str() {
                "push" => pushes.push(entry),
                "pull" => pulls.push(entry),
                other => panic!("unknown baked material name: {other}"),
            }
        }
        let faces: Vec<BrickFaceSpec> = baked
            .faces
            .iter()
            .map(|f| BrickFaceSpec {
                joints: f.joints,
                spin: f.spin,
                scale: f.scale,
                aliases: f.aliases.clone(),
                radial_strain: BakedBrick::TARGET_FACE_STRAIN,
            })
            .collect();
        assert!(
            faces.len() >= 2,
            "seed brick {brick_name:?} needs >= 2 faces for an actuator"
        );
        let actuators = vec![Actuator {
            alpha_face: 0,
            omega_face: 1,
            contraction: DEFAULT_CONTRACTION,
        }];
        Self {
            joints,
            pushes,
            pulls,
            faces,
            actuators,
        }
    }

    fn face_midpoint(&self, face: &BrickFaceSpec) -> Vec3 {
        face.joints.iter().map(|&i| self.joints[i]).sum::<Vec3>() / 3.0
    }

    pub fn express(&self, name: String) -> Expressed {
        let mut fabric = Fabric::new(name);
        let structural_joints: Vec<JointKey> = self
            .joints
            .iter()
            .map(|p| fabric.create_joint(*p))
            .collect();
        for push in &self.pushes {
            fabric.create_fixed_interval(
                structural_joints[push.alpha],
                structural_joints[push.omega],
                Role::Pushing,
                Meters(push.ideal),
            );
        }
        for pull in &self.pulls {
            fabric.create_fixed_interval(
                structural_joints[pull.alpha],
                structural_joints[pull.omega],
                Role::Pulling,
                Meters(pull.ideal),
            );
        }
        // Recreate each face: a middle joint at the triangle centroid plus
        // three FaceRadial intervals, exactly as `attach_brick` does. The
        // radial network is what self-tensions the brick.
        let face_keys: Vec<_> = self
            .faces
            .iter()
            .map(|face| {
                let middle = fabric.create_joint(self.face_midpoint(face));
                let radials = face.joints.map(|local| {
                    fabric.create_strained_interval(
                        middle,
                        structural_joints[local],
                        Role::FaceRadial,
                        face.radial_strain,
                    )
                });
                fabric.create_face(face.aliases.clone(), face.scale, face.spin, radials)
            })
            .collect();
        // Actuators: contracting pulls between two faces' centres, created
        // at rest length (strain ~0) so they start dormant.
        let actuators = self
            .actuators
            .iter()
            .map(|actuator| {
                let alpha = fabric.face(face_keys[actuator.alpha_face]).middle_joint(&fabric);
                let omega = fabric.face(face_keys[actuator.omega_face]).middle_joint(&fabric);
                let rest = fabric.distance(alpha, omega);
                fabric.create_fixed_interval(alpha, omega, Role::Pulling, rest)
            })
            .collect();
        Expressed {
            fabric,
            actuators,
            structural_joints,
        }
    }
}
