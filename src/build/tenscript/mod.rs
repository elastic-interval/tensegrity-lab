use std::fmt::{Display, Formatter};

pub use parser::parse;
use crate::build::tenscript::bootstrap::BOOTSTRAP;

use crate::build::tenscript::FaceName::{*};

mod error;
mod expression;
mod parser;
mod scanner;
mod bootstrap;

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

#[derive(Debug, Clone)]
pub enum TenscriptNode {
    Face {
        face_name: FaceName,
        node: Box<TenscriptNode>
    },
    Grow {
        forward: String,
        scale_factor: f32,
        post_growth_node: Option<Box<TenscriptNode>>,
    },
    Mark {
        mark_name: String,
    },
    Branch {
        face_nodes: Vec<TenscriptNode>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct BuildPhase {
    pub seed: Option<Spin>,
    pub root: Option<TenscriptNode>,
}

#[derive(Debug, Clone, Default)]
pub struct Space {
    pub mark_name: String,
    pub scale_factor: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ShapePhase {
    pub join: Vec<String>,
    pub space: Vec<Space>,
}

#[derive(Debug, Clone, Default)]
pub struct FabricPlan {
    pub surface: Option<SurfaceCharacterSpec>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

pub fn fabric_plan(plan_name: &str) -> FabricPlan {
    let (_, code) = BOOTSTRAP.iter().find(|(name, _)| *name == plan_name).unwrap();
    parse(code).unwrap()
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::bootstrap::BOOTSTRAP;
    use crate::build::tenscript::parser;

    #[test]
    fn parse() {
        for (name, code) in BOOTSTRAP {
            let plan = parser::parse(code);
            let parsed = plan.err();
            println!("{name}: {parsed:?}");
        }
    }
}