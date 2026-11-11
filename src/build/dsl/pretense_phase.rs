use crate::build::dsl::joint_names::{Bridle, Link};
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone)]
pub struct PretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
    /// Links whose intervals are removed at pretense time, each with its
    /// images (see `joint_names`).
    pub omit_pairs: Vec<Link>,
    /// Links whose struts are joined where they cross, just before
    /// slackening: the first end's joint keeps its name.
    pub hinge_pairs: Vec<Link>,
    /// Joints whose cables to two others are taken off through a bridle
    /// node, after hinging.
    pub bridles: Vec<Bridle>,
    /// Percentage by which each push interval's rest length grows during the
    /// pretensing phase. The same factor is applied to every push; pulls
    /// absorb the displacement and gain tension.
    pub pretenst: Percent,
}

impl PretensePhase {
    pub fn viewing_physics(&self) -> Physics {
        VIEWING.clone()
    }
}

