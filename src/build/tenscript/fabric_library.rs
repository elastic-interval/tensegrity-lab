use pest::iterators::Pair;
use pest::Parser;

use crate::build::tenscript::{FabricPlan, Rule, TenscriptError, TenscriptParser};

#[derive(Clone, Default, Debug)]
pub struct FabricLibrary {
    pub fabric_plans: Vec<FabricPlan>,
}

impl FabricLibrary {
    pub fn from_source() -> Result<Self, TenscriptError> {
        let source: String;
        #[cfg(target_arch = "wasm32")]
        {
            source = include_str!("../../../fabric_library.scm").to_string();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs;
            source = fs::read_to_string("fabric_library.scm").map_err(TenscriptError::FileRead)?;
        }
        Self::from_tenscript(&source)
    }

    fn from_tenscript(source: &str) -> Result<Self, TenscriptError> {
        let pair = TenscriptParser::parse(Rule::fabric_library, source)
            .map_err(TenscriptError::Pest)?
            .next()
            .expect("no (fabrics ..)");
        Self::from_pair(pair)
    }

    fn from_pair(pair: Pair<Rule>) -> Result<Self, TenscriptError> {
        let mut library = Self::default();
        for definition in pair.into_inner() {
            match definition.as_rule() {
                Rule::fabric_plan => {
                    let fabric_plan = FabricPlan::from_pair(definition)?;
                    library.fabric_plans.push(fabric_plan);
                }
                _ => unreachable!(),
            }
        }
        Ok(library)
    }
}
