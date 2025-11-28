use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint};
use crate::build::dsl::brick_dsl::{BrickName, BrickParams, OmniParams, SingleParams, TorqueParams};
use crate::build::dsl::brick_library::get_prototype;
use cgmath::{Point3, Vector3};

pub fn get_baked_brick(brick_name: BrickName) -> BakedBrick {
    match brick_name {
        BrickName::SingleLeftBrick => single_left_baked(),
        BrickName::SingleRightBrick => single_right_baked(),
        BrickName::OmniBrick => omni_baked(),
        BrickName::TorqueBrick => torque_baked(),
    }
}

fn joint(x: f32, y: f32, z: f32) -> BakedJoint {
    BakedJoint { location: Point3::new(x, y, z) }
}

fn push(alpha: usize, omega: usize, strain: f32) -> BakedInterval {
    BakedInterval { alpha_index: alpha, omega_index: omega, strain, material_name: "push".to_string() }
}

fn pull(alpha: usize, omega: usize, strain: f32) -> BakedInterval {
    BakedInterval { alpha_index: alpha, omega_index: omega, strain, material_name: "pull".to_string() }
}

fn single_left_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::SingleLeft(SingleParams {
            push_lengths: Vector3::new(3.204, 3.204, 3.204),
            pull_length: 2.0,
        }),
                                                scale: 0.9149,
        joints: vec![
            joint(-1.1042, 0.0001, -0.0098),
            joint(0.9616, 1.9487, -0.5432),
            joint(0.5429, 0.0002, 0.9624),
            joint(-0.9510, 1.9477, -0.5628),
            joint(0.5611, 0.0000, -0.9505),
            joint(-0.0117, 1.9484, 1.1035),
        ],
        intervals: vec![
            push(0, 1, -0.0172),
            push(2, 3, -0.0171),
            push(4, 5, -0.0175),
            pull(0, 3, 0.1070),
            pull(2, 5, 0.1068),
            pull(4, 1, 0.1069),
        ],
        faces: get_prototype(BrickName::SingleLeftBrick).derive_baked_faces(),
    }
}

fn single_right_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::SingleRight(SingleParams {
            push_lengths: Vector3::new(3.204, 3.204, 3.204),
            pull_length: 2.0,
        }),
                                                scale: 0.9104,
        joints: vec![
            joint(-0.9571, 0.0000, 0.5495),
            joint(1.1020, 1.9323, 0.0022),
            joint(0.9525, 0.0000, 0.5567),
            joint(-0.5494, 1.9326, -0.9571),
            joint(0.0024, 0.0002, -1.1014),
            joint(-0.5526, 1.9328, 0.9526),
        ],
        intervals: vec![
            push(0, 1, -0.0169),
            push(2, 3, -0.0164),
            push(4, 5, -0.0176),
            pull(0, 5, 0.1046),
            pull(2, 1, 0.1045),
            pull(4, 3, 0.1039),
        ],
        faces: get_prototype(BrickName::SingleRightBrick).derive_baked_faces(),
    }
}

fn omni_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::Omni(OmniParams {
            push_lengths: Vector3::new(3.271, 3.271, 3.271),
        }),
                                        scale: 0.9672,
        joints: vec![
            joint(-1.5594, 1.5597, -0.7731),
            joint(1.5592, 1.5626, -0.7811),
            joint(-1.5592, 1.5619, 0.7811),
            joint(1.5594, 1.5648, 0.7731),
            joint(-0.7804, 0.0000, -0.0253),
            joint(-0.7759, 3.1194, 0.0292),
            joint(0.7759, 0.0052, -0.0292),
            joint(0.7803, 3.1245, 0.0253),
            joint(-0.0043, 0.7852, -1.5863),
            joint(0.0013, 0.7834, 1.5301),
            joint(-0.0013, 2.3411, -1.5301),
            joint(0.0044, 2.3393, 1.5863),
        ],
        intervals: vec![
            push(0, 1, -0.0158),
            push(2, 3, -0.0158),
            push(4, 5, -0.0154),
            push(6, 7, -0.0154),
            push(8, 9, -0.0166),
            push(10, 11, -0.0166),
        ],
        faces: get_prototype(BrickName::OmniBrick).derive_baked_faces(),
    }
}

fn torque_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::Torque(TorqueParams {
            push_lengths: Vector3::new(3.0, 3.0, 6.0),
            pull_length: 1.86,
        }),
                                        scale: 1.0177,
        joints: vec![
            joint(-1.5062, 1.5013, -2.2217),
            joint(1.5062, 1.5013, -2.2217),
            joint(-1.4999, 1.5013, 0.0000),
            joint(1.4999, 1.5013, 0.0000),
            joint(-1.5062, 1.5013, 2.2217),
            joint(1.5062, 1.5013, 2.2217),
            joint(-1.0600, 0.0000, -1.4016),
            joint(-1.0600, 3.0027, -1.4017),
            joint(-1.0600, 0.0000, 1.4016),
            joint(-1.0600, 3.0027, 1.4017),
            joint(1.0600, 0.0000, -1.4016),
            joint(1.0600, 3.0027, -1.4017),
            joint(1.0600, 0.0000, 1.4016),
            joint(1.0600, 3.0027, 1.4017),
            joint(-0.0000, 2.3190, -2.9884),
            joint(-0.0000, 2.3189, 2.9884),
            joint(-0.0000, 0.6837, -2.9884),
            joint(-0.0000, 0.6837, 2.9883),
        ],
        intervals: vec![
            push(0, 1, -0.0088),
            push(2, 3, -0.0130),
            push(4, 5, -0.0088),
            push(6, 7, -0.0121),
            push(8, 9, -0.0121),
            push(10, 11, -0.0121),
            push(12, 13, -0.0121),
            push(14, 15, -0.0185),
            push(16, 17, -0.0185),
            pull(2, 6, 0.1121),
            pull(2, 7, 0.1121),
            pull(2, 8, 0.1121),
            pull(2, 9, 0.1121),
            pull(3, 10, 0.1121),
            pull(3, 11, 0.1121),
            pull(3, 12, 0.1121),
            pull(3, 13, 0.1121),
        ],
        faces: get_prototype(BrickName::TorqueBrick).derive_baked_faces(),
    }
}
