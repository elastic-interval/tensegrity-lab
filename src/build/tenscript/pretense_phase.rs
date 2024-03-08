use pest::iterators::{Pair, Pairs};

use crate::build::tenscript::{Rule, TenscriptError};
use crate::fabric::physics::SurfaceCharacter;

#[derive(Debug, Clone, Default)]
pub struct MuscleMovement {
    pub(crate) amplitude: f32,
    pub(crate) countdown: usize,
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

    pub fn from_pair_option(pair: Option<Pair<Rule>>) -> Result<PretensePhase, TenscriptError> {
        let Some(pair) = pair else {
            return Ok(PretensePhase::default());
        };
        Self::parse_pretense(pair)
    }

    fn parse_pretense(pair: Pair<Rule>) -> Result<PretensePhase, TenscriptError> {
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
                                        _ => unreachable!("surface character"),
                                    }
                            }
                            Rule::muscle => {
                                let mut inner = pretense_pair.into_inner();
                                let [amplitude, countdown] =
                                    [inner.next().unwrap(), inner.next().unwrap()];
                                let amplitude = TenscriptError::parse_float(
                                    amplitude.as_str(),
                                    "muscle amplitude",
                                )?;
                                let countdown = TenscriptError::parse_usize(
                                    countdown.as_str(),
                                    "muscle countdown",
                                )?;
                                pretense.muscle_movement = Some(MuscleMovement {
                                    amplitude,
                                    countdown,
                                })
                            }
                            Rule::pretense_factor => {
                                let factor = TenscriptError::parse_float_inside(
                                    pretense_pair,
                                    "pretense-factor",
                                )?;
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
