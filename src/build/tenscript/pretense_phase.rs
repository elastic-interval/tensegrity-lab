use pest::iterators::{Pair, Pairs};

use crate::build::tenscript::{parse_float, parse_float_inside, parse_usize, parse_usize_inside, Rule, TenscriptError};
use crate::fabric::physics::SurfaceCharacter;

#[derive(Debug, Clone, Default)]
pub struct MuscleMovement {
    pub(crate) contraction: f32,
    pub(crate) countdown: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface_character: SurfaceCharacter,
    pub muscle_movement: Option<MuscleMovement>,
    pub pretenst: Option<f32>,
    pub countdown: Option<usize>,
    pub stiffness: Option<f32>,
}

impl PretensePhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<PretensePhase, TenscriptError> {
        match pair.as_rule() {
            Rule::pretense => Self::parse_features(pair.into_inner()),
            _ => {
                unreachable!()
            }
        }
    }

    fn parse_features(pairs: Pairs<Rule>) -> Result<PretensePhase, TenscriptError> {
        let mut pretense = PretensePhase::default();
        for feature_pair in pairs {
            match feature_pair.as_rule() {
                Rule::pretense_feature => {
                    for pretense_pair in feature_pair.into_inner() {
                        match pretense_pair.as_rule() {
                            Rule::surface => {
                                pretense.surface_character =
                                    match pretense_pair.into_inner().next().unwrap().as_str() {
                                        ":frozen" => SurfaceCharacter::Frozen,
                                        ":bouncy" => SurfaceCharacter::Bouncy,
                                        ":absent" => SurfaceCharacter::Absent,
                                        ":sticky" => SurfaceCharacter::Sticky,
                                        _ => unreachable!("surface character"),
                                    }
                            }
                            Rule::muscle => {
                                let mut inner = pretense_pair.into_inner();
                                let contraction = parse_float(
                                    inner.next().unwrap().as_str(),
                                    "muscle contraction",
                                )?;
                                let countdown = parse_usize(
                                    inner.next().unwrap().as_str(),
                                    "muscle countdown",
                                )?;
                                pretense.muscle_movement = Some(MuscleMovement {
                                    contraction,
                                    countdown,
                                })
                            }
                            Rule::pretenst => {
                                let factor = parse_float_inside(pretense_pair, "pretenst")?;
                                pretense.pretenst = Some(factor)
                            }
                            Rule::countdown => {
                                let factor = parse_usize_inside(pretense_pair, "countdown")?;
                                pretense.countdown = Some(factor)
                            }
                            Rule::stiffness => {
                                let stiffness = parse_float_inside(pretense_pair, "stiffness")?;
                                pretense.stiffness = Some(stiffness)
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                _ => {
                    unreachable!()
                }
            }
        }
        Ok(pretense)
    }
}
