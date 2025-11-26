use crate::fabric::physics::Physics;
use crate::fabric::Fabric;

/// Minimal context for running plans headlessly (without UI)
pub struct PlanContext<'a> {
    pub fabric: &'a mut Fabric,
    pub physics: &'a mut Physics,
}

impl<'a> PlanContext<'a> {
    pub fn new(
        fabric: &'a mut Fabric,
        physics: &'a mut Physics,
    ) -> Self {
        Self {
            fabric,
            physics,
        }
    }
}
