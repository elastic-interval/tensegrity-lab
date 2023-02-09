#![allow(clippy::result_large_err)]

use std::cell::LazyCell;
use std::fmt::{Display, Formatter};

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use brick::BrickDefinition;
pub use fabric_plan::FabricPlan;

use crate::build::brick;
use crate::build::tenscript::build_phase::BuildPhase;
use crate::fabric::UniqueId;

pub mod fabric_plan;
pub mod plan_runner;
mod shape_phase;
mod build_phase;

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
pub struct TenscriptParser;

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

#[derive(Clone, Default, Debug)]
pub struct Library {
    pub(crate) fabrics: Vec<FabricPlan>,
    pub(crate) bricks: Vec<BrickDefinition>,
}

impl Library {
    pub fn bootstrap() -> Self {
        let bootstrap: LazyCell<Self> = LazyCell::new(||
            Self::from_file(include_str!("bootstrap.scm")).unwrap()
        );
        bootstrap.clone()
    }

    pub fn from_file(source: &str) -> Result<Self, ParseError> {
        let pair = TenscriptParser::parse(Rule::library, source)
            .map_err(ParseError::Pest)?
            .next()
            .expect("no (library ..)");
        Self::from_pair(pair)
    }

    pub fn from_tenscript(source: &str) -> Result<Self, ParseError> {
        let fabric_plan_pair = TenscriptParser::parse(Rule::fabric_plan, source)
            .map_err(ParseError::Pest)?
            .next()
            .unwrap();
        Self::from_pair(fabric_plan_pair)
    }

    fn from_pair(pair: Pair<Rule>) -> Result<Self, ParseError> {
        let mut library = Self::default();
        for definition in pair.into_inner() {
            match definition.as_rule() {
                Rule::fabric_plan => {
                    let fabric_plan = FabricPlan::from_pair(definition)?;
                    library.fabrics.push(fabric_plan);
                }
                Rule::brick_definition => {
                    let brick = BrickDefinition::from_pair(definition)?;
                    library.bricks.push(brick);
                }
                _ => unreachable!()
            }
        }
        Ok(library)
    }
}

pub fn parse_name(pair: Pair<Rule>) -> String {
    assert_eq!(pair.as_rule(), Rule::name);
    let name_string = pair.into_inner().next().unwrap().as_str();
    name_string[1..name_string.len() - 1].to_string()
}

pub fn parse_atom(pair: Pair<Rule>) -> String {
    let string = pair.as_str();
    string
        .strip_prefix(':')
        .unwrap_or(string)
        .to_string()
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::Library;

    #[test]
    fn parse_test() {
        let plans = Library::bootstrap();
        println!("{plans:?}")
    }
}