pub mod baked_bricks;
pub mod equilibrium;
mod omni;
mod single;
mod torque;

pub use omni::omni;
pub use single::single_left;
pub use torque::torque;

use crate::build::dsl::brick::{BakedBrick, BrickPrototype};
use crate::build::dsl::brick_dsl::*;
use crate::build::dsl::Spin;
use glam::Vec3;
use std::sync::OnceLock;

static SINGLE_LEFT_PROTO: OnceLock<BrickPrototype> = OnceLock::new();
static OMNI_PROTO: OnceLock<BrickPrototype> = OnceLock::new();
static TORQUE_PROTO: OnceLock<BrickPrototype> = OnceLock::new();

pub fn get_prototype(brick_name: BrickName) -> BrickPrototype {
    match brick_name {
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
    // Mirror if the requested role's Attach face isn't present in the
    // direct baked brick. After `BakedBrick::mirror()` the alias roles
    // flip too, so the mirrored brick will have the requested role's
    // Attach face. Seeds bypass: their attach is the down-rotation, not
    // an Attach face alias.
    let direct = baked_bricks::get_baked_brick(brick_name);
    let needs_mirror = matches!(brick_role, BrickRole::OnSpin(_))
        && !direct.faces.iter().any(|f| {
            f.aliases.iter().any(|a| {
                a.brick_role == brick_role && matches!(a.face_name, FaceName::Attach(_))
            })
        });
    let mut baked = if needs_mirror { direct.mirror() } else { direct };
    for face in &mut baked.faces {
        face.aliases.retain(|alias| alias.brick_role == brick_role);
    }
    let centroid = baked.centroid();
    assert!(
        centroid.length() < 0.01,
        "Brick {:?} centroid is not at origin: {:?}. Re-bake the brick.",
        brick_name,
        centroid
    );
    let space = match brick_role {
        BrickRole::Seed(_) => baked.down_rotation(brick_role),
        BrickRole::OnSpin(Spin::Left) | BrickRole::OnSpin(Spin::Right) => {
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
