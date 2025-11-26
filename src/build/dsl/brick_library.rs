use cgmath::SquareMatrix;

use crate::build::dsl::brick::{Baked, Brick};
use crate::build::dsl::brick_dsl::BrickName;
use crate::build::dsl::FaceAlias;

#[derive(Clone, Debug)]
pub struct BrickLibrary {
    pub brick_definitions: Vec<Brick>,
    pub baked_bricks: Vec<(FaceAlias, Baked)>,
}

impl BrickLibrary {
    
    pub fn get_brick(&self, brick_name: BrickName) -> Baked {
        let definition = self.brick_definitions.iter().find(|definition|definition.proto.brick_name == brick_name).unwrap();
        definition.clone().baked.unwrap()
    }
    
    /// Create a BrickLibrary from brick definitions.
    ///
    /// # Example
    /// ```
    /// use tensegrity_lab::build::dsl::brick_builders::build_brick_library;
    /// use tensegrity_lab::build::dsl::brick_library::BrickLibrary;
    ///
    /// let brick_library = BrickLibrary::new(build_brick_library());
    /// ```
    pub fn new(brick_definitions: Vec<Brick>) -> Self {
        let baked_bricks = Self::compute_baked_bricks(&brick_definitions);
        BrickLibrary {
            brick_definitions,
            baked_bricks,
        }
    }

    /// Compute baked bricks with all face transformations
    ///
    /// This transforms brick_definitions into pre-computed baked bricks with all possible
    /// face orientations, used for efficient brick instantiation during construction.
    fn compute_baked_bricks(brick_definitions: &[Brick]) -> Vec<(FaceAlias, Baked)> {
        brick_definitions
            .iter()
            .filter_map(|brick_def| {
                // Get baked brick with faces populated
                let mut baked = brick_def.baked.clone()?;
                // Ensure faces are populated (either from baked.faces or derived from proto)
                if baked.faces.is_empty() {
                    baked.faces = brick_def.baked_faces();
                }
                Some(baked)
            })
            .flat_map(|baked| {
                // Clone baked repeatedly for each face
                let baked_iter = std::iter::repeat(baked.clone());
                baked
                    .faces
                    .clone()
                    .into_iter()
                    .zip(baked_iter)
                    .flat_map(|(face, baked)| {
                        let face_space = face.vector_space(&baked).invert().unwrap();
                        let aliases: Vec<_> = face
                            .aliases
                            .into_iter()
                            .map(|alias| {
                                let seed = [None, Some(1), Some(2)]
                                    .into_iter()
                                    .find(|seed| alias.is_seed(*seed));
                                let space = match seed {
                                    Some(seed) => baked.down_rotation(seed),
                                    None => face_space,
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
            .collect()
    }

    pub fn new_brick(&self, search_alias: &FaceAlias) -> Baked {
        let (_, baked) = self
            .baked_bricks
            .iter()
            .filter(|(baked_alias, _)| baked_alias.is_base())
            .min_by_key(|(face_alias, _)| face_alias.0.len())
            .expect(&format!("Cannot find a face to match {:?}", search_alias));
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
        thawed
    }
}
