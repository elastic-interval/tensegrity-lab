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
pub mod converge_phase;
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

impl Fabric {
    pub fn expect_face(&self, face_id: UniqueId) -> &crate::fabric::face::Face {
        self.faces.get(&face_id).expect(&format!("Expected face {:?} in fabric with {} faces", &face_id, self.faces.len()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaceAlias {
    pub brick_role: BrickRole,
    pub face_name: FaceName,
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
    pub fn opposite(self) -> Spin {
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
