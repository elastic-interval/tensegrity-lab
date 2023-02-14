#![allow(clippy::result_large_err)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::ops::Add;

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

#[derive(Clone, Debug, Default)]
pub struct FaceAlias(pub HashSet<String>);

impl Add<&FaceAlias> for FaceAlias {
    type Output = FaceAlias;

    fn add(self, other: &Self) -> Self::Output {
        let mut combo = self.0;
        combo.extend(other.0.clone());
        Self(combo)
    }
}

impl FaceAlias {
    pub fn matches(&self, haystack: &FaceAlias) -> bool {
        self.0.is_subset(&haystack.0)
    }

    pub fn into_vec(self) -> Vec<String> {
        let mut sorted: Vec<_> = self.0.into_iter().collect();
        sorted.sort();
        sorted
    }

    pub fn single(name: &str) -> Self {
        Self(HashSet::from([name.to_string()]))
    }

    pub fn is_base(&self) -> bool {
        self.check_for(":base")
    }

    pub fn is_seed(&self) -> bool {
        self.check_for(":seed")
    }

    pub fn with_base(&self) -> Self {
        self.augmented(":base")
    }

    pub fn with_seed(&self) -> Self {
        self.augmented(":seed")
    }

    fn check_for(&self, sought_part: &str) -> bool {
        self.0.iter().any(|part| part == sought_part)
    }

    fn augmented(&self, extra: &str) -> Self {
        let mut set = self.0.clone();
        set.insert(extra.to_string());
        Self(set)
    }

    pub fn spin(&self) -> Option<Spin> {
        for part in &self.0 {
            return Some(
                match part.as_str() {
                    ":left" => Spin::Left,
                    ":right" => Spin::Right,
                    _ => continue
                }
            );
        }
        None
    }
}

impl Display for FaceAlias {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let alias = self.clone().into_vec().join(" ");
        write!(f, "{alias}")
    }
}

impl FaceAlias {
    pub fn from_pair(pair: Pair<Rule>) -> FaceAlias {
        let parts = pair.into_inner().map(parse_atom).collect();
        FaceAlias(parts)
    }

    pub fn from_pairs(pairs: impl IntoIterator<Item=Pair<Rule>>) -> Vec<FaceAlias> {
        pairs
            .into_iter()
            .map(Self::from_pair)
            .collect()
    }
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

    pub fn into_alias(self) -> FaceAlias {
        FaceAlias::single(match self {
            Spin::Left => ":left",
            Spin::Right => ":right",
        })
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
    pair.as_str().to_string()
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