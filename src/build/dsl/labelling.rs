//! Joint-labelling glue between the DSL and the generic fabric layer.
//!
//! Implements a labeller that exposes a fabric's 3-fold rotational symmetry
//! directly in the joint names, with minimum-character labels (the
//! engineer-builder engraves these onto physical parts):
//!
//! - **Off-axis joints** form 3-orbits under the 120° rotation. Each label
//!   is `<leg><brick><position>` — three characters:
//!     - `leg` ∈ `{A, B, C}` distinguishes the three rotational copies.
//!     - `brick` is a single digit naming which sub-brick of the leg the
//!       joint belongs to. `0` = seed brick; `1..N` = column-bricks along
//!       the leg (1 = closest to seed); `N+1` = the leg-end prism if any.
//!     - `position` is the joint's 1-indexed position within that brick.
//!   Examples: `A01` (leg A, seed, position 1), `A14` (leg A, column 1,
//!   position 4), `A52` (leg A, leg prism position 2 — the foot of leg A
//!   in OpenClaw).
//! - **On-axis singletons** (joints fixed under the rotation) get the prefix
//!   `Z` and an integer index. Examples: `Z1` (topmost), `Z2`.
//! - **Non-structural joints** (face-centre midpoints that aren't endpoints
//!   of any push interval) return `None`; `joint_label` falls back to
//!   `JointPath::Display`.
//!
//! ## How orbits are determined
//!
//! **Orbit grouping is purely topological** — it uses each joint's
//! `JointPath` plus the seed brick's `cyclic_axes` declaration. No
//! floating-point geometry is consulted to decide which joints sit in the
//! same orbit, so the symmetry the labels expose is a property of the build
//! recipe, not of the converged numerical state.
//!
//! Three path shapes are recognised:
//!
//! 1. **Seed joint** (empty `branches`). The seed brick prototype maps each
//!    `local_index` to a `JointName` (e.g. `BotAlphaX`). For Omni-shaped
//!    bricks `JointName::omni_decode()` yields `(OmniCategory, Axis)`; the
//!    orbit is the category and the leg letter is `cyclic_axes`-position of
//!    the axis.
//! 2. **Leg joint** (`branches[0] ∈ {0, 1, 2}`). The first branch is the
//!    leg index (0→A, 1→B, 2→C) — this convention requires the DSL author
//!    to list the three cyclic faces first, in the same order as
//!    `cyclic_axes`. The orbit key is `(brick, local_index)` where `brick`
//!    is derived from the rest of the path (see below).
//! 3. **Axis singleton** (`branches[0] ≥ 3` or `branches[0] == PRISM_MARKER`).
//!    A joint built off an axis-fixed face (e.g. OpenClaw's apex prism on
//!    `OmniTop`). Each such joint is its own orbit; they get `Z<n>` labels.
//!
//! ## How brick and position are assigned
//!
//! Both come straight from the path:
//!
//! - **Brick** is the count of structural markers in the joint's branches
//!   past the leg letter (`COLUMN_MARKER`s + `PRISM_MARKER`s). Seed joints
//!   (empty branches) get brick `0`. For an OpenClaw leg built as
//!   `column(N).prism(...)`, column-brick `K` joints get brick `K` and
//!   leg-prism joints get brick `N+1`.
//! - **Position** is the 1-indexed within-brick rank. For seed joints
//!   that's the `OmniCategory` altitude rank (1 = TopOmega … 4 = BotAlpha,
//!   based on both Omega ends sitting above both Alpha ends). For column
//!   and leg-prism joints it's `local_index + 1` (the prototype's
//!   oven-creation order).
//!
//! Axis singletons sort by `(branches lex, ¬local_index)`. The bit-not on
//! `local_index` puts an upward-facing prism's omega end (local 1, the
//! outer/far end) at `Z1` and alpha (local 0) at `Z2`.
//!
//! The whole label is a deterministic function of the joint's `JointPath`
//! plus the seed brick's prototype declarations. Re-running the same plan
//! produces byte-identical labels; the converged physics state never
//! affects what a joint is called.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;

