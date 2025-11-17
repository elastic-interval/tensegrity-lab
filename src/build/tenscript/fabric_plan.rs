#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use pest::iterators::Pair;

use crate::build::tenscript::animate_phase::AnimatePhase;
use crate::build::tenscript::build_phase::BuildNode;
use crate::build::tenscript::converge_phase::ConvergePhase;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::shape_phase::{ShapeOperation, ShapePhase};
use crate::build::tenscript::{BuildPhase, PairExt, Rule, TenscriptError};

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

impl FabricPlan {
    pub fn from_pair(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, TenscriptError> {
        let mut inner = fabric_plan_pair.into_inner();
        let quoted = inner.next().unwrap().into_inner().next().unwrap().as_str();
        let name = quoted[1..quoted.len() - 1].to_string();
        let build = inner.next().unwrap();
        let build_phase = BuildPhase::from_pair(build)?;
        let shape_phase = ShapePhase::from_pair(inner.next().unwrap())?;
        let pretense_phase = PretensePhase::from_pair(inner.next().unwrap())?;
        
        // Parse optional converge phase
        let mut converge_phase = None;
        let mut animate_phase = None;
        let mut scale = 1.0;
        
        // Process remaining optional phases
        for pair in inner {
            match pair.as_rule() {
                Rule::converge => {
                    converge_phase = Some(ConvergePhase::from_pair(pair)?);
                }
                Rule::animate => {
                    animate_phase = Some(AnimatePhase::from_pair(pair)?);
                }
                Rule::scale => {
                    scale = pair.parse_float_inner("fabric/scale")?;
                }
                _ => {}
            }
        }
        
        let plan = FabricPlan {
            name,
            build_phase,
            shape_phase,
            pretense_phase,
            converge_phase,
            animate_phase,
            scale,
        };
        Self::validate_fabric_plan(&plan)?;
        Ok(plan)
    }

    fn validate_fabric_plan(plan: &FabricPlan) -> Result<(), TenscriptError> {
        Self::validate_marks(plan)?;
        Ok(())
    }

    fn validate_marks(plan: &FabricPlan) -> Result<(), TenscriptError> {
        let mut build_marks = HashSet::new();
        plan.build_phase.root.traverse(&mut |node| {
            if let BuildNode::Mark { mark_name } = node {
                build_marks.insert(mark_name.clone());
            }
        });
        let mut shape_marks = HashSet::new();
        for operation in &plan.shape_phase.operations {
            operation.traverse(&mut |op| match op {
                ShapeOperation::Joiner { mark_name, .. }
                | ShapeOperation::PointDownwards { mark_name }
                | ShapeOperation::Spacer { mark_name, .. } => {
                    shape_marks.insert(mark_name.clone());
                }
                _ => {}
            })
        }
        let undefined_marks: Vec<_> = shape_marks.difference(&build_marks).cloned().collect();
        if !undefined_marks.is_empty() {
            return Err(TenscriptError::InvalidError(format!(
                "undefined marks in shape phase: {}",
                undefined_marks.join(", ")
            )));
        }
        Ok(())
    }
}
