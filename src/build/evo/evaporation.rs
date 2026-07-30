//! Evaporation evolution — deterministic tensegrity genesis from a strut cloud.
//!
//! The process: scatter randomly oriented struts in a space (seeded PRNG,
//! roughly double the count that will survive) → **strut evaporation**
//! (while any two struts approach closer than the clearance anywhere along
//! their lengths, one of the pair evaporates) → candidate cables between
//! nearby strut ends (locality) → **shrink-wrap** (every member's rest
//! length derived from its as-scattered length at the designed pretension
//! band) → settle under gravity on the surface → evolve by **cable
//! evaporation** (offspring = parent minus one cable).
//!
//! The genome is `(seed, cable bitmask)` and nothing else. Everything —
//! scatter, evaporation coin flips, candidate ordering, physics settling —
//! is deterministic (ChaCha8 streams, index-tie-broken orderings, no
//! hash-order dependence, single-threaded CPU physics), so a genome fully
//! reproduces its tensegrity. Determinism holds per code version: changing
//! the algorithm re-maps what a seed means.
//!
//! Fitness: height of the highest joint divided by the number of pull
//! intervals (build tall, spend few cables). A structure whose struts end
//! up closer than the clearance after settling has failed and is removed
//! from the population.

use crate::connector::attachment::segment_segment_distance;
use crate::fabric::interval::Role;
use crate::fabric::physics::presets::{PRETENSING, SETTLING};
use crate::fabric::physics::{Surface, SurfaceCharacter};
use crate::fabric::{Fabric, JointKey};
use crate::Age;
use glam::Vec3;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct EvaporationParams {
    /// Struts scattered initially — roughly double what survives evaporation.
    pub strut_count: usize,
    pub strut_length: f32,
    /// Radius of the sphere the strut centres scatter within.
    pub space_radius: f32,
    /// No two struts may be closer than this anywhere along their lengths —
    /// enforced by evaporation at genesis and as the failure criterion after
    /// settling.
    pub clearance: f32,
    /// Candidate cables per strut end, to its nearest other-strut ends.
    pub cable_neighbors: usize,
    /// Shrink-wrap band: pulls end this fraction stretched, pushes this
    /// fraction compressed, relative to their as-scattered lengths.
    pub pretension: f32,
    /// Fabric-time settling duration before evaluation.
    pub settle_seconds: f32,
    /// Failure threshold as a fraction of the genesis clearance: settling
    /// under pretension legitimately tightens the cloud, so a settled
    /// structure fails only below `clearance × fail_fraction`.
    pub fail_fraction: f32,
}

impl Default for EvaporationParams {
    fn default() -> Self {
        Self {
            strut_count: 24,
            strut_length: 3.0,
            space_radius: 4.0,
            clearance: 0.3,
            cable_neighbors: 4,
            pretension: 0.02,
            settle_seconds: 4.0,
            fail_fraction: 0.5,
        }
    }
}

/// The hereditary information: a PRNG seed that deterministically grows the
/// strut set and its candidate cables, plus one bit per candidate cable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaporationGenome {
    pub seed: u64,
    pub cable_mask: Vec<bool>,
}

impl EvaporationGenome {
    /// The founder genome: all candidate cables present.
    pub fn founder(seed: u64, params: &EvaporationParams) -> Self {
        let struts = grow_struts(seed, params);
        let cables = candidate_cables(&struts, params);
        Self {
            seed,
            cable_mask: vec![true; cables.len()],
        }
    }

    pub fn surviving_cable_count(&self) -> usize {
        self.cable_mask.iter().filter(|&&bit| bit).count()
    }

    /// Offspring: identical except one surviving cable has evaporated.
    pub fn child_without(&self, surviving_index: usize) -> Self {
        let mut mask = self.cable_mask.clone();
        let mut seen = 0;
        for bit in mask.iter_mut() {
            if *bit {
                if seen == surviving_index {
                    *bit = false;
                    break;
                }
                seen += 1;
            }
        }
        Self {
            seed: self.seed,
            cable_mask: mask,
        }
    }
}

/// Scatter then evaporate: the deterministic compression component.
/// Returns struts as `(alpha, omega)` endpoint pairs.
pub fn grow_struts(seed: u64, params: &EvaporationParams) -> Vec<(Vec3, Vec3)> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut struts = scatter(&mut rng, params);
    evaporate(&mut struts, &mut rng, params.clearance);
    struts
}