use crate::build::dsl::brick_dsl::{BrickName, BrickRole, JointName};
use crate::build::dsl::brick_library;
use crate::fabric::interval::Role;
use crate::fabric::joint_path::{JointPath, COLUMN_MARKER, PRISM_MARKER};
use crate::fabric::{Fabric, JointKey, JointLabeller};

const LEG_LETTERS: [char; 3] = ['A', 'B', 'C'];

/// `branches[0]` values reserved for the three legs of a 3-fold seed (the
/// first three entries of the seed brick's `.faces([...])` list). Above this
/// threshold the joint lives on an axis-fixed sub-structure and is treated
/// as a singleton.
const MAX_LEG_BRANCH: u8 = 2;

#[derive(Debug)]
pub struct SymmetricOrbitLabeller {
    seed: Option<SeedInfo>,
    cache: Mutex<Option<OrbitCache>>,
}

#[derive(Debug, Clone, Copy)]
struct SeedInfo {
    brick_name: BrickName,
    brick_role: BrickRole,
}

#[derive(Debug)]
struct OrbitCache {
    joint_count: usize,
    interval_count: usize,
    labels: HashMap<JointKey, String>,
}

impl SymmetricOrbitLabeller {
    /// Build a labeller that can decode seed-joint orbits from the given
    /// seed brick. Pass the same `(brick_name, brick_role)` that the root
    /// hub uses.
    pub fn new(brick_name: BrickName, brick_role: BrickRole) -> Self {
        Self {
            seed: Some(SeedInfo {
                brick_name,
                brick_role,
            }),
            cache: Mutex::new(None),
        }
    }

    /// Build a labeller without seed-brick info. Seed joints (empty paths)
    /// won't be labelled and will fall back to `JointPath::Display`. Useful
    /// only for fabrics whose seed wasn't a 3-fold brick.
    pub fn without_seed_info() -> Self {
        Self {
            seed: None,
            cache: Mutex::new(None),
        }
    }

    fn compute(&self, fabric: &Fabric) -> OrbitCache {
        let mut push_endpoints: HashSet<JointKey> = HashSet::new();
        for interval in fabric.intervals.values() {
            if interval.has_role(Role::Pushing) {
                push_endpoints.insert(interval.alpha_key);
                push_endpoints.insert(interval.omega_key);
            }
        }

        // For each structural joint compute its (orbit_id, role-in-orbit).
        // Topology only — no joint positions are consulted.
        let mut by_orbit: BTreeMap<OrbitId, Vec<(JointKey, OrbitRole)>> = BTreeMap::new();
        for &key in &push_endpoints {
            let Some(joint) = fabric.joints.get(key) else {
                continue;
            };
            if let Some((orbit_id, role)) = classify(&joint.path, &self.seed) {
                by_orbit.entry(orbit_id).or_default().push((key, role));
            }
        }

        // Each orbit's `OrbitId` already carries the brick+position it
        // contributes to its label — we don't need any global counter.
        let mut orbits: Vec<(OrbitId, Vec<(JointKey, OrbitRole)>)> = Vec::new();
        let mut singletons: Vec<JointKey> = Vec::new();
        for (id, members) in by_orbit {
            if members.len() == 1 && matches!(members[0].1, OrbitRole::Singleton) {
                singletons.push(members[0].0);
            } else {
                orbits.push((id, members));
            }
        }

        let mut labels: HashMap<JointKey, String> = HashMap::new();
        for (i, key) in singletons.iter().enumerate() {
            labels.insert(*key, format!("Z{}", i + 1));
        }
        for (orbit_id, members) in orbits.iter() {
            let (brick, position) = match orbit_id {
                OrbitId::Seed { position } => (0_u32, *position),
                OrbitId::Leg { brick, position } => (*brick, *position),
                OrbitId::Singleton { .. } => continue,
            };
            for (key, role) in members {
                let OrbitRole::Leg(leg) = role else { continue };
                labels.insert(*key, format!("{}{}{}", leg, brick, position));
            }
        }

        OrbitCache {
            joint_count: fabric.joints.len(),
            interval_count: fabric.intervals.len(),
            labels,
        }
    }
}

