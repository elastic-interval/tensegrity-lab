//! Bricks regenerated at startup. Each brick is baked once on first
//! access (a few ms) and cached. Mirroring (when needed for an
//! `OnSpin(Right)` attach onto a Left-native brick like SingleTwist,
//! or vice versa) happens in `get_brick` via `BakedBrick::mirror`.

use crate::build::dsl::brick::BakedBrick;
use crate::build::dsl::brick_dsl::{
    BrickName, BrickParams, OmniParams, SingleParams, TorqueParams, PHI,
};
use crate::build::dsl::brick_library::equilibrium::bake_brick_pure;
use glam::Vec3;
use std::collections::HashMap;
use std::sync::OnceLock;

static BAKED_CACHE: OnceLock<HashMap<BrickName, BakedBrick>> = OnceLock::new();

pub fn get_baked_brick(brick_name: BrickName) -> BakedBrick {
    BAKED_CACHE
        .get_or_init(populate_cache)
        .get(&brick_name)
        .cloned()
        .unwrap_or_else(|| panic!("no baked brick for {brick_name:?}"))
}

fn populate_cache() -> HashMap<BrickName, BakedBrick> {
    use BrickName::*;
    let mut cache = HashMap::new();
    for &brick_name in &[
        OmniSymmetrical,
        OmniTetrahedral,
        SingleTwistLeft,
        TorqueSymmetrical,
    ] {
        let baked = bake_brick_pure(
            brick_name,
            initial_scale(brick_name),
            brick_params(brick_name),
        );
        cache.insert(brick_name, baked);
    }
    cache
}

/// Initial guess for the bisection — refined to the strain target by `bake_brick_pure`.
fn initial_scale(brick_name: BrickName) -> f32 {
    use BrickName::*;
    match brick_name {
        OmniSymmetrical => 0.96720,
        OmniTetrahedral => 1.26953,
        SingleTwistLeft => 0.90909,
        TorqueSymmetrical => 0.97848,
    }
}

fn brick_params(brick_name: BrickName) -> BrickParams {
    use BrickName::*;
    match brick_name {
        OmniSymmetrical | OmniTetrahedral => BrickParams::Omni(OmniParams {
            push_lengths: Vec3::new(3.271, 3.271, 3.271),
        }),
        SingleTwistLeft => BrickParams::SingleLeft(SingleParams {
            push_lengths: Vec3::new(3.204, 3.204, 3.204),
            pull_length: 2.0,
        }),
        TorqueSymmetrical => BrickParams::Torque(TorqueParams {
            // Strut lengths relate by the golden mean: unit struts (at the
            // 3.0 base the face-radial rest lengths anchor) and a φ pair —
            // the z pair (TopLeft-TopRight / BottomLeft-BottomRight), down
            // from the original 2× that dominated the brick. NB the base is
            // NOT arbitrary: face radials have fixed absolute rests, and
            // push rests far below the face-anchored size leave every strut
            // slack (a degenerate bake that ignores these numbers entirely).
            push_lengths: Vec3::new(3.0, 3.0, 3.0 * PHI),
            pull_length: 1.86,
        }),
    }
}
