use std::collections::HashMap;
use std::fmt::{Display, Formatter};

pub use parser::parse;

use crate::build::tenscript::FaceName::{*};

mod error;
mod expression;
mod parser;
mod scanner;

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
        node: Box<TenscriptNode>,
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

#[derive(Debug, Clone, Copy, Default)]
pub enum Seed {
    #[default]
    Left,
    Right,
    LeftRight,
    RightLeft,
}

impl Seed {
    pub fn spin(&self) -> Spin {
        match self {
            Seed::Left | Seed::LeftRight => Spin::Left,
            Seed::Right | Seed::RightLeft => Spin::Right,
        }
    }

    pub fn needs_double(&self) -> bool {
        match self {
            Seed::Left | Seed::Right => false,
            Seed::LeftRight | Seed::RightLeft => true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuildPhase {
    pub seed: Seed,
    pub root: Option<TenscriptNode>,
}

#[derive(Debug, Clone)]
pub enum ShaperSpec {
    Join { mark_name: String },
    Distance { mark_name: String, distance_factor: f32 },
}

#[derive(Debug, Clone, Default)]
pub struct ShapePhase {
    pub shaper_specs: Vec<ShaperSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct FabricPlan {
    pub surface: Option<SurfaceCharacterSpec>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

fn bootstrap() -> HashMap<&'static str, &'static str> {
    include_str!("bootstrap.scm")
        .split(";;;")
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            let line_end = chunk.find('\n').unwrap_or_else(|| {
                panic!("bootstrap.scm not structured properly");
            });
            (&chunk[0..line_end], &chunk[(line_end + 1)..])
        })
        .collect()
}

pub fn fabric_plan(plan_name: &str) -> FabricPlan {
    let map = bootstrap();
    let code = map.get(plan_name).unwrap_or_else(|| {
        panic!("{plan_name} not found")
    });
    parse(code).unwrap()
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::{bootstrap, parser};

    #[test]
    fn parse() {
        let map = bootstrap();
        for (name, code) in map.into_iter() {
            match parser::parse(code) {
                Ok(_) => println!("[{name}] Good plan!"),
                Err(error) => println!("[{name}] Error: {error:?}"),
            }
        }
    }
}