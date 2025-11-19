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
    pub viscosity: Option<f32>,
    pub drag: Option<f32>,
}

impl PretensePhase {
    /// Create the viewing physics by applying pretense customizations to BASE_PHYSICS
    pub fn viewing_physics(&self) -> Physics {
        let pretenst = self.pretenst
            .map(|p| Percent(p))
            .unwrap_or(BASE_PHYSICS.pretenst);
        let surface_character = self.surface_character.unwrap_or(BASE_PHYSICS.surface_character);
        // Viscosity and drag are percentages of the default values
        let viscosity = self.viscosity
            .map(|percent| BASE_PHYSICS.viscosity * percent / 100.0)
            .unwrap_or(BASE_PHYSICS.viscosity);
        let drag = self.drag
            .map(|percent| BASE_PHYSICS.drag * percent / 100.0)
            .unwrap_or(BASE_PHYSICS.drag);
        Physics {
            pretenst,
            surface_character,
            viscosity,
            drag,
            ..BASE_PHYSICS
        }
    }
}
