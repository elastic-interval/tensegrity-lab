#![allow(clippy::result_large_err)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::io::Error as IOError;
use std::ops::Add;

use pest::error::Error as PestError;
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;

pub use fabric_plan::FabricPlan;

use crate::build::tenscript::build_phase::BuildPhase;
use crate::fabric::face::Face;
use crate::fabric::{Fabric, UniqueId};

pub mod animate_phase;
pub mod brick;
pub mod brick_library;
pub mod build_phase;
pub mod converge_phase;
pub mod fabric_library;
pub mod fabric_plan;
pub mod plan_context;
pub mod plan_runner;
pub mod pretense_phase;
pub mod pretenser;
pub mod shape_phase;

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
pub struct TenscriptParser;

#[derive(Debug)]
pub enum TenscriptError {
    FileReadError(IOError),
    PestError(PestError<Rule>),
    FormatError(String),
    InvalidError(String),
    FaceAliasError(String),
    MarkError(String),
}

pub fn parse_float(string: &str, spot: &str) -> Result<f32, TenscriptError> {
    string
        .parse()
        .map_err(|_| TenscriptError::FormatError(format!("[{spot}]: Not a float: '{string}'")))
}

pub fn parse_usize(string: &str, spot: &str) -> Result<usize, TenscriptError> {
    string
        .parse()
        .map_err(|_| TenscriptError::FormatError(format!("[{spot}]: Not an int: '{string}'")))
}

pub fn parse_float_inside(pair: Pair<Rule>, spot: &str) -> Result<f32, TenscriptError> {
    parse_float(pair.into_inner().next().unwrap().as_str(), spot)
        .map_err(|error| TenscriptError::FormatError(format!("Not a float pair: [{error}]")))
}

pub fn parse_usize_inside(pair: Pair<Rule>, spot: &str) -> Result<usize, TenscriptError> {
    parse_usize(pair.into_inner().next().unwrap().as_str(), spot)
        .map_err(|error| TenscriptError::FormatError(format!("Not a usize pair: [{error}]")))
}

/// Extension trait for Pair to simplify parsing
pub trait PairExt {
    /// Parse the pair's inner content as a float
    fn parse_float_inner(&self, context: &str) -> Result<f32, TenscriptError>;

    /// Parse the pair's inner content as a usize
    fn parse_usize_inner(&self, context: &str) -> Result<usize, TenscriptError>;

    /// Parse the pair's string directly as a float
    fn parse_float_str(&self, context: &str) -> Result<f32, TenscriptError>;

    /// Parse the pair's string directly as a usize
    fn parse_usize_str(&self, context: &str) -> Result<usize, TenscriptError>;

    /// Get the atom string (strips leading ':' if present)
    fn as_atom(&self) -> String;
}

impl PairExt for Pair<'_, Rule> {
    fn parse_float_inner(&self, context: &str) -> Result<f32, TenscriptError> {
        parse_float_inside(self.clone(), context)
    }

    fn parse_usize_inner(&self, context: &str) -> Result<usize, TenscriptError> {
        parse_usize_inside(self.clone(), context)
    }

    fn parse_float_str(&self, context: &str) -> Result<f32, TenscriptError> {
        parse_float(self.as_str(), context)
    }

    fn parse_usize_str(&self, context: &str) -> Result<usize, TenscriptError> {
        parse_usize(self.as_str(), context)
    }

    fn as_atom(&self) -> String {
        let s = self.as_str();
        if s.starts_with(':') {
            s[1..].to_string()
        } else {
            s.to_string()
        }
    }
}

/// Extension trait for Pairs iterator to simplify parsing
pub trait PairsExt<'i> {
    /// Get the next pair and parse it as a float
    fn next_float(&mut self, context: &str) -> Result<f32, TenscriptError>;

    /// Get the next pair and parse it as a usize
    fn next_usize(&mut self, context: &str) -> Result<usize, TenscriptError>;

    /// Get the next pair and parse its inner content as a float
    fn next_float_inner(&mut self, context: &str) -> Result<f32, TenscriptError>;

    /// Get the next pair and parse its inner content as a usize
    fn next_usize_inner(&mut self, context: &str) -> Result<usize, TenscriptError>;

    /// Get the next pair as an atom string
    fn next_atom(&mut self) -> String;
}

impl<'i> PairsExt<'i> for Pairs<'i, Rule> {
    fn next_float(&mut self, context: &str) -> Result<f32, TenscriptError> {
        self.next()
            .ok_or_else(|| TenscriptError::FormatError(format!("[{context}]: Missing float")))
            .and_then(|p| p.parse_float_str(context))
    }

    fn next_usize(&mut self, context: &str) -> Result<usize, TenscriptError> {
        self.next()
            .ok_or_else(|| TenscriptError::FormatError(format!("[{context}]: Missing usize")))
            .and_then(|p| p.parse_usize_str(context))
    }

    fn next_float_inner(&mut self, context: &str) -> Result<f32, TenscriptError> {
        self.next()
            .ok_or_else(|| TenscriptError::FormatError(format!("[{context}]: Missing float")))
            .and_then(|p| p.parse_float_inner(context))
    }

    fn next_usize_inner(&mut self, context: &str) -> Result<usize, TenscriptError> {
        self.next()
            .ok_or_else(|| TenscriptError::FormatError(format!("[{context}]: Missing usize")))
            .and_then(|p| p.parse_usize_inner(context))
    }

    fn next_atom(&mut self) -> String {
        self.next().map(|p| p.as_atom()).unwrap_or_default()
    }
}

impl Fabric {
    pub fn expect_face(&self, face_id: UniqueId) -> Result<&Face, TenscriptError> {
        self.faces
            .get(&face_id)
            .ok_or(TenscriptError::InvalidError("Face missing".to_string()))
    }
}

impl Display for TenscriptError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TenscriptError::FileReadError(error) => write!(f, "TenscriptError::FileRead: {error}"),
            TenscriptError::PestError(error) => write!(f, "TenscriptError::Pest: {error}"),
            TenscriptError::FormatError(error) => write!(f, "TenscriptError::Format: {error}"),
            TenscriptError::InvalidError(warning) => {
                write!(f, "TenscriptError::Invalid: {warning}")
            }
            TenscriptError::FaceAliasError(name) => write!(f, "TenscriptError::FaceAlias: {name}"),
            TenscriptError::MarkError(name) => write!(f, "TenscriptError::Mark: {name}"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
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
        face_list.iter().find_map(|&face_id| {
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

    pub fn with_base(&self) -> Self {
        self.with(":base")
    }

    pub fn is_seed(&self, which: Option<usize>) -> bool {
        match which {
            Some(which) => self.has(format!(":seed-{}", which).as_str()),
            None => self.has(":seed"),
        }
    }

    pub fn with_seed(&self, which: Option<usize>) -> Self {
        match which {
            Some(which) => self.with(format!(":seed-{}", which).as_str()),
            None => self.with(":seed"),
        }
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
            return Some(match part.as_str() {
                ":left" => Spin::Left,
                ":right" => Spin::Right,
                _ => continue,
            });
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

    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = Pair<'a, Rule>>) -> Vec<FaceAlias> {
        pairs.into_iter().map(Self::from_pair).collect()
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
