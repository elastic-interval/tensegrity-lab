use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone)]
pub struct ZeroGPretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
    pub omit_pairs: Vec<(JointPath, JointPath)>,
    pub min_push_strain: f32,
    pub pull_lengthening: f32,
}

impl ZeroGPretensePhase {
    pub fn viewing_physics(&self) -> Physics {
        VIEWING.clone()
    }
}
