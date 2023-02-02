#![allow(clippy::result_large_err)]

use std::fmt::{Display, Formatter};

use pest::error::Error;
use pest::Parser;
use pest_derive::Parser;

use crate::build::tenscript::{BuildPhase, FabricPlan, SurfaceCharacterSpec};
use crate::build::tenscript::shape_phase::ShapePhase;

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct PestParser;

#[derive(Debug, Clone)]
pub enum ParseError {
    Syntax(String),
    Pest(Error<Rule>),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Syntax(message) => write!(f, "syntax error: {message}"),
            ParseError::Pest(error) => write!(f, "pest parse error: {error}"),
        }
    }
}


impl FabricPlan {
    pub fn from_tenscript(source: &str) -> Result<Self, ParseError> {
        let fabric_plan_pair = PestParser::parse(Rule::fabric_plan, source)
            .map_err(ParseError::Pest)?
            .next()
            .unwrap();
        let mut plan = FabricPlan::default();
        for pair in fabric_plan_pair.into_inner() {
            match pair.as_rule() {
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
                    plan.build_phase = BuildPhase::from_pair(pair)?;
                }
                Rule::shape => {
                    plan.shape_phase = ShapePhase::from_pair(pair)?;
                }
                _ => unreachable!("fabric plan"),
            }
        }
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::{bootstrap_fabric_plans, FabricPlan};
    use crate::build::tenscript::parser::ParseError;

    #[test]
    fn parse_test() {
        let plans = bootstrap_fabric_plans();
        for (name, code) in plans.iter() {
            match FabricPlan::from_tenscript(code.as_str()) {
                Ok(plan) => {
                    println!("[{name}] Good plan!");
                    dbg!(plan);
                }
                Err(ParseError::Pest(error)) => panic!("[{name}] Error: {error}"),
                Err(error) => panic!("[{name}] Error: {error:?}"),
            }
        }
    }
}