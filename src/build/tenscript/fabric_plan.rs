#![allow(clippy::result_large_err)]

use pest::error::Error;
use pest::Parser;
use pest_derive::Parser;

use crate::build::tenscript::{BuildPhase, SurfaceCharacterSpec};
use crate::build::tenscript::shape_phase::ShapePhase;

#[derive(Debug, Default, Clone)]
pub struct FabricPlan {
    pub surface: Option<SurfaceCharacterSpec>,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
}

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct PestParser;

pub type ParseError = Error<Rule>;

pub fn fabric_plans_from_bootstrap() -> Vec<(String, String)> {
    include_str!("bootstrap.scm")
        .split(";;;")
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            let line_end = chunk.find('\n').unwrap_or_else(|| {
                panic!("bootstrap.scm not structured properly");
            });
            (chunk[0..line_end].to_string(), chunk[(line_end + 1)..].to_string())
        })
        .collect()
}

impl FabricPlan {
    pub fn from_bootstrap(plan_name: &str) -> Self {
        let plans = fabric_plans_from_bootstrap();
        let Some((_, code)) = plans.iter().find(|&(name, _)| *name == plan_name) else {
            panic!("{plan_name} not found");
        };
        match Self::from_tenscript(code.as_str()) {
            Ok(plan) => plan,
            Err(error) => panic!("error parsing bootstrap fabric plan: {error}")
        }
    }

    pub fn from_tenscript(source: &str) -> Result<Self, ParseError> {
        let fabric_plan_pair = PestParser::parse(Rule::fabric_plan, source)?
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
                    plan.build_phase = BuildPhase::from_pair(pair);
                }
                Rule::shape => {
                    plan.shape_phase = ShapePhase::from_pair(pair);
                }
                _ => unreachable!("fabric plan"),
            }
        }
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::fabric_plan::{fabric_plans_from_bootstrap, FabricPlan};

    #[test]
    fn parse_test() {
        let plans = fabric_plans_from_bootstrap();
        for (name, code) in plans.iter() {
            match FabricPlan::from_tenscript(code.as_str()) {
                Ok(plan) => {
                    println!("[{name}] Good plan!");
                    dbg!(plan);
                }
                Err(error) => panic!("[{name}] Error: {error}"),
            }
        }
    }
}