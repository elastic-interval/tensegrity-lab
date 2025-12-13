use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub pretenst: Option<Percent>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
}

impl PretensePhase {
    pub fn viewing_physics(&self) -> Physics {
        let pretenst = self.pretenst
            .unwrap_or(VIEWING.pretenst);

        Physics {
            pretenst,
            surface: self.surface,
            ..VIEWING
        }
    }
}
