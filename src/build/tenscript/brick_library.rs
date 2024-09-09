use std::iter;

use cgmath::SquareMatrix;
use pest::iterators::Pair;
use pest::Parser;

use crate::build::tenscript::brick::{Baked, BrickDefinition};
use crate::build::tenscript::{FaceAlias, Rule, TenscriptError, TenscriptParser};

#[derive(Clone, Debug)]
pub struct BrickLibrary {
    pub brick_definitions: Vec<BrickDefinition>,
    pub baked_bricks: Vec<(FaceAlias, Baked)>,
}

impl BrickLibrary {
    pub fn from_source() -> Result<Self, TenscriptError> {
        let source: String;
        #[cfg(target_arch = "wasm32")]
        {
            source = include_str!("../../../brick_library.tenscript").to_string();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs;
            source = fs::read_to_string("brick_library.tenscript").map_err(TenscriptError::FileReadError)?;
        }
        Self::from_tenscript(&source)
    }

    fn from_tenscript(source: &str) -> Result<Self, TenscriptError> {
        let pair = TenscriptParser::parse(Rule::brick_library, source)
            .map_err(TenscriptError::PestError)?
            .next()
            .expect("no (bricks ..)");
        Self::from_pair(pair)
    }

    fn from_pair(pair: Pair<Rule>) -> Result<Self, TenscriptError> {
        let mut brick_definitions = Vec::new();
        for definition in pair.into_inner() {
            match definition.as_rule() {
                Rule::brick_definition => {
                    let brick = BrickDefinition::from_pair(definition)?;
                    brick_definitions.push(brick);
                }
                _ => unreachable!(),
            }
        }
        let baked_bricks: Vec<_> = brick_definitions
            .iter()
            .filter_map(|brick| brick.baked.clone())
            .flat_map(|baked| {
                let cloned_bricks = iter::repeat(baked.clone());
                baked
                    .faces
                    .into_iter()
                    .zip(cloned_bricks)
                    .flat_map(|(face, baked)| {
                        let face_space = face.vector_space(&baked).invert().unwrap();
                        let aliases: Vec<_> = face
                            .aliases
                            .into_iter()
                            .map(|alias| {
                                let seed = [None, Some(1), Some(2)]
                                    .into_iter()
                                    .find(|seed|  alias.is_seed(*seed));
                                let space = match seed {
                                    Some(seed) => {
                                        baked.down_rotation(seed)
                                    }
                                    None=> {
                                        face_space
                                    }
                                };
                                (alias, space)
                            })
                            .collect();
                        aliases.into_iter().map(move |(alias, space)| {
                            let mut baked = baked.clone();
                            baked.apply_matrix(space);
                            (alias, baked)
                        })
                    })
            })
            .collect();
        Ok(BrickLibrary {
            brick_definitions,
            baked_bricks,
        })
    }

    pub fn new_brick(&self, search_alias: &FaceAlias) -> Result<Baked, TenscriptError> {
        let search_with_base = search_alias.with_base();
        let (_, baked) = self
            .baked_bricks
            .iter()
            .filter(|(baked_alias, _)| search_with_base.matches(baked_alias))
            .min_by_key(|(brick_alias, _)| brick_alias.0.len())
            .ok_or(TenscriptError::FaceAliasError(format!("Cannot find a face to match {:?}", search_with_base)))?;
        let mut thawed = baked.clone();
        for face in &mut thawed.faces {
            face.aliases
                .retain(|candidate| search_alias.matches(candidate));
            assert_eq!(
                face.aliases.len(),
                1,
                "exactly one face should be retained {:?}",
                face.aliases
            );
        }
        Ok(thawed)
    }
}
