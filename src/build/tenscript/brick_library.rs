use std::{fs, iter};
use cgmath::SquareMatrix;
use pest::iterators::Pair;
use pest::Parser;
use crate::build::tenscript::{FaceAlias, Rule, TenscriptError, TenscriptParser};
use crate::build::tenscript::brick::{Baked, BrickDefinition};

#[derive(Clone, Default, Debug)]
pub struct BrickLibrary {
    pub brick_definitions: Vec<BrickDefinition>,
}

impl BrickLibrary {
    pub fn from_source() -> Result<Self, TenscriptError> {
        let source = fs::read_to_string("brick_library.scm")
            .map_err(TenscriptError::FileRead)?;
        Self::from_tenscript(&source)
    }

    fn from_tenscript(source: &str) -> Result<Self, TenscriptError> {
        let pair = TenscriptParser::parse(Rule::bricks, source)
            .map_err(TenscriptError::Pest)?
            .next()
            .expect("no (bricks ..)");
        Self::from_pair(pair)
    }

    fn from_pair(pair: Pair<Rule>) -> Result<Self, TenscriptError> {
        let mut library = Self::default();
        for definition in pair.into_inner() {
            match definition.as_rule() {
                Rule::brick_definition => {
                    let brick = BrickDefinition::from_pair(definition)?;
                    library.brick_definitions.push(brick);
                }
                _ => unreachable!()
            }
        }
        Ok(library)
    }

    pub fn new_brick(&self, search_alias: &FaceAlias) -> Baked {
        let baked_bricks: Vec<_> = self
            .brick_definitions
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
                        let aliases: Vec<_> = face.aliases
                            .into_iter()
                            .map(|alias| {
                                let space = if alias.is_seed() {
                                    baked.down_rotation()
                                } else {
                                    face_space
                                };
                                (alias, space)
                            })
                            .collect();
                        aliases
                            .into_iter()
                            .map(move |(alias, space)| {
                                let alias = alias + &baked.alias;
                                let mut baked = baked.clone();
                                baked.apply_matrix(space);
                                (alias, baked)
                            })
                    })
            })
            .collect();
        let search_with_base = search_alias.with_base();
        let (_, baked) = baked_bricks
            .iter()
            .filter(|(baked_alias, _)| search_with_base.matches(baked_alias))
            .min_by_key(|(brick_alias, _)| brick_alias.0.len())
            .expect(&format!("no such brick: '{search_with_base}'"));
        let mut thawed = baked.clone();
        for face in &mut thawed.faces {
            face.aliases.retain(|candidate| search_alias.matches(candidate));
            assert_eq!(face.aliases.len(), 1, "exactly one face should be retained {:?}", face.aliases);
        }
        thawed.clone()
    }
}