impl JointLabeller for SymmetricOrbitLabeller {
    fn label(&self, fabric: &Fabric, key: JointKey) -> Option<String> {
        let mut cache_guard = self.cache.lock().ok()?;
        let stale = match cache_guard.as_ref() {
            None => true,
            Some(cache) => {
                cache.joint_count != fabric.joints.len()
                    || cache.interval_count != fabric.intervals.len()
            }
        };
        if stale {
            *cache_guard = Some(self.compute(fabric));
        }
        cache_guard.as_ref()?.labels.get(&key).cloned()
    }
}

/// Canonical identifier of a rotational orbit, derived purely from a
/// joint's `JointPath` (plus the seed brick's prototype for seed joints).
/// Two joints in the same orbit produce the same `OrbitId`, and an
/// `OrbitId` already carries everything needed to format the joint's label
/// — `(brick, position)` for legs, `(category_rank → position)` for seed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum OrbitId {
    /// Seed joint orbit. `position` is the within-leg ranking 1..N derived
    /// from the omni category (1 = TopOmega, 2 = BotOmega, 3 = TopAlpha,
    /// 4 = BotAlpha for OmniSymmetrical under Seed(1)).
    Seed { position: u8 },
    /// Leg joint orbit. `brick` is the brick number along the leg (1 =
    /// column-brick closest to the seed, N+1 = leg-prism). `position` is
    /// `local_index + 1` within that brick.
    Leg { brick: u32, position: u8 },
    /// Axis singleton. Each singleton is its own orbit. The sort key
    /// `(branches, local_idx_desc)` puts an upward-facing prism's outer
    /// end first.
    Singleton {
        branches: Vec<u8>,
        local_idx_desc: u8,
    },
}

#[derive(Debug, Clone, Copy)]
enum OrbitRole {
    Leg(char),
    Singleton,
}

fn classify(path: &JointPath, seed: &Option<SeedInfo>) -> Option<(OrbitId, OrbitRole)> {
    if path.branches.is_empty() {
        let seed = seed.as_ref()?;
        let (category_rank, leg_letter) = decode_seed_joint(path.local_index, seed)?;
        let position = category_rank + 1;
        return Some((OrbitId::Seed { position }, OrbitRole::Leg(leg_letter)));
    }
    let head = path.branches[0];
    if head <= MAX_LEG_BRANCH {
        let leg_letter = LEG_LETTERS[head as usize];
        let brick = brick_number_from_sub_branches(&path.branches[1..]);
        return Some((
            OrbitId::Leg {
                brick,
                position: path.local_index + 1,
            },
            OrbitRole::Leg(leg_letter),
        ));
    }
    // head > MAX_LEG_BRANCH (face index ≥ 3) or head == PRISM_MARKER /
    // COLUMN_MARKER (markers attached without a leg). In a 3-fold-symmetric
    // fabric these joints must lie on the rotation axis, so they're
    // singletons.
    Some((
        OrbitId::Singleton {
            branches: path.branches.clone(),
            local_idx_desc: !path.local_index,
        },
        OrbitRole::Singleton,
    ))
}

/// Count of structural markers in the sub-branches (everything after the
/// leg letter). For an OpenClaw leg built as `column(N).prism(...)`, this
/// is exactly the brick number: `K` for column-brick `K`, `N+1` for the
/// leg-end prism. Assumes a linear leg topology — if a fabric ever grows
/// side branches off a leg this won't distinguish them and the labels for
/// the side branch will clash with the main column.
fn brick_number_from_sub_branches(sub_branches: &[u8]) -> u32 {
    sub_branches
        .iter()
        .filter(|&&b| b == COLUMN_MARKER || b == PRISM_MARKER)
        .count() as u32
}

