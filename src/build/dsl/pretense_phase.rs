use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone)]
pub struct PretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
    pub omit_pairs: Vec<(JointPath, JointPath)>,
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
