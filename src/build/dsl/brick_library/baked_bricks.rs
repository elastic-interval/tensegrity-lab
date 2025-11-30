use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint};
use crate::build::dsl::brick_dsl::{
    BrickName, BrickParams, OmniParams, SingleParams, TorqueParams,
};
use crate::build::dsl::brick_library::get_prototype;
use cgmath::{Point3, Vector3};

pub fn get_baked_brick(brick_name: BrickName) -> BakedBrick {
    match brick_name {
        BrickName::SingleLeftBrick => single_left_baked(),
        BrickName::SingleRightBrick => single_right_baked(),
        BrickName::OmniBrick => omni_baked(),
        BrickName::OmniTetrahedral => omni_tetrahedral_baked(),
        BrickName::TorqueBrick => torque_baked(),
    }
}

fn joint(x: f32, y: f32, z: f32) -> BakedJoint {
    BakedJoint {
        location: Point3::new(x, y, z),
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

fn single_left_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::SingleLeft(SingleParams {
            push_lengths: Vector3::new(3.204, 3.204, 3.204),
            pull_length: 2.0,
        }),
                scale: 0.91490,
        joints: vec![
            joint(-1.10425, 0.00005, -0.00978),
            joint(0.96160, 1.94868, -0.54322),
            joint(0.54292, 0.00016, 0.96237),
            joint(-0.95096, 1.94765, -0.56279),
            joint(0.56106, 0.00000, -0.95055),
            joint(-0.01173, 1.94844, 1.10355),
        ],
        intervals: vec![
            push(0, 1, -0.01719),
            push(2, 3, -0.01712),
            push(4, 5, -0.01750),
            pull(0, 3, 0.10701),
            pull(2, 5, 0.10683),
            pull(4, 1, 0.10688),
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
                scale: 0.91040,
        joints: vec![
            joint(-0.95706, 0.00000, 0.54951),
            joint(1.10204, 1.93226, 0.00221),
            joint(0.95248, 0.00002, 0.55669),
            joint(-0.54939, 1.93259, -0.95708),
            joint(0.00236, 0.00025, -1.10145),
            joint(-0.55261, 1.93285, 0.95255),
        ],
        intervals: vec![
            push(0, 1, -0.01693),
            push(2, 3, -0.01640),
            push(4, 5, -0.01760),
            pull(0, 5, 0.10461),
            pull(2, 1, 0.10448),
            pull(4, 3, 0.10395),
        ],
        faces: get_prototype(BrickName::SingleRightBrick).derive_baked_faces(),
    }
}

fn omni_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::Omni(OmniParams {
            push_lengths: Vector3::new(3.271, 3.271, 3.271),
        }),
                scale: 0.96720,
        joints: vec![
            joint(-1.55943, 1.55970, -0.77313),
            joint(1.55917, 1.56264, -0.78110),
            joint(-1.55918, 1.56191, 0.78111),
            joint(1.55942, 1.56480, 0.77312),
            joint(-0.78035, 0.00000, -0.02534),
            joint(-0.77589, 3.11936, 0.02920),
            joint(0.77589, 0.00516, -0.02917),
            joint(0.78035, 3.12452, 0.02532),
            joint(-0.00433, 0.78517, -1.58628),
            joint(0.00125, 0.78339, 1.53009),
            joint(-0.00131, 2.34113, -1.53009),
            joint(0.00440, 2.33935, 1.58628),
        ],
        intervals: vec![
            push(0, 1, -0.01579),
            push(2, 3, -0.01579),
            push(4, 5, -0.01540),
            push(6, 7, -0.01540),
            push(8, 9, -0.01656),
            push(10, 11, -0.01656),
        ],
        faces: get_prototype(BrickName::OmniBrick).derive_baked_faces(),
    }
}

fn omni_tetrahedral_baked() -> BakedBrick {
    // Initial values - will be re-baked with face scaling applied
    BakedBrick {
        params: BrickParams::Omni(OmniParams {
            push_lengths: Vector3::new(3.271, 3.271, 3.271),
        }),
                scale: 1.20250,
        joints: vec![
            joint(-1.72721, 2.62150, -0.95229),
            joint(1.71535, 0.83067, -0.97171),
            joint(-1.70069, 0.80427, 0.96631),
            joint(1.71176, 2.63659, 0.95066),
            joint(-0.95610, 0.00000, 0.92101),
            joint(-0.97452, 3.42229, -0.91198),
            joint(0.96071, 0.04124, -0.92077),
            joint(0.97118, 3.44856, 0.91874),
            joint(0.90866, 0.77826, -1.72131),
            joint(-0.90585, 0.75336, 1.70951),
            joint(-0.92379, 2.67785, -1.69904),
            joint(0.92448, 2.69052, 1.69996),
        ],
        intervals: vec![
            push(0, 1, -0.01425),
            push(2, 3, -0.01106),
            push(4, 5, -0.01379),
            push(6, 7, -0.01134),
            push(8, 9, -0.01410),
            push(10, 11, -0.01213),
        ],
        faces: get_prototype(BrickName::OmniTetrahedral).derive_baked_faces(),
    }
}

fn torque_baked() -> BakedBrick {
    BakedBrick {
        params: BrickParams::Torque(TorqueParams {
            push_lengths: Vector3::new(3.0, 3.0, 6.0),
            pull_length: 1.86,
        }),
                scale: 1.01770,
        joints: vec![
            joint(-1.50623, 1.50133, -2.22171),
            joint(1.50623, 1.50133, -2.22171),
            joint(-1.49994, 1.50134, 0.00001),
            joint(1.49994, 1.50134, 0.00001),
            joint(-1.50623, 1.50132, 2.22171),
            joint(1.50623, 1.50132, 2.22171),
            joint(-1.06002, 0.00001, -1.40163),
            joint(-1.05999, 3.00265, -1.40167),
            joint(-1.05996, 0.00000, 1.40164),
            joint(-1.06000, 3.00265, 1.40166),
            joint(1.06002, 0.00001, -1.40163),
            joint(1.05999, 3.00265, -1.40167),
            joint(1.05996, 0.00000, 1.40164),
            joint(1.06000, 3.00265, 1.40166),
            joint(-0.00000, 2.31895, -2.98837),
            joint(-0.00000, 2.31895, 2.98835),
            joint(-0.00000, 0.68367, -2.98836),
            joint(-0.00000, 0.68371, 2.98835),
        ],
        intervals: vec![
            push(0, 1, -0.00880),
            push(2, 3, -0.01298),
            push(4, 5, -0.00880),
            push(6, 7, -0.01208),
            push(8, 9, -0.01207),
            push(10, 11, -0.01208),
            push(12, 13, -0.01207),
            push(14, 15, -0.01848),
            push(16, 17, -0.01848),
            pull(2, 6, 0.11212),
            pull(2, 7, 0.11214),
            pull(2, 8, 0.11213),
            pull(2, 9, 0.11213),
            pull(3, 10, 0.11212),
            pull(3, 11, 0.11214),
            pull(3, 12, 0.11213),
            pull(3, 13, 0.11213),
        ],
        faces: get_prototype(BrickName::TorqueBrick).derive_baked_faces(),
    }
}
