use crate::build::dsl::brick::{BakedBrick, BakedInterval, BakedJoint, BrickFace};
use crate::build::dsl::brick_dsl::{
    BrickName, BrickParams, OmniParams, SingleParams, TorqueParams,
};
use crate::build::dsl::brick_library::get_prototype;
use cgmath::{Point3, Vector3};

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

fn baked_faces(brick_name: BrickName) -> Vec<BrickFace> {
    get_prototype(brick_name).derive_baked_faces(brick_name.face_scaling())
}

fn single_twist_left_baked() -> BakedBrick {
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
        faces: baked_faces(BrickName::SingleTwistLeft),
    }
}

fn single_twist_right_baked() -> BakedBrick {
    single_twist_left_baked().mirror()
}

fn omni_symmetrical_baked() -> BakedBrick {
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
        faces: baked_faces(BrickName::OmniSymmetrical),
    }
}

fn omni_tetrahedral_baked() -> BakedBrick {
    // Initial values - will be re-baked with face scaling applied
    BakedBrick {
        params: BrickParams::Omni(OmniParams {
            push_lengths: Vector3::new(3.271, 3.271, 3.271),
        }),
        scale: 1.20312,
        joints: vec![
            joint(-1.71376, 0.79510, -0.96644),
            joint(1.71376, 2.60387, -0.96647),
            joint(-1.71376, 2.60383, 0.96649),
            joint(1.71378, 0.79513, 0.96644),
            joint(-0.96339, 0.00000, -0.92476),
            joint(-0.96340, 3.39893, 0.92478),
            joint(0.96342, 0.00001, 0.92473),
            joint(0.96337, 3.39896, -0.92477),
            joint(-0.91172, 0.74432, -1.71431),
            joint(0.91173, 0.74433, 1.71428),
            joint(0.91168, 2.65465, -1.71426),
            joint(-0.91171, 2.65462, 1.71430),
        ],
        intervals: vec![
            push(0, 1, -0.01099),
            push(2, 3, -0.01100),
            push(4, 5, -0.01250),
            push(6, 7, -0.01250),
            push(8, 9, -0.01405),
            push(10, 11, -0.01406),
        ],
        faces: baked_faces(BrickName::OmniTetrahedral),
    }
}

fn torque_symmetrical_baked() -> BakedBrick {
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
        faces: baked_faces(BrickName::TorqueSymmetrical),
    }
}
