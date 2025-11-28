mod single_right;
mod single_left;
mod omni;
mod torque;

pub use single_right::single_right;
pub use single_left::single_left;
pub use omni::omni;
pub use torque::torque;

use crate::build::dsl::brick::{BakedBrick, Brick, Prototype};
use crate::build::dsl::brick_dsl::*;
use cgmath::{SquareMatrix, Vector3};
use std::sync::OnceLock;

static SINGLE_RIGHT: OnceLock<Brick> = OnceLock::new();
static SINGLE_LEFT: OnceLock<Brick> = OnceLock::new();
static OMNI: OnceLock<Brick> = OnceLock::new();
static TORQUE: OnceLock<Brick> = OnceLock::new();

fn get_brick_definition(brick_name: BrickName) -> &'static Brick {
    match brick_name {
        BrickName::SingleLeftBrick => SINGLE_LEFT.get_or_init(|| single_left(Vector3::new(3.204, 3.204, 3.204), 2.0)),
        BrickName::SingleRightBrick => SINGLE_RIGHT.get_or_init(|| single_right(Vector3::new(3.204, 3.204, 3.204), 2.0)),
        BrickName::OmniBrick => OMNI.get_or_init(|| omni(Vector3::new(3.271, 3.271, 3.271))),
        BrickName::TorqueBrick => TORQUE.get_or_init(|| torque(Vector3::new(3.0, 3.0, 6.0), 1.86)),
    }
}

pub fn get_prototype(brick_name: BrickName) -> Prototype {
    get_brick_definition(brick_name).prototype.clone()
}

pub fn get_brick(brick_name: BrickName, brick_role: BrickRole) -> BakedBrick {
    let brick = get_brick_definition(brick_name);
    let mut baked = brick.baked.clone();
    for face in &mut baked.faces {
        face.aliases.retain(|alias| alias.brick_role == brick_role);
    }
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
            face.vector_space(&baked).invert().unwrap()
        }
    };
    baked.apply_matrix(space);
    baked
}
