#![allow(clippy::result_large_err)]

use std::cell::LazyCell;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::build::tenscript::{BuildPhase, SurfaceCharacterSpec};
use crate::build::tenscript::build_phase::BuildNode;
use crate::build::tenscript::shape_phase::{Operation, ShapePhase};

#[derive(Debug, Default, Clone)]
pub struct FabricPlan {
    pub name: String,
    pub surface: Option<SurfaceCharacterSpec>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct PestParser;

#[derive(Debug)]
pub enum ParseError {
    Pest(Error<Rule>),
    Invalid(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Pest(error) => write!(f, "parse error: {error}"),
            ParseError::Invalid(warning) => write!(f, "warning: {warning}"),
        }
    }
}

impl FabricPlan {
    pub fn bootstrap() -> Vec<FabricPlan> {
        let bootstrap: LazyCell<Vec<_>> = LazyCell::new(||
            FabricPlan::from_file(include_str!("bootstrap.scm")).unwrap()
        );
        bootstrap.clone()
    }

    pub fn from_file(source: &str) -> Result<Vec<Self>, ParseError> {
        PestParser::parse(Rule::fabrics, source)
            .map_err(ParseError::Pest)?
            .next()
            .expect("no (fabrics ..)")
            .into_inner()
            .map(FabricPlan::from_pair)
            .collect()
    }

    pub fn boostrap_with_name(plan_name: &str) -> Option<Self> {
        Self::bootstrap()
            .iter()
            .find(|plan| plan.name == plan_name)
            .cloned()
    }

    pub fn from_tenscript(source: &str) -> Result<Self, ParseError> {
        let fabric_plan_pair = PestParser::parse(Rule::fabric_plan, source)
            .map_err(ParseError::Pest)?
            .next()
            .unwrap();
        Self::from_pair(fabric_plan_pair)
    }

    fn from_pair(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, ParseError> {
        let mut plan = FabricPlan::default();
        for pair in fabric_plan_pair.into_inner() {
            match pair.as_rule() {
                Rule::name => {
                    let name_string = pair.into_inner().next().unwrap().as_str();
                    plan.name = name_string[1..name_string.len() - 1].to_string();
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
                _ => unreachable!("fabric plan"),
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
        if let Some(unused_mark) = build_marks.difference(&shape_marks).next() {
            return Err(ParseError::Invalid(format!("unused mark in build phase: :{unused_mark}")));
        }
        if let Some(undefined_mark) = shape_marks.difference(&build_marks).next() {
            return Err(ParseError::Invalid(format!("undefined mark in shape phase: :{undefined_mark}")));
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::fabric_plan::FabricPlan;

    #[test]
    fn parse_test() {
        let plans = FabricPlan::bootstrap();
        println!("{plans:?}")
    }
}