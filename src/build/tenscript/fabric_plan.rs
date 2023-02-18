#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use pest::iterators::Pair;

use crate::build::tenscript::{BuildPhase, Library, parse_name, TenscriptError, Rule};
use crate::build::tenscript::build_phase::BuildNode;
use crate::build::tenscript::shape_phase::{Operation, ShapePhase};

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: Vec<String>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

impl FabricPlan {
    pub fn load_preset(plan_name: Vec<String>) -> Option<Self> {
        Library::standard()
            .fabrics
            .into_iter()
            .find(|plan| plan.name == plan_name)
    }

    pub fn from_pair(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, TenscriptError> {
        let [name, build, shape] = fabric_plan_pair.into_inner().next_chunk().unwrap();
        let name = parse_name(name);
        let build_phase = BuildPhase::from_pair(build)?;
        let shape_phase = ShapePhase::from_pair(shape)?;
        let plan = FabricPlan { name, build_phase, shape_phase };
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
                    Operation::Join { mark_name } |
                    Operation::Distance { mark_name, .. } => {
                        shape_marks.insert(mark_name.clone());
                    }
                    Operation::RemoveShapers { mark_names } => {
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