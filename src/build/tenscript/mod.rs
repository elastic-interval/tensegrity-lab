use std::fmt::{Display, Formatter};

use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::FaceName::{*};
use crate::build::tenscript::shape_phase::ShapePhase;
use crate::fabric::UniqueId;

mod parser;
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

#[derive(Debug, Default, Clone)]
pub struct FabricPlan {
    pub surface: Option<SurfaceCharacterSpec>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

pub fn bootstrap_fabric_plans() -> Vec<(String, String)> {
    include_str!("bootstrap.scm")
        .split(";;;")
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            let line_end = chunk.find('\n').unwrap_or_else(|| {
                panic!("bootstrap.scm not structured properly");
            });
            (chunk[0..line_end].to_string(), chunk[(line_end + 1)..].to_string())
        })
        .collect()
}

pub fn fabric_plan(plan_name: &str) -> FabricPlan {
    let plans = bootstrap_fabric_plans();
    let Some((_, code)) = plans.iter().find(|&(name, _)| *name == plan_name) else {
        panic!("{plan_name} not found");
    };
    match FabricPlan::from_tenscript(code.as_str()) {
        Ok(plan) => plan,
        Err(error) => panic!("error parsing fabric plan: {error}")
    }
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::{bootstrap_fabric_plans, FabricPlan};

    #[test]
    fn parse() {
        let map = bootstrap_fabric_plans();
        for (name, code) in map.iter() {
            match FabricPlan::from_tenscript(code) {
                Ok(_) => println!("[{name}] Good plan!"),
                Err(error) => panic!("[{name}] Error: {error:?}"),
            }
        }
    }
}
