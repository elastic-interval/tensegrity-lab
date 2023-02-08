#![allow(clippy::result_large_err)]

use std::fmt::{Display, Formatter};

pub use fabric_plan::FabricPlan;

use crate::build::tenscript::build_phase::BuildPhase;
use crate::fabric::UniqueId;

pub mod fabric_plan;
pub mod plan_runner;
mod shape_phase;
mod build_phase;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FaceName(pub usize);

impl Display for FaceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("F{}", self.0))
    }
}

impl TryFrom<&str> for FaceName {
    type Error = ();

    fn try_from(face_name: &str) -> Result<Self, Self::Error> {
        if !face_name.starts_with('F') {
            return Err(());
        }
        face_name[1..].parse().map(FaceName).map_err(|_| ())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SurfaceCharacterSpec {
    Frozen,
    Bouncy,
    Sticky,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spin {
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

#[derive(Debug, Default, Clone)]
pub struct FaceMark {
    face_id: UniqueId,
    mark_name: String,
}
