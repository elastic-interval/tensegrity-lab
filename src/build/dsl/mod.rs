#![allow(clippy::result_large_err)]

use std::fmt::{Display, Formatter};
use std::ops::Add;

pub use fabric_plan::FabricPlan;

use crate::fabric::{Fabric, UniqueId};
use crate::build::dsl::brick_dsl::{BrickName, FaceName, FaceContext, MarkName, SingleFace};
use crate::build::dsl::brick_dsl::FaceName::Single;

pub mod animate_phase;
pub mod brick;
pub mod brick_builders;
pub mod brick_dsl;
pub mod brick_library;
pub mod build_phase;
pub mod converge_phase;
pub mod fabric_builders;
pub mod fabric_dsl;
pub mod fabric_library;
pub mod fabric_plan;
pub mod fabric_plan_executor;
pub mod plan_context;
pub mod plan_runner;
pub mod pretense_phase;
pub mod pretenser;
pub mod shape_phase;
pub mod single_interval_drop_test;

impl Fabric {
    pub fn expect_face(&self, face_id: UniqueId) -> &crate::fabric::face::Face {
        self.faces
            .get(&face_id)
            .expect("Face missing")
    }
}

/// A tag that identifies faces - can be a brick name, face name, or context
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum FaceTag {
    Brick(BrickName),
    Face(FaceName),
    Context(FaceContext),
}

impl FaceTag {
    pub fn as_str(&self) -> String {
        match self {
            FaceTag::Brick(b) => b.to_string(),
            FaceTag::Face(f) => f.to_string(),
            FaceTag::Context(c) => c.to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceAlias(pub Vec<FaceTag>);

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
        // Check if all tags in self are also in haystack (subset check)
        self.0.iter().all(|tag| haystack.0.contains(tag))
    }

    pub fn into_vec(self) -> Vec<String> {
        let mut sorted: Vec<_> = self.0.into_iter().map(|tag| tag.as_str()).collect();
        sorted.sort();
        sorted
    }

    pub fn find_face_in(&self, face_list: &[UniqueId], fabric: &Fabric) -> Option<UniqueId> {
        face_list.iter().find_map(|&face_id| {
            let face = fabric.faces.get(&face_id)?;
            self.matches(face.alias()).then_some(face_id)
        })
    }

    pub fn single(tag: FaceTag) -> Self {
        Self(vec![tag])
    }

    pub fn is_base(&self) -> bool {
        self.has(&FaceTag::Face(Single(SingleFace::Base)))
    }

    pub fn with_base(&self) -> Self {
        self.with(FaceTag::Face(Single(SingleFace::Base)))
    }

    pub fn is_seed(&self, which: Option<usize>) -> bool {
        match which {
            Some(1) => self.has(&FaceTag::Context(FaceContext::SeedB)),
            Some(_) => false, // Only SeedB is supported in FaceContext
            None => self.has(&FaceTag::Context(FaceContext::SeedA)),
        }
    }

    pub fn with_seed(&self, which: Option<usize>) -> Self {
        match which {
            Some(1) => self.with(FaceTag::Context(FaceContext::SeedB)),
            Some(_) => self.clone(), // Only SeedB is supported
            None => self.with(FaceTag::Context(FaceContext::SeedA)),
        }
    }

    fn has(&self, sought_tag: &FaceTag) -> bool {
        self.0.contains(sought_tag)
    }

    fn with(&self, extra: FaceTag) -> Self {
        let mut vec = self.0.clone();
        vec.push(extra);
        Self(vec)
    }

    pub fn spin(&self) -> Option<Spin> {
        for tag in &self.0 {
            match tag {
                FaceTag::Context(FaceContext::OnSpinLeft) => return Some(Spin::Left),
                FaceTag::Context(FaceContext::OnSpinRight) => return Some(Spin::Right),
                _ => continue,
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
        let context = match self {
            Spin::Left => FaceContext::OnSpinLeft,
            Spin::Right => FaceContext::OnSpinRight,
        };
        FaceAlias::single(FaceTag::Context(context))
    }
}

#[derive(Debug, Clone)]
pub struct FaceMark {
    face_id: UniqueId,
    mark_name: MarkName,
}

pub fn into_atom(name: String) -> String {
    if name.chars().next().expect("empty string").is_uppercase() {
        name
    } else {
        format!(":{name}")
    }
}
