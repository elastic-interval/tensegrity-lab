pub mod baked_bricks;
mod omni;
mod single;
mod torque;

pub use omni::omni;
pub use single::single_left;
pub use torque::torque;

use crate::build::dsl::brick::{BakedBrick, BrickPrototype};
use crate::build::dsl::brick_dsl::*;
use glam::Vec3;
use std::sync::OnceLock;

static SINGLE_LEFT_PROTO: OnceLock<BrickPrototype> = OnceLock::new();
static OMNI_PROTO: OnceLock<BrickPrototype> = OnceLock::new();
static TORQUE_PROTO: OnceLock<BrickPrototype> = OnceLock::new();

/// Returns true if this brick is derived from another (via mirror, etc.)
/// Derived bricks should not be baked directly.
pub fn is_derived(brick_name: BrickName) -> bool {
    matches!(brick_name, BrickName::SingleTwistRight)
}

pub fn get_prototype(brick_name: BrickName) -> BrickPrototype {
    match brick_name {
        BrickName::SingleTwistRight => {
            panic!("SingleTwistRight is derived via mirror() - use SingleTwistLeft prototype")
        }
        BrickName::SingleTwistLeft => SINGLE_LEFT_PROTO
            .get_or_init(|| {
                single_left(&SingleParams {
                    push_lengths: Vec3::new(3.204, 3.204, 3.204),
                    pull_length: 2.0,
                })
            })
            .clone(),
        BrickName::OmniSymmetrical | BrickName::OmniTetrahedral => OMNI_PROTO
            .get_or_init(|| {
                omni(&OmniParams {
                    push_lengths: Vec3::new(3.271, 3.271, 3.271),
                })
            })
            .clone(),
        BrickName::TorqueSymmetrical => TORQUE_PROTO
            .get_or_init(|| {
                torque(&TorqueParams {
                    push_lengths: Vec3::new(3.0, 3.0, 6.0),
                    pull_length: 1.86,
                })
            })
            .clone(),
    }
}

pub fn get_scale(brick_name: BrickName) -> f32 {
    baked_bricks::get_baked_brick(brick_name).scale
}

pub fn get_brick(brick_name: BrickName, brick_role: BrickRole) -> BakedBrick {
    // For OnSpinRight on role-mirrored bricks, mirror the OnSpinLeft version.
    // Single bricks handle chirality via separate Left/Right name variants.
    let needs_mirror = brick_role == BrickRole::OnSpinRight && brick_name.mirrors_for_role();
    let mut baked = if needs_mirror {
        baked_bricks::get_baked_brick(brick_name).mirror()
    } else {
        baked_bricks::get_baked_brick(brick_name)
    };
    for face in &mut baked.faces {
        face.aliases.retain(|alias| alias.brick_role == brick_role);
    }
    // Verify brick is centered at origin (baked bricks should already be centered)
    let centroid = baked.centroid();
    assert!(
        centroid.length() < 0.01,
        "Brick {:?} centroid is not at origin: {:?}. Re-bake the brick.",
        brick_name,
        centroid
    );
    let space = match brick_role {
        BrickRole::Seed(_) => baked.down_rotation(brick_role),
        BrickRole::OnSpinLeft | BrickRole::OnSpinRight => {
            let face = baked
                .faces
                .iter()
                .find(|face| {
                    face.aliases.iter().any(|alias| {
                        alias.brick_role == brick_role
                            && matches!(alias.face_name, FaceName::Attach(_))
                    })
                })
                .expect("Brick does not have any face aliases for this role");
            face.vector_space(&baked).inverse()
        }
    };
    baked.apply_matrix(space);
    baked
}
