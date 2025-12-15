#![allow(clippy::result_large_err)]

pub use fabric_plan::FabricPlan;
use std::fmt::{Display, Formatter};
use strum::Display;

use crate::build::dsl::brick_dsl::{BrickRole, FaceName, MarkName};
use crate::fabric::{Fabric, UniqueId};

pub mod animate_phase;
pub mod brick;
pub mod brick_dsl;
pub mod brick_library;
pub mod build_phase;
pub mod fall_phase;
pub mod settle_phase;
pub mod fabric_dsl;
pub mod fabric_library;
pub mod fabric_plan;
pub mod fabric_plan_executor;
pub mod plan_context;
pub mod plan_runner;
pub mod pretense_phase;
pub mod pretenser;
pub mod shape_phase;
pub mod single_interval_drop_test;
pub mod bake_brick_test;

impl Fabric {
    pub fn expect_face(&self, face_id: UniqueId) -> &crate::fabric::face::Face {
        self.faces.get(&face_id).expect(&format!("Expected face {:?} in fabric with {} faces", &face_id, self.faces.len()))
    }
}

/// Named scaling schemes for face size variations
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScaleMode {
    None,        // Use default 1.0 for all faces
    Tetrahedral, // Tetrahedral variant (4 large + 4 small faces)
}

impl ScaleMode {
    /// The ratio between large and small faces (large/small = ratio)
    pub fn ratio(self) -> f32 {
        match self {
            ScaleMode::None => 1.0,
            ScaleMode::Tetrahedral => 3.0,
        }
    }

    /// Scale override for large faces: (self, √ratio)
    pub fn small(self) -> (ScaleMode, f32) {
        (self, 1.0 / self.ratio().sqrt())
    }

    /// Scale override for small faces: (self, 1/√ratio)
    pub fn large(self) -> (ScaleMode, f32) {
        (self, self.ratio().sqrt())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaceAlias {
    pub brick_role: BrickRole,
    pub face_name: FaceName,
}

impl FaceAlias {
    /// Mirror this alias (swap OnSpinLeft↔OnSpinRight and spin in Attach)
    pub fn mirror(&self) -> FaceAlias {
        FaceAlias {
            brick_role: self.brick_role.mirror(),
            face_name: self.face_name.mirror(),
        }
    }
}

impl Display for FaceAlias {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.brick_role, self.face_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Display)]
pub enum Spin {
    #[default]
    Left,
    Right,
}

impl Spin {
    pub fn mirror(self) -> Spin {
        match self {
            Spin::Left => Spin::Right,
            Spin::Right => Spin::Left,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FaceMark {
    face_id: UniqueId,
    mark_name: MarkName,
}

pub fn into_atom(name: String) -> String {
    if name.chars().next().expect("empty string").is_uppercase() {
        name
    } else {
        format!(":{name}")
    }
}
