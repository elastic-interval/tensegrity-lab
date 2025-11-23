#![allow(clippy::result_large_err)]

use crate::build::dsl::animate_phase::AnimatePhase;
use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::converge_phase::ConvergePhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::ShapePhase;

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: String,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
    pub converge_phase: Option<ConvergePhase>,
    pub animate_phase: Option<AnimatePhase>,
    pub scale: f32,
}
