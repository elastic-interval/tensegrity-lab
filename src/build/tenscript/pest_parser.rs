#![allow(clippy::result_large_err)]
use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use crate::build::tenscript::{BuildPhase, FabricPlan, Seed, SurfaceCharacterSpec};

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct PestParser;

#[derive(Debug, Clone)]
enum ParseError {
    Something(String),
    PestError(Error<Rule>),
}

fn fabric_plan(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, ParseError> {
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
                plan.build_phase = build(pair)?;
            }
            _ => unreachable!("fabric plan"),
        }
    }
    Ok(plan)
}

fn build(build_phase_pair: Pair<Rule>)-> Result<BuildPhase, ParseError> {
    let mut phase = BuildPhase::default();
    for pair in build_phase_pair.into_inner() {
        match pair.as_rule() {
            Rule::seed => {
                phase.seed =
                    match pair.into_inner().next().unwrap().as_str() {
                        ":left-right" => Seed::LeftRight,
                        ":right-left" => Seed::RightLeft,
                        ":left" => Seed::Left,
                        ":right" => Seed::Right,
                        _ => unreachable!()
                    };
            }
            _ => unreachable!("build phase: {:?}", pair.as_rule()),
        }
    }
    Ok(phase)
}

// fn seed(_seed_pair: Pair<Rule>)-> Result<Seed, ParseError> {
//     Ok(Seed::default())
// }

fn parse(source: &str) -> Result<FabricPlan, ParseError> {
    let mut pairs = PestParser::parse(Rule::fabric_plan, source)
        .map_err(ParseError::PestError)?;
    let plan_rule = pairs.next().unwrap();
    fabric_plan(plan_rule)
}

#[cfg(test)]
mod tests {
    use crate::build::tenscript::bootstrap_fabric_plans;
    use crate::build::tenscript::pest_parser::{parse, ParseError};

    #[test]
    fn parse_test() {
        let plans = bootstrap_fabric_plans();
        for (name, code) in plans.iter() {
            if name != "Seed" {
                continue;
            }
            match parse(code) {
                Ok(plan) => {
                    println!("[{name}] Good plan!");
                    dbg!(plan);
                },
                Err(ParseError::PestError(error)) => panic!("[{name}] Error: {error}"),
                Err(error) => panic!("[{name}] Error: {error:?}"),
            }
        }
    }
}