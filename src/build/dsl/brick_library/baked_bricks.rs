use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint, BrickFace};
use crate::build::dsl::brick_dsl::{
    BrickName, BrickParams, OmniParams, SingleParams, TorqueParams,
};
use crate::build::dsl::brick_library::get_prototype;
use glam::Vec3;

pub fn get_baked_brick(brick_name: BrickName) -> BakedBrick {
    use BrickName::*;
    match brick_name {
        SingleTwistLeft => single_twist_left_baked(),
        SingleTwistRight => single_twist_right_baked(),
        OmniSymmetrical => omni_symmetrical_baked(),
        OmniTetrahedral => omni_tetrahedral_baked(),
        TorqueSymmetrical => torque_symmetrical_baked(),
    }
}

fn joint(x: f32, y: f32, z: f32) -> BakedJoint {
    BakedJoint {
        location: Vec3::new(x, y, z),
    }
}

fn push(alpha: usize, omega: usize, strain: f32) -> BakedInterval {
    BakedInterval {
        alpha_index: alpha,
        omega_index: omega,
        strain,
        material_name: "push".to_string(),
    }
}

fn pull(alpha: usize, omega: usize, strain: f32) -> BakedInterval {
    BakedInterval {
        alpha_index: alpha,
        omega_index: omega,
        strain,
        material_name: "pull".to_string(),
    }
}

fn baked_faces(brick_name: BrickName) -> Vec<BrickFace> {
    get_prototype(brick_name).derive_baked_faces(brick_name.face_scaling())
}

fn single_twist_left_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::SingleLeft(SingleParams {
            push_lengths: Vec3::new(3.204, 3.204, 3.204),
            pull_length: 2.0,
        }),
        scale: 0.90909,
        joints: vec![
            joint(-1.10019, -0.96247, 0.00000),
            joint(0.95280, 0.96246, -0.55010),
            joint(0.55011, -0.96245, 0.95280),
            joint(-0.95281, 0.96245, -0.55010),
            joint(0.55011, -0.96246, -0.95280),
            joint(-0.00001, 0.96246, 1.10020),
        ],
        intervals: vec![
            push(0, 1, -0.01509),
            push(2, 3, -0.01509),
            push(4, 5, -0.01509),
            pull(0, 3, 0.10576),
            pull(2, 5, 0.10576),
            pull(4, 1, 0.10576),
        ],
        faces: baked_faces(BrickName::SingleTwistLeft),
    }
}

fn single_twist_right_baked() -> BakedBrick {
    single_twist_left_baked().mirror()
}

fn omni_symmetrical_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::Omni(OmniParams {
            push_lengths: Vec3::new(3.271, 3.271, 3.271),
        }),
        scale: 0.96720,
        joints: vec![
            joint(-1.55675, -0.00000, -0.77838),
            joint(1.55675, -0.00000, -0.77838),
            joint(-1.55675, -0.00000, 0.77838),
            joint(1.55675, -0.00000, 0.77838),
            joint(-0.77838, -1.55675, 0.00000),
            joint(-0.77838, 1.55675, -0.00000),
            joint(0.77838, -1.55675, -0.00000),
            joint(0.77838, 1.55675, -0.00000),
            joint(0.00000, -0.77838, -1.55675),
            joint(-0.00000, -0.77838, 1.55675),
            joint(-0.00000, 0.77839, -1.55675),
            joint(0.00000, 0.77839, 1.55675),
        ],
        intervals: vec![
            push(0, 1, -0.01428),
            push(2, 3, -0.01428),
            push(4, 5, -0.01428),
            push(6, 7, -0.01428),
            push(8, 9, -0.01428),
            push(10, 11, -0.01428),
        ],
        faces: baked_faces(BrickName::OmniSymmetrical),
    }
}

fn omni_tetrahedral_baked() -> BakedBrick {
    // Initial values - will be re-baked with face scaling applied
    BakedBrick {
        params: BrickParams::Omni(OmniParams {
            push_lengths: Vec3::new(3.271, 3.271, 3.271),
        }),
        scale: 1.22593,
        joints: vec![
            joint(-1.72360, -0.95635, -0.98157),
            joint(1.72360, 0.95634, -0.98158),
            joint(-1.72360, 0.95634, 0.98158),
            joint(1.72360, -0.95635, 0.98157),
            joint(-0.98158, -1.72360, -0.95636),
            joint(-0.98158, 1.72360, 0.95636),
            joint(0.98158, -1.72360, 0.95636),
            joint(0.98158, 1.72360, -0.95636),
            joint(-0.95636, -0.98158, -1.72359),
            joint(0.95636, -0.98158, 1.72359),
            joint(0.95635, 0.98159, -1.72360),
            joint(-0.95635, 0.98159, 1.72360),
        ],
        intervals: vec![
            push(0, 1, -0.01524),
            push(2, 3, -0.01524),
            push(4, 5, -0.01524),
            push(6, 7, -0.01524),
            push(8, 9, -0.01524),
            push(10, 11, -0.01524),
        ],
        faces: baked_faces(BrickName::OmniTetrahedral),
    }
}

fn torque_symmetrical_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::Torque(TorqueParams {
            push_lengths: Vec3::new(3.0, 3.0, 6.0),
            pull_length: 1.86,
        }),
        scale: 1.02172,
        joints: vec![
            joint(-1.51340, -0.00000, -2.24376),
            joint(1.51340, -0.00000, -2.24376),
            joint(-1.51547, 0.00000, -0.00000),
            joint(1.51547, 0.00000, -0.00000),
            joint(-1.51340, -0.00000, 2.24376),
            joint(1.51340, -0.00000, 2.24376),
            joint(-1.06390, -1.50813, -1.42055),
            joint(-1.06390, 1.50814, -1.42055),
            joint(-1.06390, -1.50813, 1.42055),
            joint(-1.06390, 1.50814, 1.42055),
            joint(1.06390, -1.50813, -1.42055),
            joint(1.06390, 1.50814, -1.42055),
            joint(1.06390, -1.50813, 1.42055),
            joint(1.06390, 1.50814, 1.42055),
            joint(-0.00000, 0.82050, -3.01384),
            joint(-0.00000, 0.82050, 3.01384),
            joint(-0.00000, -0.82052, -3.01384),
            joint(-0.00000, -0.82052, 3.01384),
        ],
        intervals: vec![
            push(0, 1, -0.01126),
            push(2, 3, -0.00989),
            push(4, 5, -0.01126),
            push(6, 7, -0.01475),
            push(8, 9, -0.01475),
            push(10, 11, -0.01475),
            push(12, 13, -0.01475),
            push(14, 15, -0.01555),
            push(16, 17, -0.01555),
            pull(2, 6, 0.11591),
            pull(2, 7, 0.11592),
            pull(2, 8, 0.11591),
            pull(2, 9, 0.11592),
            pull(3, 10, 0.11591),
            pull(3, 11, 0.11592),
            pull(3, 12, 0.11591),
            pull(3, 13, 0.11592),
        ],
        faces: baked_faces(BrickName::TorqueSymmetrical),
    }
}
