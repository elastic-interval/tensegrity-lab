//! The census: every brick of a class, up to its threefold symmetry, baked
//! and ranked by the rules (docs/brick-evolution.md).
//!
//! A class is a set of struts placed as the prototypes place them, in
//! groups of three along X, Y and Z, so one turn of the symmetry carries a
//! strut along X to its partner along Y, Y to Z, and Z to X. A candidate is
//! a choice of face orbits and pull orbits under that turn.

use super::bake;
use super::measure::{measure, Measures};
use crate::build::dsl::brick::{Axis, BrickPrototype, FaceDef, PullDef};
use crate::build::dsl::brick_dsl::{BrickName, JointName};
use crate::build::dsl::Spin;
use std::collections::BTreeSet;

pub type Face = [usize; 3];
pub type Pull = (usize, usize);

pub struct Class {
    pub name: &'static str,
    /// Strut end names, alpha then omega, in groups of three along X, Y, Z.
    pub struts: Vec<(JointName, JointName)>,
    pub push_ideal: f32,
    pub initial_scale: f32,
    pub max_face_orbits: usize,
    pub max_pull_orbits: usize,
    /// Pull rest length over push rest length, tried for each topology with pulls.
    pub pull_ratios: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Topology {
    pub faces: BTreeSet<Face>,
    pub pulls: BTreeSet<Pull>,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub topology: Topology,
    pub pull_ratio: f32,
    pub measures: Measures,
}

impl Entry {
    pub fn members(&self) -> usize {
        self.topology.faces.len() * 3 + self.topology.pulls.len()
    }
}

pub struct Census {
    pub tried: usize,
    /// Viable entries, most uniformly stressed first, fewer members first among equals.
    pub viable: Vec<Entry>,
}

impl Class {
    fn strut_count(&self) -> usize {
        self.struts.len()
    }

    fn turn_end(&self, end: usize) -> usize {
        let strut = end / 2;
        let turned = 3 * (strut / 3) + (strut % 3 + 1) % 3;
        2 * turned + end % 2
    }

    fn face_orbits(&self) -> Vec<Vec<Face>> {
        let ends = 2 * self.strut_count();
        let canonical = |mut f: Face| {
            f.sort();
            f
        };
        let mut orbits: Vec<Vec<Face>> = vec![];
        let mut seen = BTreeSet::new();
        for a in 0..ends {
            for b in a + 1..ends {
                for c in b + 1..ends {
                    let struts: BTreeSet<_> = [a / 2, b / 2, c / 2].into();
                    if struts.len() < 3 || seen.contains(&[a, b, c]) {
                        continue;
                    }
                    let mut orbit = vec![];
                    let mut face = [a, b, c];
                    for _ in 0..3 {
                        if !orbit.contains(&face) {
                            orbit.push(face);
                            seen.insert(face);
                        }
                        face = canonical(face.map(|e| self.turn_end(e)));
                    }
                    orbits.push(orbit);
                }
            }
        }
        orbits
    }

    fn pull_orbits(&self) -> Vec<Vec<Pull>> {
        let ends = 2 * self.strut_count();
        let canonical = |(a, b): Pull| (a.min(b), a.max(b));
        let mut orbits: Vec<Vec<Pull>> = vec![];
        let mut seen = BTreeSet::new();
        for a in 0..ends {
            for b in a + 1..ends {
                if a / 2 == b / 2 || seen.contains(&(a, b)) {
                    continue;
                }
                let mut orbit = vec![];
                let mut pull = (a, b);
                for _ in 0..3 {
                    if !orbit.contains(&pull) {
                        orbit.push(pull);
                        seen.insert(pull);
                    }
                    pull = canonical((self.turn_end(pull.0), self.turn_end(pull.1)));
                }
                orbits.push(orbit);
            }
        }
        orbits
    }

    /// The topology of a prototype whose struts are this class's.
    pub fn topology_of(&self, proto: &BrickPrototype) -> Topology {
        let index = |name: JointName| {
            self.struts
                .iter()
                .enumerate()
                .find_map(|(s, (alpha, omega))| {
                    (*alpha == name).then_some(2 * s).or((*omega == name).then_some(2 * s + 1))
                })
                .unwrap()
        };
        Topology {
            faces: proto
                .faces
                .iter()
                .map(|f| {
                    let mut face = f.joints.map(index);
                    face.sort();
                    face
                })
                .collect(),
            pulls: proto
                .pulls
                .iter()
                .map(|p| {
                    let (a, b) = (index(p.alpha), index(p.omega));
                    (a.min(b), a.max(b))
                })
                .collect(),
        }
    }

