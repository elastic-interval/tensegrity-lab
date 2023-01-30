#![allow(clippy::result_large_err)]

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::build::tenscript::{BuildPhase, FabricPlan, FaceName, PostShapeOperation, Seed, ShapePhase, ShaperSpec, SurfaceCharacterSpec, BuildNode};

#[derive(Parser)]
#[grammar = "build/tenscript/tenscript.pest"] // relative to src
struct PestParser;

#[derive(Debug, Clone)]
pub enum ParseError {
    Syntax(String),
    Pest(Error<Rule>),
}

impl FabricPlan {
    pub fn from_tenscript(source: &str) -> Result<Self, ParseError> {
        Self::parse_fabric_plan(
            PestParser::parse(Rule::fabric_plan, source)
                .map_err(ParseError::Pest)?
                .next()
                .unwrap()
        )
    }

    fn parse_fabric_plan(fabric_plan_pair: Pair<Rule>) -> Result<FabricPlan, ParseError> {
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
                    plan.build_phase = Self::parse_build_phase(pair)?;
                }
                Rule::shape => {
                    plan.shape_phase = Self::parse_shape_phase(pair)?;
                }
                _ => unreachable!("fabric plan"),
            }
        }
        Ok(plan)
    }

    fn parse_shape_phase(shape_phase_pair: Pair<Rule>) -> Result<ShapePhase, ParseError> {
        let mut shape_phase = ShapePhase::default();
        for pair in shape_phase_pair.into_inner() {
            match pair.as_rule() {
                Rule::space_statement => {
                    let [mark_name, distance_string] = pair.into_inner().next_chunk().unwrap().map(|p| p.as_str());
                    let distance_factor = distance_string.parse().unwrap();
                    shape_phase.shaper_specs.push(ShaperSpec::Distance {
                        mark_name: mark_name[1..].into(),
                        distance_factor,
                    })
                }
                Rule::join_statement => {
                    let mark_name = pair.into_inner().next().unwrap().as_str();
                    shape_phase.shaper_specs.push(ShaperSpec::Join { mark_name: mark_name[1..].into() })
                }
                Rule::finally_statement => {
                    match pair.into_inner().next().unwrap().as_str() {
                        ":bow-tie-pulls" => {
                            shape_phase.post_shape_operations.push(PostShapeOperation::BowTiePulls)
                        }
                        ":faces-to-triangles" => {
                            shape_phase.post_shape_operations.push(PostShapeOperation::FacesToTriangles)
                        }
                        _ => {
                            return Err(ParseError::Syntax("finally what?".into()));
                        }
                    }
                }
                _ => unreachable!("shape phase")
            }
        }
        Ok(shape_phase)
    }

    fn parse_build_phase(build_phase_pair: Pair<Rule>) -> Result<BuildPhase, ParseError> {
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
                Rule::build_node => {
                    phase.root = Some(Self::parse_build_node(pair).unwrap());
                }
                _ => unreachable!("build phase: {pair:?}"),
            }
        }
        Ok(phase)
    }

    fn parse_build_node(node_pair: Pair<Rule>) -> Result<BuildNode, ParseError> {
        let pair = node_pair.into_inner().next().unwrap();
        match pair.as_rule() {
            Rule::face => {
                let [face_name_pair, node_pair] = pair.into_inner().next_chunk().unwrap();
                let face_name: FaceName = face_name_pair.as_str().try_into().unwrap();
                let node = Self::parse_build_node(node_pair).unwrap();
                Ok(BuildNode::Face {
                    face_name,
                    node: Box::new(node),
                })
            }
            Rule::grow => {
                let mut inner = pair.into_inner();
                let forward_string = inner.next().unwrap().as_str();
                let forward = match forward_string.parse() {
                    Ok(count) => { "X".repeat(count) }
                    Err(_) => { forward_string[1..forward_string.len() - 1].into() }
                };
                let scale_factor = Self::parse_scale(inner.next());
                let post_growth_node = inner.next()
                    .map(|post_growth| Box::new(Self::parse_build_node(post_growth).unwrap()));
                Ok(BuildNode::Grow {
                    forward,
                    scale_factor,
                    post_growth_node,
                })
            }
            Rule::mark => {
                let mark_name = pair.into_inner().next().unwrap().as_str()[1..].into();
                Ok(BuildNode::Mark { mark_name })
            }
            Rule::branch => {
                Ok(BuildNode::Branch {
                    face_nodes: pair.into_inner()
                        .map(|face_node| Self::parse_build_node(face_node).unwrap())
                        .collect()
                })
            }
            _ => unreachable!("node"),
        }
    }

    fn parse_scale(scale_pair: Option<Pair<Rule>>) -> f32 {
        match scale_pair {
            None => 1.0,
            Some(scale_pair) => {
                let scale_string = scale_pair.into_inner().next().unwrap().as_str();
                scale_string.parse().unwrap()
            }
        }
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