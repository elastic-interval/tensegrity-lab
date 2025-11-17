use pest::iterators::{Pair, Pairs};

use crate::build::tenscript::{PairExt, Rule, TenscriptError};
use crate::units::Seconds;

#[derive(Debug, Clone)]
pub struct ConvergePhase {
    pub seconds: Seconds,
}

impl ConvergePhase {
    pub fn from_pair(pair: Pair<Rule>) -> Result<ConvergePhase, TenscriptError> {
        match pair.as_rule() {
            Rule::converge => Self::parse_features(pair.into_inner()),
            _ => {
                unreachable!()
            }
        }
    }

    fn parse_features(mut pairs: Pairs<Rule>) -> Result<ConvergePhase, TenscriptError> {
        let float_pair = pairs.next()
            .ok_or_else(|| TenscriptError::FormatError("converge phase missing obligatory time duration".to_string()))?;
        
        let value = float_pair.parse_float_str("converge duration")?;
        Ok(ConvergePhase { seconds: Seconds(value) })
    }
}