    pub fn prototype(&self, topology: &Topology, pull_ratio: f32) -> BrickPrototype {
        let name = |end: usize| {
            let (alpha, omega) = self.struts[end / 2];
            if end % 2 == 0 { alpha } else { omega }
        };
        let axes = [Axis::X, Axis::Y, Axis::Z];
        BrickPrototype {
            brick_name: BrickName::OmniSymmetrical,
            brick_roles: vec![],
            scale_modes: vec![],
            joints: vec![],
            pushes: self
                .struts
                .iter()
                .enumerate()
                .map(|(s, (alpha, omega))| axes[s % 3].push(self.push_ideal, *alpha, *omega))
                .collect(),
            pulls: topology
                .pulls
                .iter()
                .map(|&(a, b)| PullDef {
                    alpha: name(a),
                    omega: name(b),
                    ideal: self.push_ideal * pull_ratio,
                    material: "pull".to_string(),
                })
                .collect(),
            faces: topology
                .faces
                .iter()
                .map(|f| FaceDef {
                    spin: Spin::Left,
                    joints: f.map(name),
                    aliases: vec![],
                    scale_overrides: vec![],
                })
                .collect(),
            cyclic_axes: vec![],
            face_twists: vec![],
        }
    }

    /// Every candidate: at least one face orbit, and every strut end held
    /// by a face or a pull.
    pub fn candidates(&self) -> Vec<Topology> {
        let face_orbits = self.face_orbits();
        let pull_orbits = self.pull_orbits();
        let ends = 2 * self.strut_count();
        let mut candidates = vec![];
        for face_choice in combinations(face_orbits.len(), 1, self.max_face_orbits) {
            for pull_choice in combinations(pull_orbits.len(), 0, self.max_pull_orbits) {
                let faces: BTreeSet<Face> =
                    face_choice.iter().flat_map(|&o| face_orbits[o].iter().copied()).collect();
                let pulls: BTreeSet<Pull> =
                    pull_choice.iter().flat_map(|&o| pull_orbits[o].iter().copied()).collect();
                let held: BTreeSet<usize> = faces
                    .iter()
                    .flat_map(|f| f.iter().copied())
                    .chain(pulls.iter().flat_map(|&(a, b)| [a, b]))
                    .collect();
                if held.len() == ends {
                    candidates.push(Topology { faces, pulls });
                }
            }
        }
        candidates
    }

    /// Bake every candidate, keep the viable, and rank them.
    pub fn census(&self) -> Census {
        let candidates = self.candidates();
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        let chunk = candidates.len().div_ceil(threads).max(1);
        let results: Vec<Option<Entry>> = std::thread::scope(|scope| {
            let handles: Vec<_> = candidates
                .chunks(chunk)
                .map(|part| scope.spawn(move || part.iter().map(|t| self.best_ratio(t)).collect::<Vec<_>>()))
                .collect();
            handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut viable: Vec<Entry> = results.into_iter().flatten().collect();
        viable.sort_by(|a, b| {
            a.measures
                .strain_spread
                .partial_cmp(&b.measures.strain_spread)
                .unwrap()
                .then(a.members().cmp(&b.members()))
        });
        Census { tried: candidates.len(), viable }
    }

    /// The candidate at its most uniformly stressed viable pull ratio, if any.
    fn best_ratio(&self, topology: &Topology) -> Option<Entry> {
        let ratios = if topology.pulls.is_empty() { &[1.0][..] } else { &self.pull_ratios[..] };
        ratios
            .iter()
            .filter_map(|&pull_ratio| {
                let (bake, _) = bake(&self.prototype(topology, pull_ratio), self.initial_scale);
                let measures = measure(&bake);
                measures.viable().then(|| Entry { topology: topology.clone(), pull_ratio, measures })
            })
            .min_by(|a, b| a.measures.strain_spread.partial_cmp(&b.measures.strain_spread).unwrap())
    }

    /// Whether two topologies are the same up to renaming the struts and
    /// swapping their ends.
    pub fn isomorphic(&self, a: &Topology, b: &Topology) -> bool {
        if a.faces.len() != b.faces.len() || a.pulls.len() != b.pulls.len() {
            return false;
        }
        let n = self.strut_count();
        let mut order: Vec<usize> = (0..n).collect();
        loop {
            for flips in 0..(1usize << n) {
                let map = |end: usize| 2 * order[end / 2] + ((end % 2) ^ ((flips >> (end / 2)) & 1));
                let faces: BTreeSet<Face> = a
                    .faces
                    .iter()
                    .map(|f| {
                        let mut g = f.map(map);
                        g.sort();
                        g
                    })
                    .collect();
                let pulls: BTreeSet<Pull> = a
                    .pulls
                    .iter()
                    .map(|&(x, y)| (map(x).min(map(y)), map(x).max(map(y))))
                    .collect();
                if faces == b.faces && pulls == b.pulls {
                    return true;
                }
            }
            if !next_permutation(&mut order) {
                return false;
            }
        }
    }
}

/// All index subsets of `0..n` with between `min` and `max` members, in order.
fn combinations(n: usize, min: usize, max: usize) -> Vec<Vec<usize>> {
    fn extend(start: usize, n: usize, max: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>, min: usize) {
        if current.len() >= min {
            out.push(current.clone());
        }
        if current.len() == max {
            return;
        }
        for i in start..n {
            current.push(i);
            extend(i + 1, n, max, current, out, min);
            current.pop();
        }
    }
    let mut out = vec![];
    extend(0, n, max, &mut vec![], &mut out, min);
    out
}

fn next_permutation(v: &mut [usize]) -> bool {
    let Some(i) = (1..v.len()).rev().find(|&i| v[i - 1] < v[i]) else {
        return false;
    };
    let j = (i..v.len()).rev().find(|&j| v[j] > v[i - 1]).unwrap();
    v.swap(i - 1, j);
    v[i..].reverse();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::brick_library::{get_prototype, get_scale};
    use JointName::*;

    fn single() -> Class {
        Class {
            name: "3 struts, faces and pulls",
            struts: vec![(AlphaX, OmegaX), (AlphaY, OmegaY), (AlphaZ, OmegaZ)],
            push_ideal: 3.204,
            initial_scale: get_scale(BrickName::SingleTwistLeft),
            max_face_orbits: 4,
            max_pull_orbits: 4,
            pull_ratios: vec![0.45, 0.5, 0.55, 0.6, 0.624, 0.65],
        }
    }

    fn omni() -> Class {
        Class {
            name: "6 struts, faces only",
            struts: vec![
                (BotAlphaX, BotOmegaX),
                (BotAlphaY, BotOmegaY),
                (BotAlphaZ, BotOmegaZ),
                (TopAlphaX, TopOmegaX),
                (TopAlphaY, TopOmegaY),
                (TopAlphaZ, TopOmegaZ),
            ],
            push_ideal: 3.271,
            initial_scale: get_scale(BrickName::OmniSymmetrical),
            max_face_orbits: 4,
            max_pull_orbits: 0,
            pull_ratios: vec![],
        }
    }

    fn report(class: &Class, target: BrickName) {
        let wanted = class.topology_of(&get_prototype(target));
        let started = std::time::Instant::now();
        let census = class.census();
        eprintln!(
            "\n=== {}: {} tried, {} viable, {:.1?}",
            class.name,
            census.tried,
            census.viable.len(),
            started.elapsed()
        );
        let distinct = distinct(class, &census.viable, 6);
        for (i, entry) in distinct.iter().enumerate() {
            let m = &entry.measures;
            eprintln!(
                "  {}. {} faces {} pulls ratio {:.3}  strain± {:.5} stiff {:.0} clear {:.3} face± {:.4} sphere± {:.4}{}",
                i + 1,
                entry.topology.faces.len(),
                entry.topology.pulls.len(),
                entry.pull_ratio,
                m.strain_spread,
                m.stiffness,
                m.clearance,
                m.face_irregularity,
                m.sphere_spread,
                if class.isomorphic(&entry.topology, &wanted) { format!("  <- {target:?}") } else { String::new() }
            );
        }
    }

    /// The best `count` entries that are not relabellings of a better one.
    fn distinct<'a>(class: &Class, ranked: &'a [Entry], count: usize) -> Vec<&'a Entry> {
        let mut found: Vec<&Entry> = vec![];
        for entry in ranked {
            if found.len() == count {
                break;
            }
            if !found.iter().any(|f| class.isomorphic(&f.topology, &entry.topology)) {
                found.push(entry);
            }
        }
        found
    }

