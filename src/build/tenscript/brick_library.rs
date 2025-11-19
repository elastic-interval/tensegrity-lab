use cgmath::SquareMatrix;

use crate::build::tenscript::brick::{Baked, BrickDefinition};
use crate::build::tenscript::{FaceAlias, TenscriptError};

#[derive(Clone, Debug)]
pub struct BrickLibrary {
    pub brick_definitions: Vec<BrickDefinition>,
    pub baked_bricks: Vec<(FaceAlias, Baked)>,
}

impl BrickLibrary {
    /// Create BrickLibrary from Rust DSL brick definitions
    ///
    /// This uses type-safe Rust builders instead of parsing tenscript.
    ///
    /// # Example
    /// ```
    /// use tensegrity_lab::build::brick_builders::build_brick_library;
    /// use tensegrity_lab::build::tenscript::brick_library::BrickLibrary;
    ///
    /// let brick_library = BrickLibrary::from_rust(build_brick_library());
    /// ```
    pub fn from_rust(brick_definitions: Vec<BrickDefinition>) -> Self {
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
    fn compute_baked_bricks(brick_definitions: &[BrickDefinition]) -> Vec<(FaceAlias, Baked)> {
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

    pub fn new_brick(&self, search_alias: &FaceAlias) -> Result<Baked, TenscriptError> {
        let search_with_base = search_alias.with_base();
        let (_, baked) = self
            .baked_bricks
            .iter()
            .filter(|(baked_alias, _)| search_with_base.matches(baked_alias))
            .min_by_key(|(brick_alias, _)| brick_alias.0.len())
            .ok_or(TenscriptError::FaceAliasError(format!(
                "Cannot find a face to match {:?}",
                search_with_base
            )))?;
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
