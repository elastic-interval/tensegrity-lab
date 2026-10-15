use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::{Percent, Seconds};

#[derive(Debug, Clone)]
pub struct PretensePhase {
    pub surface: Option<SurfaceCharacter>,
    pub seconds: Option<Seconds>,
    pub rigidity: Option<Percent>,
    /// Joint label pairs (e.g. `("B00.2", "C00.3")`) whose connecting
    /// interval should be removed at pretense time.
    pub omit_pairs: Vec<(String, String)>,
    /// Joint label pairs whose struts are joined where they cross, just
    /// before slackening.
    pub hinge_pairs: Vec<(String, String)>,
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

