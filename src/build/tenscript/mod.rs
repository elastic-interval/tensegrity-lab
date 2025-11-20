#![allow(clippy::result_large_err)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::io::Error as IOError;
use std::ops::Add;

pub use fabric_plan::FabricPlan;

use crate::fabric::{Fabric, UniqueId};

pub mod animate_phase;
pub mod brick;
pub mod brick_library;
pub mod build_phase;
pub mod converge_phase;
pub mod fabric_library;
pub mod fabric_plan;
pub mod fabric_plan_executor;
pub mod plan_context;
pub mod plan_runner;
pub mod pretense_phase;
pub mod pretenser;
pub mod shape_phase;

#[derive(Debug)]
pub enum TenscriptError {
    FileReadError(IOError),
    FormatError(String),
    InvalidError(String),
    FaceAliasError(String),
    MarkError(String),
}

impl Fabric {
    pub fn expect_face(&self, face_id: UniqueId) -> Result<&crate::fabric::face::Face, TenscriptError> {
        self.faces
            .get(&face_id)
            .ok_or(TenscriptError::InvalidError("Face missing".to_string()))
    }
}

impl Display for TenscriptError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TenscriptError::FileReadError(error) => write!(f, "TenscriptError::FileRead: {error}"),
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

pub fn into_atom(name: String) -> String {
    if name.chars().next().expect("empty string").is_uppercase() {
        name
    } else {
        format!(":{name}")
    }
}
