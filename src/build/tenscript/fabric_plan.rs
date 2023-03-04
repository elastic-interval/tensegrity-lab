#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use pest::iterators::Pair;

use crate::build::tenscript::{BuildPhase, parse_name, Rule, TenscriptError};
use crate::build::tenscript::build_phase::BuildNode;
use crate::build::tenscript::shape_phase::{ShapeOperation, ShapePhase};
use crate::fabric::physics::SurfaceCharacter;

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: Vec<String>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
}

impl FabricPlan {
    pub fn from_pair(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, TenscriptError> {
        let mut inner = fabric_plan_pair.into_inner();
        let [name, build] = inner.next_chunk().unwrap();
        let name = parse_name(name);
        let build_phase = BuildPhase::from_pair(build)?;
        let shape_phase = ShapePhase::from_pair(inner.next())?;
        let pretense_phase = PretensePhase::from_pair(inner.next())?;
        let plan = FabricPlan { name, build_phase, shape_phase, pretense_phase };
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
            operation.traverse(&mut |op| {
                match op {
                    ShapeOperation::Join { mark_name } |
                    ShapeOperation::PointDownwards { mark_name } |
                    ShapeOperation::Distance { mark_name, .. } => {
                        shape_marks.insert(mark_name.clone());
                    }
                    ShapeOperation::RemoveShapers { mark_names } => {
                        shape_marks.extend(mark_names.iter().cloned());
                    }
                    _ => {}
                }
            })
        }
        let unused_marks: Vec<_> = build_marks.difference(&shape_marks).cloned().collect();
        if !unused_marks.is_empty() {
            return Err(TenscriptError::Invalid(format!("unused marks in build phase: {}", unused_marks.join(", "))));
        }
        let undefined_marks: Vec<_> = shape_marks.difference(&build_marks).cloned().collect();
        if !undefined_marks.is_empty() {
            return Err(TenscriptError::Invalid(format!("undefined marks in shape phase: {}", undefined_marks.join(", "))));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub(crate) surface_character: SurfaceCharacter,
}

impl PretensePhase {
    pub fn from_pair(pair: Option<Pair<Rule>>) -> Result<PretensePhase, TenscriptError> {
        let mut pretense = PretensePhase::default();
        if let Some(pair) = pair {
            match pair.as_rule() {
                Rule::pretense => {
                    let surface_character = match pair.into_inner().next().unwrap().as_str() {
                        ":frozen" => SurfaceCharacter::Frozen,
                        ":bouncy" => SurfaceCharacter::Bouncy,
                        ":absent" => SurfaceCharacter::Absent,
                        _ => unreachable!("surface character")
                    };
                    pretense.surface_character = surface_character
                }
                _ => {
                    unreachable!()
                }
            }
        };
        Ok(pretense)
    }
}