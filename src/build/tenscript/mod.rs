#![allow(clippy::result_large_err)]

use std::fmt::{Display, Formatter};

use pest::error::Error;
use pest::Parser;
use pest_derive::Parser;

pub use fabric_plan::FabricPlan;

use crate::build::tenscript::build_phase::BuildPhase;
use crate::fabric::UniqueId;

pub mod fabric_plan;
pub mod plan_runner;
mod shape_phase;
mod build_phase;

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct TenscriptParser;

#[derive(Debug)]
pub enum ParseError {
    Pest(Error<Rule>),
    Invalid(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Pest(error) => write!(f, "parse error: {error}"),
            ParseError::Invalid(warning) => write!(f, "warning: {warning}"),
        }
    }
}

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

mod brick {}

pub struct Collection {
    fabric_plans: Vec<FabricPlan>,
    bricks: Vec<brick::Definition>,
}