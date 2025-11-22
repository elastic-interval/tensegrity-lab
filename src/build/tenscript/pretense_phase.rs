use crate::fabric::physics::presets::BASE_PHYSICS;
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
    /// Create the viewing physics by applying pretense customizations to BASE_PHYSICS
    pub fn viewing_physics(&self) -> Physics {
        let pretenst = self.pretenst
            .map(|p| Percent(p))
            .unwrap_or(BASE_PHYSICS.pretenst);
        let surface_character = self.surface_character.unwrap_or(BASE_PHYSICS.surface_character);

        Physics {
            pretenst,
            surface_character,
            ..BASE_PHYSICS
        }
    }
}
