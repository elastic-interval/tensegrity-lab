use pest::iterators::Pair;
use pest::Parser;

use crate::build::tenscript::{FabricPlan, Rule, TenscriptError, TenscriptParser};

#[derive(Clone, Default, Debug)]
pub struct FabricLibrary {
    pub fabric_plans: Vec<FabricPlan>,
}

impl FabricLibrary {
    pub fn from_source() -> Result<Self, TenscriptError> {
        let source = Self::load_source()?;
        Self::from_tenscript(&source)
    }
    
    pub fn fabric_list(&self) -> Result<Vec<String>, TenscriptError> {
        Ok(self
            .fabric_plans
            .iter()
            .map(|plan| plan.name.clone())
            .collect())
    }

    pub fn load_source() -> Result<String, TenscriptError> {
        let source: String;
        #[cfg(target_arch = "wasm32")]
        {
            source = include_str!("../../../fabric_library.scm").to_string();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs;
            source = fs::read_to_string("fabric_library.scm").map_err(TenscriptError::FileReadError)?;
        }
        Ok(source)
    }

    pub fn load_specific_fabric_source(
        name: Vec<String>,
    ) -> Result<Option<String>, TenscriptError> {
        let source = Self::load_source()?;
        let pair = Self::parse_fabric_library_pair(&source)?;
        for fabric_pair in pair.into_inner() {
            let name_pair = fabric_pair
                .clone()
                .into_inner()
                .find(|pair| pair.as_rule() == Rule::name)
                .expect("no name pair");
            let fabric_name: Vec<String> = name_pair
                .into_inner()
                .map(|pair| {
                    let str_value = pair.as_str();
                    str_value[1..str_value.len() - 1].to_string()
                })
                .collect();
            if name == fabric_name {
                return Ok(Some(fabric_pair.as_str().to_string()));
            }
        }
        Ok(None)
    }

    fn from_tenscript(source: &str) -> Result<Self, TenscriptError> {
        let pair = Self::parse_fabric_library_pair(source)?;
        Self::from_pair(pair)
    }

    fn parse_fabric_library_pair(source: &str) -> Result<Pair<Rule>, TenscriptError> {
        let pair = TenscriptParser::parse(Rule::fabric_library, source)
            .map_err(TenscriptError::PestError)?
            .next()
            .expect("no (fabric-library ..)");
        Ok(pair)
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