    /// The current brick wins its class, and the runner-up is clearly worse.
    fn appears(class: &Class, target: BrickName) {
        let wanted = class.topology_of(&get_prototype(target));
        let census = class.census();
        let top = distinct(class, &census.viable, 2);
        assert!(class.isomorphic(&top[0].topology, &wanted), "{target:?} is not first");
        let (winner, rival) = (top[0].measures.strain_spread, top[1].measures.strain_spread);
        assert!(rival > 10.0 * winner.max(1.0e-4), "{target:?}: rival {rival} too close to {winner}");
    }

    #[test]
    fn single_appears() {
        appears(&single(), BrickName::SingleTwistLeft);
    }

    #[test]
    #[ignore]
    fn omni_appears() {
        appears(&omni(), BrickName::OmniSymmetrical);
    }

    /// Single's topology across pull ratios: does any measure pick today's 0.624?
    #[test]
    #[ignore]
    fn single_across_pull_ratios() {
        let class = single();
        let topology = class.topology_of(&get_prototype(BrickName::SingleTwistLeft));
        eprintln!();
        for step in 0..=30 {
            let ratio = 0.40 + 0.01 * step as f32;
            let (bake, _) = bake(&class.prototype(&topology, ratio), class.initial_scale);
            let m = measure(&bake);
            eprintln!(
                "  ratio {ratio:.2}  viable {:<5} face strain {:.3} strain± {:.5} stiff {:.0} clear {:.3}",
                m.viable(),
                m.face_strain,
                m.strain_spread,
                m.stiffness,
                m.clearance
            );
        }
    }

    #[test]
    fn current_topologies_are_candidates() {
        for (class, target) in [(single(), BrickName::SingleTwistLeft), (omni(), BrickName::OmniSymmetrical)] {
            let wanted = class.topology_of(&get_prototype(target));
            assert!(class.candidates().contains(&wanted), "{target:?}");
        }
    }

    #[test]
    #[ignore]
    fn census_of_single_and_omni() {
        report(&single(), BrickName::SingleTwistLeft);
        report(&omni(), BrickName::OmniSymmetrical);
    }
}
