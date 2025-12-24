use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

/// Default target compression for push intervals (1%)
pub const DEFAULT_MIN_PUSH_STRAIN: f32 = 0.01;
/// Default maximum compression per extension round (3%)
pub const DEFAULT_MAX_PUSH_STRAIN: f32 = 0.03;

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub pretenst: Option<Percent>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
    pub omit_pairs: Vec<(JointPath, JointPath)>,
    /// Target compression for push intervals (default 1%)
    pub min_push_strain: Option<f32>,
    /// Maximum compression per extension round (default 3%)
    pub max_push_strain: Option<f32>,
}

impl PretensePhase {
    /// Note: This returns physics without surface since scale is not known here.
    /// The surface with proper scale is set by FabricPlanExecutor during transition_to_fall.
    pub fn viewing_physics(&self) -> Physics {
        let pretenst = self.pretenst.unwrap_or(VIEWING.pretenst);

        Physics {
            pretenst,
            surface: None,
            ..VIEWING
        }
    }
}
