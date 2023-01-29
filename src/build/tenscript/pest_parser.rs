#![allow(clippy::result_large_err)]

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use crate::build::tenscript::{BuildPhase, FabricPlan, FaceName, Seed, ShapePhase, SurfaceCharacterSpec, TenscriptNode};

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct PestParser;

#[derive(Debug, Clone)]
enum ParseError {
    ToBeDone,
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
            Rule::shape => {
                plan.shape_phase = shape(pair)?;
            }
            _ => unreachable!("fabric plan"),
        }
    }
    Ok(plan)
}

fn shape(shape_pair: Pair<Rule>) -> Result<ShapePhase, ParseError> {
    Ok(ShapePhase::default())
}

fn build(build_phase_pair: Pair<Rule>) -> Result<BuildPhase, ParseError> {
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
            Rule::node => {
                phase.root = Some(node(pair).unwrap());
            }
            _ => unreachable!("build phase: {:?}", pair.as_rule()),
        }
    }
    Ok(phase)
}

fn node(node_pair: Pair<Rule>) -> Result<TenscriptNode, ParseError> {
    let pair = node_pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::face => {
            let mut inner_pairs = pair.into_inner();
            let face_name_string = inner_pairs.next().unwrap().as_str();
            let face_name: FaceName = face_name_string[1..].try_into().unwrap();
            let node = node(inner_pairs.next().unwrap()).unwrap();
            Ok(TenscriptNode::Face {
                face_name,
                node: Box::new(node),
            })
        }
        Rule::grow => {
            let mut inner = pair.into_inner();
            let count = inner.next().unwrap().as_str().parse().unwrap();
            let scale_factor = scale(inner.next());
            Ok(TenscriptNode::Grow {
                forward: "X".repeat(count),
                scale_factor,
                post_growth_node: None,
            })
        }
        Rule::mark => {
            Err(ParseError::ToBeDone)
        }
        Rule::branch => {
            Ok(TenscriptNode::Branch{
                face_nodes:pair.into_inner()
                    .map(|face_node| node(face_node).unwrap()).collect()
            })
        }
        _ => unreachable!("node"),
    }
}

fn scale(scale_pair: Option<Pair<Rule>>) -> f32 {
    match scale_pair {
        None => 1.0,
        Some(scale_pair) => {
            let scale_string = scale_pair.into_inner().next().unwrap().as_str();
            scale_string.parse().unwrap()
        }
    }
}

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
            if name != "Knee" {
                continue;
            }
            match parse(code) {
                Ok(plan) => {
                    println!("[{name}] Good plan!");
                    dbg!(plan);
                }
                Err(ParseError::PestError(error)) => panic!("[{name}] Error: {error}"),
                Err(error) => panic!("[{name}] Error: {error:?}"),
            }
        }
    }
}