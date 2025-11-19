#![allow(clippy::result_large_err)]

use crate::build::tenscript::animate_phase::AnimatePhase;
use crate::build::tenscript::build_phase::BuildPhase;
use crate::build::tenscript::converge_phase::ConvergePhase;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::shape_phase::ShapePhase;

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
