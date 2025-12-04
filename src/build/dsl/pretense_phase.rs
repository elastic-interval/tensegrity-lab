use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface_character: Option<SurfaceCharacter>,
    pub pretenst: Option<f32>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<f32>,
    pub altitude: Option<f32>,
}

impl PretensePhase {
    pub fn viewing_physics(&self) -> Physics {
        let pretenst = self.pretenst
            .map(|p| Percent(p))
            .unwrap_or(VIEWING.pretenst);
        let surface_character = self.surface_character.unwrap_or(VIEWING.surface_character);

        Physics {
            pretenst,
            surface_character,
            ..VIEWING
        }
    }
}
