use pest::iterators::Pair;
use crate::build::tenscript::{Rule, TenscriptError};
use crate::fabric::physics::SurfaceCharacter;

#[derive(Debug, Clone, Default)]
pub struct PretensePhase {
    pub surface_character: SurfaceCharacter,
    pub muscle_shortening: Option<f32>,
    pub pretense_factor: Option<f32>,
}

impl PretensePhase {
    pub fn new(surface_character: SurfaceCharacter) -> Self {
        Self {
            surface_character,
            muscle_shortening: None,
            pretense_factor: None,
        }
    }

    pub fn from_pair(pair: Option<Pair<Rule>>) -> Result<PretensePhase, TenscriptError> {
        let mut pretense = PretensePhase::default();
        if let Some(pair) = pair {
            match pair.as_rule() {
                Rule::pretense => {
                    for feature_pair in pair.into_inner() {
                        match feature_pair.as_rule() {
                            Rule::pretense_feature => {
                                for pretense_pair in feature_pair.into_inner() {
                                    match pretense_pair.as_rule() {
                                        Rule::surface => {
                                            pretense.surface_character = match pretense_pair.into_inner().next().unwrap().as_str() {
                                                ":frozen" => SurfaceCharacter::Frozen,
                                                ":bouncy" => SurfaceCharacter::Bouncy,
                                                ":absent" => SurfaceCharacter::Absent,
                                                _ => unreachable!("surface character")
                                            }
                                        }
                                        Rule::muscle => {
                                            let shortening = TenscriptError::parse_float_inside(pretense_pair, "muscle")?;
                                            pretense.muscle_shortening = Some(shortening)
                                        }
                                        Rule::pretense_factor => {
                                            let factor = TenscriptError::parse_float_inside(pretense_pair, "pretense-factor")?;
                                            pretense.pretense_factor = Some(factor)
                                        }
                                        _ => unreachable!()
                                    }
                                }
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                    }
                }
                _ => {
                    unreachable!()
                }
            }
        };
        Ok(pretense)
    }
}