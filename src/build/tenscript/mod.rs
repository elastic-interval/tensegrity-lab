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
use crate::fabric::{Fabric, UniqueId};

pub mod fabric_plan;
pub mod plan_runner;
mod shape_phase;
mod build_phase;

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
pub struct TenscriptParser;

#[derive(Debug)]
pub enum TenscriptError {
    Pest(Error<Rule>),
    Format(String),
    Invalid(String),
}

impl TenscriptError {
    pub fn parse_float(string: &str, spot: &str) -> Result<f32, Self> {
        string.parse().map_err(|_| TenscriptError::Format(format!("[{spot}]: Not a float: '{string}'")))
    }

    pub fn parse_usize(string: &str, spot: &str) -> Result<usize, Self> {
        string.parse().map_err(|_| TenscriptError::Format(format!("[{spot}]: Not an int: '{string}'")))
    }

    pub fn parse_float_inside(pair: Pair<Rule>, spot: &str) -> Result<f32, TenscriptError> {
        Self::parse_float(pair.into_inner().next().unwrap().as_str(), spot)
            .map_err(|error| TenscriptError::Format(format!("Not a float pair: [{error}]")))
    }
}

impl Display for TenscriptError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TenscriptError::Pest(error) => write!(f, "parse error: {error}"),
            TenscriptError::Format(error) => write!(f, "format: {error}"),
            TenscriptError::Invalid(warning) => write!(f, "warning: {warning}"),
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

    pub fn find_face_in(&self, face_list: &[UniqueId], fabric: &Fabric) -> Option<UniqueId> {
        face_list
            .iter()
            .find_map(|&face_id| {
                let face = fabric.faces.get(&face_id)?;
                self.matches(face.alias()).then_some(face_id)
            })
    }

    pub fn single(name: &str) -> Self {
        Self(HashSet::from([name.to_string()]))
    }

    pub fn is_base(&self) -> bool {
        self.has(":base")
    }

    pub fn is_seed(&self) -> bool {
        self.has(":seed")
    }

    pub fn with_base(&self) -> Self {
        self.with(":base")
    }

    pub fn with_seed(&self) -> Self {
        self.with(":seed")
    }

    fn has(&self, sought_part: &str) -> bool {
        self.0.iter().any(|part| part == sought_part)
    }

    fn with(&self, extra: &str) -> Self {
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
            Err(TenscriptError::Pest(_error)) => panic!("pest parse error: \n{_error}"),
            Err(_error) => panic!("{_error:?}")
        }
    }

    pub fn from_tenscript(source: &str) -> Result<Self, TenscriptError> {
        let pair = TenscriptParser::parse(Rule::library, source)
            .map_err(TenscriptError::Pest)?
            .next()
            .expect("no (library ..)");
        Self::from_pair(pair)
    }

    fn from_pair(pair: Pair<Rule>) -> Result<Self, TenscriptError> {
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

pub fn parse_name(pair: Pair<Rule>) -> Vec<String> {
    assert_eq!(pair.as_rule(), Rule::name);
    pair
        .into_inner()
        .map(|pair| pair.as_str())
        .map(|quoted| quoted[1..quoted.len() - 1].to_string())
        .collect()
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