/// Decode a seed-joint local index into (category_rank, leg_letter) for an
/// Omni-shaped seed brick. Returns `None` if the brick or local index isn't
/// Omni-decodable (then the joint falls back to `JointPath::Display`).
fn decode_seed_joint(local_index: u8, seed: &SeedInfo) -> Option<(u8, char)> {
    let proto = brick_library::get_prototype(seed.brick_name);
    let cyclic_axes = proto.cyclic_axes_for(seed.brick_role)?;
    if cyclic_axes.len() != 3 {
        return None;
    }
    let idx = local_index as usize;
    let explicit_n = proto.joints.len();
    let name: JointName = if idx < explicit_n {
        proto.joints[idx]
    } else {
        let push_offset = idx - explicit_n;
        let push_idx = push_offset / 2;
        let push = proto.pushes.get(push_idx)?;
        if push_offset % 2 == 0 {
            push.alpha
        } else {
            push.omega
        }
    };
    let (category, axis) = name.omni_decode()?;
    let pos = cyclic_axes.iter().position(|a| *a == axis)?;
    // OmniSymmetrical under Seed(1) altitude order (verified empirically):
    // TopOmega > BotOmega > TopAlpha > BotAlpha. (A "top" strut and a "bot"
    // strut interleave once oriented — the Omega ends are pushed *outward*
    // from the brick centre, so both Omega ends sit above both Alpha ends.)
    let category_rank = match category {
        crate::build::dsl::brick_dsl::OmniCategory::TopOmega => 0,
        crate::build::dsl::brick_dsl::OmniCategory::BotOmega => 1,
        crate::build::dsl::brick_dsl::OmniCategory::TopAlpha => 2,
        crate::build::dsl::brick_dsl::OmniCategory::BotAlpha => 3,
    };
    let leg_letter = LEG_LETTERS[pos];
    Some((category_rank, leg_letter))
}

