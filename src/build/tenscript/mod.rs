#![allow(clippy::result_large_err)]

use std::fmt::{Display, Formatter};

pub use fabric_plan::{fabric_plans_from_bootstrap, FabricPlan};

use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::FaceName::{*};
use crate::fabric::UniqueId;

pub mod fabric_plan;
pub mod plan_runner;
mod shape_phase;
mod build_phase;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FaceName { Apos, Bpos, Cpos, Dpos, Aneg, Bneg, Cneg, Dneg }

impl Display for FaceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Apos => "A+",
            Bpos => "B+",
            Cpos => "C+",
            Dpos => "D+",
            Aneg => "A-",
            Bneg => "B-",
            Cneg => "C-",
            Dneg => "D-",
        })
    }
}

impl TryFrom<&str> for FaceName {
    type Error = ();

    fn try_from(face_name: &str) -> Result<Self, Self::Error> {
        Ok(match face_name {
            "A+" => Apos,
            "B+" => Bpos,
            "C+" => Cpos,
            "D+" => Dpos,
            "A-" => Aneg,
            "B-" => Bneg,
            "C-" => Cneg,
            "D-" => Dneg,
            _ => return Err(())
        })
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
