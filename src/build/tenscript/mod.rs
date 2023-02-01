use std::fmt::{Display, Formatter};

use crate::build::tenscript::FaceName::{*};

mod parser;

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

#[derive(Debug, Clone)]
pub enum BuildNode {
    Face {
        face_name: FaceName,
        node: Box<BuildNode>,
    },
    Grow {
        forward: String,
        scale_factor: f32,
        post_growth_node: Option<Box<BuildNode>>,
    },
    Mark {
        mark_name: String,
    },
    Branch {
        face_nodes: Vec<BuildNode>,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SeedType {
    #[default]
    Left,
    Right,
    LeftRight,
    RightLeft,
}

impl Seed {
    pub fn spin(&self) -> Spin {
        match self.seed_type {
            SeedType::Left | SeedType::LeftRight => Spin::Left,
            SeedType::Right | SeedType::RightLeft => Spin::Right,
        }
    }

    pub fn needs_double(&self) -> bool {
        match self.seed_type {
            SeedType::Left | SeedType::Right => false,
            SeedType::LeftRight | SeedType::RightLeft => true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Seed {
    pub seed_type: SeedType,
    pub down_faces: Vec<FaceName>,
}

#[derive(Debug, Clone, Default)]
pub struct BuildPhase {
    pub seed: Seed,
    pub root: Option<BuildNode>,
}

#[derive(Debug, Clone)]
pub enum ShapeOperation {
    Countdown {
        count: usize,
        operations: Vec<ShapeOperation>,
    },
    Join { mark_name: String },
    Distance { mark_name: String, distance_factor: f32 },
    RemoveShapers { mark_names: Vec<String> },
    Vulcanize,
    ReplaceFaces,
}

#[derive(Debug, Clone, Default)]
pub struct ShapePhase {
    pub operations: Vec<ShapeOperation>,
}

#[derive(Debug, Clone, Default)]
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