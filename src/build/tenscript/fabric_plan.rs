#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use pest::iterators::Pair;

use crate::build::tenscript::{BuildPhase, Library, parse_name, ParseError, Rule, SurfaceCharacterSpec};
use crate::build::tenscript::build_phase::BuildNode;
use crate::build::tenscript::shape_phase::{Operation, ShapePhase};

#[derive(Debug, Default, Clone)]
pub struct FabricPlan {
    pub name: String,
    pub surface: Option<SurfaceCharacterSpec>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

impl FabricPlan {
    pub fn boostrap_with_name(plan_name: &str) -> Option<Self> {
        Library::bootstrap()
            .fabrics
            .iter()
            .find(|plan| plan.name == plan_name)
            .cloned()
    }

    pub(crate) fn from_pair(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, ParseError> {
        let mut plan = FabricPlan::default();
        for pair in fabric_plan_pair.into_inner() {
            match pair.as_rule() {
                Rule::name => {
                    plan.name = parse_name(pair);
                }
                Rule::surface => {
                    plan.surface = Some(
                        match pair.into_inner().next().unwrap().as_str() {
                            ":bouncy" => SurfaceCharacterSpec::Bouncy,
                            ":frozen" => SurfaceCharacterSpec::Frozen,
                            ":sticky" => SurfaceCharacterSpec::Sticky,
                            _ => unreachable!()
                        }
                    );
                }
                Rule::build => {
                    plan.build_phase = BuildPhase::from_pair(pair);
                }
                Rule::shape => {
                    plan.shape_phase = ShapePhase::from_pair(pair);
                }
                _ => unreachable!("fabric plan {:?}", pair.as_rule()),
            }
        }
        Self::validate_fabric_plan(&plan)?;
        Ok(plan)
    }

    fn validate_fabric_plan(plan: &FabricPlan) -> Result<(), ParseError> {
        Self::validate_marks(plan)?;
        Ok(())
    }

    fn validate_marks(plan: &FabricPlan) -> Result<(), ParseError> {
        let mut build_marks = HashSet::new();
        if let Some(node) = &plan.build_phase.root {
            node.traverse(&mut |node| {
                if let BuildNode::Mark { mark_name } = node {
                    build_marks.insert(mark_name.clone());
                }
            });
        }
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
            return Err(ParseError::Invalid(format!("unused marks in build phase: {}", unused_marks.join(", "))));
        }
        let undefined_marks: Vec<_> = shape_marks.difference(&build_marks).cloned().collect();
        if !undefined_marks.is_empty() {
            return Err(ParseError::Invalid(format!("undefined marks in shape phase: {}", undefined_marks.join(", "))));
        }
        Ok(())
    }
}