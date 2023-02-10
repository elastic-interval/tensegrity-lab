#![allow(clippy::result_large_err)]

use std::fmt::{Display, Formatter};
use std::fs;

use pest::error::Error;
use pest::iterators::{Pair, Pairs};
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FaceAlias {
    pub name: String,
    pub down: bool,
}

impl FaceAlias {
    pub fn from_pairs(inner: &mut Pairs<Rule>) -> Vec<FaceAlias> {
        inner
            .map(|pair| {
                let mut inner = pair.into_inner();
                let name = parse_atom(inner.next().unwrap());
                let down = inner.next().is_some();
                FaceAlias { name, down }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SurfaceCharacterSpec {
    Frozen,
    Bouncy,
    Sticky,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    pub fn standard() -> Self {
        let source = fs::read_to_string("src/build/tenscript/library.scm").unwrap();
        match Self::from_tenscript(&source) {
            Ok(library) => library,
            Err(ParseError::Pest(error)) => panic!("pest parse error: \n{error}"),
            Err(e) => panic!("{e:?}")
        }
    }

    pub fn from_tenscript(source: &str) -> Result<Self, ParseError> {
        let pair = TenscriptParser::parse(Rule::library, source)
            .map_err(ParseError::Pest)?
            .next()
            .expect("no (library ..)");
        Self::from_pair(pair)
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
    assert_eq!(pair.as_rule(), Rule::atom);
    let string = pair.as_str();
    string
        .strip_prefix(':')
        .unwrap_or(string)
        .to_string()
}

pub fn into_atom(name: String) -> String {
    if name.chars().next().expect("empty string").is_uppercase() {
        name
    } else {
        format!(":{name}")
    }
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::Library;

    #[test]
    fn parse_test() {
        let plans = Library::standard();
        println!("{plans:?}")
    }
}