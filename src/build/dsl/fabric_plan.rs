#![allow(clippy::result_large_err)]

use crate::build::dsl::animate_phase::AnimatePhase;
use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::settle_phase::SettlePhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::ShapePhase;

use crate::units::Millimeters;

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: String,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
    pub fall_phase: FallPhase,
    pub settle_phase: SettlePhase,
    pub animate_phase: Option<AnimatePhase>,
    pub scale: f32,
    pub altitude: Millimeters,
}