// Silence unused warning when both PRISM_MARKER/COLUMN_MARKER aren't used.
const _: u8 = PRISM_MARKER;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
    use std::collections::HashSet;

    fn build_open_claw_to_slack() -> FabricPlanExecutor {
        let plan = fabric_library::get_fabric_plan(FabricName::OpenClaw);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        executor
    }

    fn parse_leg(label: &str) -> Option<(u8, u32)> {
        let bytes = label.as_bytes();
        if bytes.len() < 2 {
            return None;
        }
        if !matches!(bytes[0], b'A' | b'B' | b'C') {
            return None;
        }
        let n: u32 = std::str::from_utf8(&bytes[1..]).ok()?.parse().ok()?;
        Some((bytes[0], n))
    }

    fn parse_singleton(label: &str) -> Option<u32> {
        let bytes = label.as_bytes();
        if bytes.len() < 2 || bytes[0] != b'Z' {
            return None;
        }
        std::str::from_utf8(&bytes[1..]).ok()?.parse().ok()
    }

    /// Every push interval whose endpoints are both off-axis sits in a
    /// rotational triple. The three rotational partners must share the same
    /// numeric index at each end and use leg letters {A, B, C} exactly once.
    #[test]
    fn test_symmetric_orbit_labeller_rotates_legs() {
        let executor = build_open_claw_to_slack();
        let fabric = &executor.fabric;

        use std::collections::BTreeMap;
        let mut groups: BTreeMap<(u32, u32), Vec<(u8, u8)>> = BTreeMap::new();
        for (_, interval) in fabric.intervals.iter() {
            if !interval.has_role(crate::fabric::interval::Role::Pushing) {
                continue;
            }
            let a = fabric.joint_label(interval.alpha_key);
            let b = fabric.joint_label(interval.omega_key);
            let (Some((la, ia)), Some((lb, ib))) = (parse_leg(&a), parse_leg(&b)) else {
                continue;
            };
            if la != lb {
                continue;
            }
            let key = if ia <= ib { (ia, ib) } else { (ib, ia) };
            groups.entry(key).or_default().push((la, lb));
        }

        for ((ia, ib), members) in &groups {
            assert_eq!(
                members.len(),
                3,
                "push triple for indices ({ia},{ib}) should have 3 members, has {}",
                members.len()
            );
            let legs: HashSet<u8> = members.iter().map(|(la, _)| *la).collect();
            assert_eq!(
                legs.len(),
                3,
                "push triple for indices ({ia},{ib}) should use all three leg letters, got {:?}",
                legs
            );
            assert!(legs.contains(&b'A'));
            assert!(legs.contains(&b'B'));
            assert!(legs.contains(&b'C'));
        }
    }

    /// Every structural joint gets a unique label; every `(brick, position)`
    /// pair that appears with one leg letter appears with all three; axis
    /// singletons number contiguously from 1.
    #[test]
    fn test_symmetric_orbit_labeller_no_collisions() {
        let executor = build_open_claw_to_slack();
        let fabric = &executor.fabric;

        let mut push_endpoints: HashSet<JointKey> = HashSet::new();
        for interval in fabric.intervals.values() {
            if interval.has_role(crate::fabric::interval::Role::Pushing) {
                push_endpoints.insert(interval.alpha_key);
                push_endpoints.insert(interval.omega_key);
            }
        }

        // For leg labels we now parse <leg><brick><position> — `parse_leg`
        // returns (leg_byte, brick*10+position). Recover (brick, position).
        let mut labels: Vec<String> = Vec::new();
        let mut leg_pairs: std::collections::HashMap<(u32, u32), HashSet<u8>> =
            std::collections::HashMap::new();
        let mut singleton_indices: HashSet<u32> = HashSet::new();
        for key in push_endpoints.iter().copied() {
            let label = fabric.joint_label(key);
            assert!(
                !label.is_empty(),
                "structural joint {:?} got an empty label",
                key
            );
            if let Some((leg, idx)) = parse_leg(&label) {
                let brick = idx / 10;
                let position = idx % 10;
                leg_pairs.entry((brick, position)).or_default().insert(leg);
            } else if let Some(idx) = parse_singleton(&label) {
                singleton_indices.insert(idx);
            } else {
                panic!("unexpected label shape for structural joint: '{label}'");
            }
            labels.push(label);
        }
        let unique: HashSet<&String> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "label collisions: {} structural joints share {} unique labels",
            labels.len(),
            unique.len()
        );

        // Every (brick, position) that appears must have all three legs.
        for ((brick, position), legs) in &leg_pairs {
            assert_eq!(
                legs.len(),
                3,
                "(brick {brick}, position {position}) only has legs {legs:?}, expected all three"
            );
            assert!(legs.contains(&b'A'));
            assert!(legs.contains(&b'B'));
            assert!(legs.contains(&b'C'));
        }

        // Axis singletons number 1..N contiguously.
        for i in 1..=singleton_indices.len() as u32 {
            assert!(
                singleton_indices.contains(&i),
                "axis-singleton index {i} missing from labels (got {singleton_indices:?})"
            );
        }
    }

    /// A fabric with no labeller installed falls back to the `JointPath`
    /// `Display` impl — the existing behaviour for algorithmic fabrics
    /// (sphere, klein, mobius). The labeller wiring must not regress that.
    #[test]
    fn test_no_labeller_falls_back_to_joint_path() {
        use crate::fabric::Fabric;
        use glam::Vec3;
        let mut fabric = Fabric::new("plain".to_string());
        let key = fabric.create_joint(Vec3::ZERO);
        assert!(
            fabric.labeller.is_none(),
            "freshly-built Fabric has no labeller"
        );
        let label = fabric.joint_label(key);
        assert_eq!(label, "Z0");
    }

    /// With `without_seed_info`, seed joints (empty paths) get no label and
    /// fall back to `JointPath::Display`.
    #[test]
    fn test_orbit_labeller_without_seed_info_falls_back() {
        use crate::fabric::Fabric;
        use glam::Vec3;
        use std::sync::Arc;
        let mut fabric = Fabric::new("plain".to_string());
        let key = fabric.create_joint(Vec3::new(1.0, 2.0, 3.0));
        fabric.labeller = Some(Arc::new(SymmetricOrbitLabeller::without_seed_info()));
        let label = fabric.joint_label(key);
        assert_eq!(label, "Z0", "fallback to JointPath when no orbit info");
    }
}
