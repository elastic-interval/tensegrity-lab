use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub pretenst: Option<Percent>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
    pub omit_pairs: Vec<(JointPath, JointPath)>,
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