fn scatter(rng: &mut ChaCha8Rng, params: &EvaporationParams) -> Vec<(Vec3, Vec3)> {
    let mut struts = Vec::with_capacity(params.strut_count);
    for _ in 0..params.strut_count {
        let center = point_in_unit_ball(rng) * params.space_radius;
        let direction = direction_on_unit_sphere(rng);
        let half = direction * (params.strut_length / 2.0);
        struts.push((center - half, center + half));
    }
    struts
}

fn point_in_unit_ball(rng: &mut ChaCha8Rng) -> Vec3 {
    loop {
        let v = Vec3::new(
            rng.gen_range(-1.0f32..1.0),
            rng.gen_range(-1.0f32..1.0),
            rng.gen_range(-1.0f32..1.0),
        );
        if v.length_squared() <= 1.0 {
            return v;
        }
    }
}

fn direction_on_unit_sphere(rng: &mut ChaCha8Rng) -> Vec3 {
    loop {
        let v = point_in_unit_ball(rng);
        let len = v.length();
        if len > 1.0e-3 {
            return v / len;
        }
    }
}

/// While any two struts approach closer than the clearance, one of the
/// closest pair evaporates (seeded coin flip). Always processes the global
/// closest pair first, ties broken by index — deterministic.
fn evaporate(struts: &mut Vec<(Vec3, Vec3)>, rng: &mut ChaCha8Rng, clearance: f32) {
    loop {
        let mut closest: Option<(usize, usize, f32)> = None;
        for i in 0..struts.len() {
            for j in (i + 1)..struts.len() {
                let d = segment_segment_distance(
                    struts[i].0,
                    struts[i].1,
                    struts[j].0,
                    struts[j].1,
                );
                let better = match closest {
                    None => true,
                    Some((_, _, best)) => d < best,
                };
                if better {
                    closest = Some((i, j, d));
                }
            }
        }
        match closest {
            Some((i, j, d)) if d < clearance => {
                let victim = if rng.gen_bool(0.5) { i } else { j };
                struts.remove(victim);
            }
            _ => return,
        }
    }
}

/// Candidate pull intervals: each strut end connects toward its k nearest
/// ends of *other* struts. Pairs are unordered, deduplicated, and returned
/// in canonical (sorted) order — the bitmask indexes this list. End index
/// convention: `2 × strut + {0 alpha, 1 omega}`.
pub fn candidate_cables(
    struts: &[(Vec3, Vec3)],
    params: &EvaporationParams,
) -> Vec<(usize, usize)> {
    let ends: Vec<Vec3> = struts
        .iter()
        .flat_map(|(alpha, omega)| [*alpha, *omega])
        .collect();
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (e, end) in ends.iter().enumerate() {
        let mut others: Vec<(usize, f32)> = ends
            .iter()
            .enumerate()
            .filter(|(o, _)| *o / 2 != e / 2)
            .map(|(o, p)| (o, (*p - *end).length()))
            .collect();
        others.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        for (o, _) in others.into_iter().take(params.cable_neighbors) {
            pairs.insert((e.min(o), e.max(o)));
        }
    }
    pairs.into_iter().collect()
}

