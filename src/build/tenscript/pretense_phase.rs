use pest::iterators::{Pair, Pairs};
use std::collections::HashSet;

use crate::build::tenscript::{parse_float, parse_float_inside, parse_usize, Rule, TenscriptError};
use crate::fabric::physics::SurfaceCharacter;

#[derive(Debug, Clone, Default)]
pub struct MuscleMovement {
    pub(crate) contraction: f32,
    pub(crate) countdown: usize,
    pub(crate) reversed_groups: HashSet<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface_character: SurfaceCharacter,
    pub muscle_movement: Option<MuscleMovement>,
    pub pretense_factor: Option<f32>,
}

impl PretensePhase {
    pub fn new(surface_character: SurfaceCharacter) -> Self {
        Self {
            surface_character,
            muscle_movement: None,
            pretense_factor: None,
        }
    }

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
                                let mut reversed_groups = HashSet::new();
                                for next_pair in inner {
                                    let group =
                                        parse_usize(next_pair.as_str(), "muscle reversed groups")?;
                                    reversed_groups.insert(group);
                                }
                                pretense.muscle_movement = Some(MuscleMovement {
                                    contraction,
                                    countdown,
                                    reversed_groups,
                                })
                            }
                            Rule::pretense_factor => {
                                let factor = parse_float_inside(pretense_pair, "pretense-factor")?;
                                pretense.pretense_factor = Some(factor)
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
