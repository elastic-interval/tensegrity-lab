#![allow(clippy::result_large_err)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::io::Error as IOError;
use std::ops::Add;

use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest_derive::Parser;

pub use fabric_plan::FabricPlan;

use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::pretense_phase::MuscleMovement;
use crate::fabric::face::Face;
use crate::fabric::interval::Span;
use crate::fabric::interval::Span::Muscle;
use crate::fabric::{Fabric, UniqueId};

pub mod brick;
pub mod brick_library;
pub mod build_phase;
pub mod fabric_library;
pub mod fabric_plan;
pub mod plan_runner;
pub mod pretense_phase;
pub mod pretenser;
pub mod shape_phase;

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
pub struct TenscriptParser;

#[derive(Debug)]
pub enum TenscriptError {
    FileRead(IOError),
    Pest(PestError<Rule>),
    Format(String),
    Invalid(String),
    FaceAlias(String),
    Mark(String),
}

impl TenscriptError {
    pub fn parse_float(string: &str, spot: &str) -> Result<f32, Self> {
        string
            .parse()
            .map_err(|_| TenscriptError::Format(format!("[{spot}]: Not a float: '{string}'")))
    }

    pub fn parse_usize(string: &str, spot: &str) -> Result<usize, Self> {
        string
            .parse()
            .map_err(|_| TenscriptError::Format(format!("[{spot}]: Not an int: '{string}'")))
    }

    pub fn parse_float_inside(pair: Pair<Rule>, spot: &str) -> Result<f32, Self> {
        Self::parse_float(pair.into_inner().next().unwrap().as_str(), spot)
            .map_err(|error| TenscriptError::Format(format!("Not a float pair: [{error}]")))
    }
}

impl Fabric {
    pub fn expect_face(&self, face_id: UniqueId) -> Result<&Face, TenscriptError> {
        self.faces
            .get(&face_id)
            .ok_or(TenscriptError::Invalid("Face missing".to_string()))
    }

    pub fn activate_muscles(&mut self, muscle_movement: &MuscleMovement) {
        self.muscle_nuance = 0.5;
        let north_material = self.material(":north".to_string());
        let south_material = self.material(":south".to_string());
        for interval in self.intervals.values_mut() {
            let Span::Fixed { length } = interval.span else {
                continue;
            };
            let contracted = length * muscle_movement.contraction;
            let reversed =  muscle_movement.reversed_groups.contains(&interval.group);
            if interval.material == north_material {
                interval.span = Muscle { length, contracted, reverse: reversed };
            }
            if interval.material == south_material {
                interval.span = Muscle { length, contracted, reverse: !reversed };
            }
        }
    }
}

impl Display for TenscriptError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TenscriptError::FileRead(error) => write!(f, "file read error: {error}"),
            TenscriptError::Pest(error) => write!(f, "parse error: {error}"),
            TenscriptError::Format(error) => write!(f, "format: {error}"),
            TenscriptError::Invalid(warning) => write!(f, "warning: {warning}"),
            TenscriptError::FaceAlias(name) => write!(f, "alias: {name}"),
            TenscriptError::Mark(name) => write!(f, "mark: {name}"),
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

    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item=Pair<'a, Rule>>) -> Vec<FaceAlias> {
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