/// Express a genome: regrow the struts from the seed, apply the cable mask,
/// shrink-wrap every member at the pretension band, and settle under
/// gravity on the frozen surface. Fully deterministic.
pub fn express(genome: &EvaporationGenome, params: &EvaporationParams) -> Fabric {
    let struts = grow_struts(genome.seed, params);
    let cables = candidate_cables(&struts, params);
    assert_eq!(
        cables.len(),
        genome.cable_mask.len(),
        "cable mask length must match the seed's candidate count"
    );

    let mut fabric = Fabric::new(format!("Evaporation {}", genome.seed));
    let mut end_keys: Vec<JointKey> = Vec::with_capacity(struts.len() * 2);
    for (alpha, omega) in &struts {
        end_keys.push(fabric.create_joint(*alpha));
        end_keys.push(fabric.create_joint(*omega));
    }
    for s in 0..struts.len() {
        fabric.create_strained_interval(
            end_keys[2 * s],
            end_keys[2 * s + 1],
            Role::Pushing,
            -params.pretension,
        );
    }
    for (cable, &kept) in cables.iter().zip(&genome.cable_mask) {
        if kept {
            fabric.create_strained_interval(
                end_keys[cable.0],
                end_keys[cable.1],
                Role::Pulling,
                params.pretension,
            );
        }
    }

    // Stage 1 — form-finding: settle in zero-G so the cable web organises
    // the cloud, free of gravity.
    let iterations = (params.settle_seconds * Age::iterations_per_second()) as usize;
    for _ in 0..iterations {
        fabric.iterate(&PRETENSING);
    }
    fabric.zero_velocities();

    // Stage 2 — the shrink-wrap proper: pin every member's rest length to
    // the pretension band on the organised geometry (as the brick baker
    // does). Deliberately NO re-settle here: without the face-radial anchors
    // bricks have, wrap-and-resettle cycles contract monotonically until
    // struts collide.
    rewrap(&mut fabric, params.pretension);

    // Stage 3 — reality: drop it just above the surface and settle under
    // gravity. Height fitness and the clearance failure test read this state.
    let translation = fabric.centralize_translation(Some(0.2));
    fabric.apply_translation(translation);
    let mut physics = SETTLING.clone();
    physics.surface = Some(Surface::new(SurfaceCharacter::Frozen, 1.0));
    for _ in 0..iterations {
        fabric.iterate(&physics);
    }
    fabric
}

/// Set every member's rest length so its current length sits exactly at the
/// pretension band: pulls stretched by `t`, pushes compressed by `t`.
fn rewrap(fabric: &mut Fabric, pretension: f32) {
    use crate::fabric::interval::Span;
    use crate::units::Meters;
    let lengths: Vec<(crate::fabric::IntervalKey, f32, bool)> = fabric
        .intervals
        .iter()
        .map(|(key, interval)| {
            let (alpha, omega) = interval.locations(&fabric.joints);
            (key, (omega - alpha).length(), interval.has_role(Role::Pushing))
        })
        .collect();
    for (key, length, is_push) in lengths {
        let rest = if is_push {
            length / (1.0 - pretension)
        } else {
            length / (1.0 + pretension)
        };
        fabric.intervals[key].span = Span::Fixed {
            length: Meters(rest),
        };
    }
}

#[derive(Clone, Debug)]
pub struct Evaluation {
    pub height: f32,
    pub pull_count: usize,
    pub min_strut_clearance: f32,
    pub failed: bool,
    pub fitness: f32,
}

/// Fitness: height of the highest joint per pull interval. A settled
/// structure whose struts approach closer than the clearance has failed.
pub fn evaluate(fabric: &Fabric, params: &EvaporationParams) -> Evaluation {
    let pushes: Vec<(Vec3, Vec3)> = fabric
        .interval_values()
        .filter(|interval| interval.has_role(Role::Pushing))
        .map(|interval| interval.locations(&fabric.joints))
        .collect();
    let mut min_clearance = f32::MAX;
    for i in 0..pushes.len() {
        for j in (i + 1)..pushes.len() {
            let d = segment_segment_distance(pushes[i].0, pushes[i].1, pushes[j].0, pushes[j].1);
            min_clearance = min_clearance.min(d);
        }
    }
    let pull_count = fabric
        .interval_values()
        .filter(|interval| interval.has_role(Role::Pulling))
        .count();
    let height = fabric
        .joints
        .values()
        .map(|joint| joint.location.y)
        .fold(0.0f32, f32::max);
    let failed = min_clearance < params.clearance * params.fail_fraction || pull_count == 0;
    let fitness = if failed {
        0.0
    } else {
        height / pull_count as f32
    };
    Evaluation {
        height,
        pull_count,
        min_strut_clearance: min_clearance,
        failed,
        fitness,
    }
}

