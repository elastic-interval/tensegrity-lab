use crate::build::tenscript::brick_library::BrickLibrary;
use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

/// Minimal context for running plans headlessly (without UI)
pub struct PlanContext<'a> {
    pub fabric: &'a mut Fabric,
    pub physics: &'a mut Physics,
    pub brick_library: &'a BrickLibrary,
}

impl<'a> PlanContext<'a> {
    pub fn new(
        fabric: &'a mut Fabric,
        physics: &'a mut Physics,
        brick_library: &'a BrickLibrary,
    ) -> Self {
        Self {
            fabric,
            physics,
            brick_library,
        }
    }
}
