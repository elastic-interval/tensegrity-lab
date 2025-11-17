use pest::iterators::{Pair, Pairs};

use crate::build::tenscript::{PairExt, PairsExt, Rule, TenscriptError};
use crate::fabric::UniqueId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuscleDirection {
    Alpha, // First muscle group - contracts while Omega relaxes
    Omega, // Second muscle group - contracts while Alpha relaxes
}

#[derive(Debug, Clone)]
pub struct AnimatePhase {
    pub contraction: Option<f32>,
    pub frequency_hz: f32, // Obligatory - cycles per second (Hertz)
    pub muscle_intervals: Vec<(UniqueId, UniqueId, MuscleDirection)>,
}

impl AnimatePhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<AnimatePhase, TenscriptError> {
        match pair.as_rule() {
            Rule::animate => Self::parse_features(pair.into_inner()),
            _ => {
                unreachable!()
            }
        }
    }

    fn parse_features(mut pairs: Pairs<Rule>) -> Result<AnimatePhase, TenscriptError> {
        // First element must be frequency (obligatory)
        let frequency_pair = pairs.next()
            .ok_or_else(|| TenscriptError::FormatError("animate phase missing obligatory frequency".to_string()))?;
        let frequency_hz = frequency_pair.parse_float_inner("frequency")?;
        
        let mut animate = AnimatePhase {
            contraction: None,
            frequency_hz,
            muscle_intervals: Vec::new(),
        };
        
        // Parse remaining optional features
        for feature_pair in pairs {
            match feature_pair.as_rule() {
                Rule::animate_feature => {
                    for animate_pair in feature_pair.into_inner() {
                        match animate_pair.as_rule() {
                            Rule::contraction => {
                                animate.contraction = Some(animate_pair.parse_float_inner("contraction")?);
                            }
                            Rule::muscle_interval => {
                                let mut inner = animate_pair.into_inner();
                                let alpha = inner.next_usize("muscle interval alpha")?;
                                let omega = inner.next_usize("muscle interval omega")?;
                                let group_str = inner.next()
                                    .ok_or_else(|| TenscriptError::FormatError("muscle interval missing :alpha or :omega".to_string()))?
                                    .as_str();
                                let direction = match group_str {
                                    ":alpha" => MuscleDirection::Alpha,
                                    ":omega" => MuscleDirection::Omega,
                                    _ => return Err(TenscriptError::FormatError(format!("invalid muscle group: {}", group_str))),
                                };
                                animate.muscle_intervals.push((UniqueId(alpha), UniqueId(omega), direction));
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
        Ok(animate)
    }
}