/// Run the evolution: founder plus generations of cable evaporation.
/// Deterministic for a given `(master_seed, params, generations, …)`.
/// Returns the surviving population, best first.
pub fn evolve(
    master_seed: u64,
    params: &EvaporationParams,
    generations: usize,
    population_cap: usize,
    children_per_parent: usize,
) -> Vec<(EvaporationGenome, Evaluation)> {
    let mut rng = ChaCha8Rng::seed_from_u64(master_seed ^ 0x5EED_CAB1E);
    let founder = EvaporationGenome::founder(master_seed, params);
    let founder_eval = evaluate(&express(&founder, params), params);
    let mut population: Vec<(EvaporationGenome, Evaluation)> = vec![(founder, founder_eval)];
    let mut seen_masks: BTreeSet<Vec<bool>> = population
        .iter()
        .map(|(genome, _)| genome.cable_mask.clone())
        .collect();

    for _ in 0..generations {
        let mut offspring: Vec<(EvaporationGenome, Evaluation)> = Vec::new();
        for (parent, _) in &population {
            let survivors = parent.surviving_cable_count();
            if survivors == 0 {
                continue;
            }
            for _ in 0..children_per_parent {
                let child = parent.child_without(rng.gen_range(0..survivors));
                if !seen_masks.insert(child.cable_mask.clone()) {
                    continue;
                }
                let evaluation = evaluate(&express(&child, params), params);
                if !evaluation.failed {
                    offspring.push((child, evaluation));
                }
            }
        }
        population.extend(offspring);
        population.sort_by(|a, b| {
            b.1.fitness
                .partial_cmp(&a.1.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cable_mask.cmp(&b.0.cable_mask))
        });
        population.truncate(population_cap);
    }
    population
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_params() -> EvaporationParams {
        EvaporationParams {
            strut_count: 12,
            settle_seconds: 1.0,
            ..EvaporationParams::default()
        }
    }

    /// Same seed → bitwise-identical struts and candidate cables.
    #[test]
    fn growth_is_deterministic() {
        let params = small_params();
        let a = grow_struts(42, &params);
        let b = grow_struts(42, &params);
        assert_eq!(a.len(), b.len());
        for (sa, sb) in a.iter().zip(&b) {
            assert_eq!(sa.0.to_array().map(f32::to_bits), sb.0.to_array().map(f32::to_bits));
            assert_eq!(sa.1.to_array().map(f32::to_bits), sb.1.to_array().map(f32::to_bits));
        }
        assert_eq!(candidate_cables(&a, &params), candidate_cables(&b, &params));
    }

    /// After evaporation, no strut pair is closer than the clearance.
    #[test]
    fn evaporation_respects_clearance() {
        let params = small_params();
        for seed in [1u64, 7, 42] {
            let struts = grow_struts(seed, &params);
            assert!(struts.len() >= 2, "seed {seed}: everything evaporated");
            for i in 0..struts.len() {
                for j in (i + 1)..struts.len() {
                    let d = segment_segment_distance(
                        struts[i].0,
                        struts[i].1,
                        struts[j].0,
                        struts[j].1,
                    );
                    assert!(
                        d >= params.clearance,
                        "seed {seed}: struts {i},{j} at {d:.3} < clearance"
                    );
                }
            }
        }
    }

    /// Same genome → bitwise-identical settled fabric: the full pipeline,
    /// physics included, is reproducible from (seed, mask) alone.
    #[test]
    fn expression_is_deterministic() {
        let params = small_params();
        let genome = EvaporationGenome::founder(42, &params);
        let a = express(&genome, &params);
        let b = express(&genome, &params);
        assert_eq!(a.joints.len(), b.joints.len());
        for (ja, jb) in a.joints.values().zip(b.joints.values()) {
            assert_eq!(
                ja.location.to_array().map(f32::to_bits),
                jb.location.to_array().map(f32::to_bits),
                "settled joint positions must be bitwise identical"
            );
        }
    }

    /// A few generations of cable evaporation run to completion. NB random
    /// founders rarely survive the gravity landing yet (rigidity of the
    /// k-nearest web is the open tuning question — see module docs); this
    /// exercises the loop's mechanics, not survival.
    #[test]
    fn evolution_smoke() {
        let params = small_params();
        let population = evolve(42, &params, 2, 6, 2);
        assert!(!population.is_empty(), "population vanished entirely");
        let (best, eval) = &population[0];
        eprintln!(
            "best: seed {} cables {}/{} height {:.2}m clear {:.3} failed {} fitness {:.4}",
            best.seed,
            best.surviving_cable_count(),
            best.cable_mask.len(),
            eval.height,
            eval.min_strut_clearance,
            eval.failed,
            eval.fitness,
        );
    }
}
