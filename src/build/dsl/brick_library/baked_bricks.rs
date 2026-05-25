//! Baked bricks — regenerated at startup, not hardcoded.
//!
//! Each non-derived brick is baked once on first access via
//! `oven::bake_brick_to_baked` and cached. The bake takes a few ms per
//! brick (Verlet physics + reorientation + symmetrise), well below the
//! threshold of perception even on cold startup.
//!
//! What's kept here:
//! - The brick's **design parameters** (push/pull lengths). These are
//!   intentional design choices, not bake outputs.
//! - The brick's **initial scale estimate**. Strain bisection finds the
//!   final scale; a good initial estimate makes it converge in one
//!   iteration. These values were the result of past bake runs; they
//!   are the smallest piece of "discovered" state we keep.
//!
//! What's gone:
//! - The 200+ lines of hardcoded joint/interval literals that used to
//!   live here. They're now a deterministic function of the prototype
//!   plus a few ms of CPU.

use crate::build::dsl::brick::BakedBrick;
use crate::build::dsl::brick_dsl::{
    BrickName, BrickParams, OmniParams, SingleParams, TorqueParams,
};
use crate::build::oven::bake_brick_to_baked;
use glam::Vec3;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Cache: each non-derived brick baked once, reused thereafter.
/// `SingleTwistRight` is *derived* — never baked, always `.mirror()`-ed
/// from `SingleTwistLeft`.
static BAKED_CACHE: OnceLock<HashMap<BrickName, BakedBrick>> = OnceLock::new();

pub fn get_baked_brick(brick_name: BrickName) -> BakedBrick {
    use BrickName::*;
    if brick_name == SingleTwistRight {
        return get_baked_brick(SingleTwistLeft).mirror();
    }
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
        let baked = bake_brick_to_baked(
            brick_name,
            initial_scale(brick_name),
            brick_params(brick_name),
        );
        cache.insert(brick_name, baked);
    }
    cache
}

/// Initial scale estimate for each brick — strain bisection refines
/// from here. Values come from previous bake runs (one float per brick
/// is much smaller than the 60+ joint coordinates we used to ship).
fn initial_scale(brick_name: BrickName) -> f32 {
    use BrickName::*;
    match brick_name {
        OmniSymmetrical => 0.96720,
        OmniTetrahedral => 1.22593,
        SingleTwistLeft => 0.90909,
        TorqueSymmetrical => 1.02172,
        SingleTwistRight => unreachable!("derived; never baked directly"),
    }
}

/// Design parameters per brick — the same values used in
/// `brick_library/mod.rs` when constructing the prototype.
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
            push_lengths: Vec3::new(3.0, 3.0, 6.0),
            pull_length: 1.86,
        }),
        SingleTwistRight => unreachable!("derived; never baked directly"),
    }
}
