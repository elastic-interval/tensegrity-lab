use pest::iterators::{Pair, Pairs};

use crate::build::tenscript::{PairExt, PairsExt, Rule, TenscriptError};
use crate::fabric::physics::presets::AIR_GRAVITY;
use crate::fabric::physics::{Physics, SurfaceCharacter};
use crate::units::Seconds;

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
    pub seconds: Option<Seconds>,
    pub stiffness: Option<f32>,
    pub altitude: Option<f32>,
    pub viscosity: Option<f32>,
    pub drag: Option<f32>,
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
                                let contraction = inner.next_float("muscle contraction")?;
                                let countdown = inner.next_usize("muscle countdown")?;
                                pretense.muscle_movement = Some(MuscleMovement {
                                    contraction,
                                    countdown,
                                })
                            }
                            Rule::pretenst => {
                                pretense.pretenst = Some(pretense_pair.parse_float_inner("pretenst")?);
                            }
                            Rule::seconds => {
                                let factor = pretense_pair.parse_float_inner("seconds")?;
                                pretense.seconds = Some(Seconds(factor));
                            }
                            Rule::stiffness => {
                                pretense.stiffness = Some(pretense_pair.parse_float_inner("stiffness")?);
                            }
                            Rule::altitude => {
                                pretense.altitude = Some(pretense_pair.parse_float_inner("altitude")?);
                            }
                            Rule::viscosity => {
                                pretense.viscosity = Some(pretense_pair.parse_float_inner("viscosity")?);
                            }
                            Rule::drag => {
                                pretense.drag = Some(pretense_pair.parse_float_inner("drag")?);
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

    /// Create the viewing physics by applying pretense customizations to AIR_GRAVITY
    pub fn viewing_physics(&self) -> Physics {
        let pretenst = self.pretenst.unwrap_or(AIR_GRAVITY.pretenst);
        let surface_character = self.surface_character;
        let stiffness_factor = self.stiffness.unwrap_or(AIR_GRAVITY.stiffness_factor);
        // Viscosity and drag are percentages of the default values
        let viscosity = self.viscosity
            .map(|percent| AIR_GRAVITY.viscosity * percent / 100.0)
            .unwrap_or(AIR_GRAVITY.viscosity);
        let drag = self.drag
            .map(|percent| AIR_GRAVITY.drag * percent / 100.0)
            .unwrap_or(AIR_GRAVITY.drag);
        Physics {
            pretenst,
            surface_character,
            stiffness_factor,
            viscosity,
            drag,
            ..AIR_GRAVITY
        }
    }
}
