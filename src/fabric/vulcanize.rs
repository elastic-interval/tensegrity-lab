use std::collections::HashMap;

use cgmath::{InnerSpace, MetricSpace};
use itertools::Itertools;

use crate::fabric::interval::Role::{BowTie, Pushing};
use crate::fabric::interval::Span::Approaching;
use crate::fabric::interval::{Interval, Span};
use crate::fabric::joint_incident::{JointIncident, Path};
use crate::fabric::material::Material;
use crate::fabric::{Fabric, IntervalKey, JointKey, Joints};
use crate::units::Seconds;

const BOW_TIE_SHORTEN: f32 = 0.5;

impl Fabric {
    pub fn install_bow_ties(&mut self) {
        for Pair {
            alpha_key,
            omega_key,
            length,
        } in self.pair_generator().bow_tie_pulls(&self.joints)
        {
            self.create_interval(alpha_key, omega_key, length, BowTie);
        }
    }

    pub fn strain_limits(&self, target_material: Material) -> (f32, f32) {
        let choose_target =
            |&Interval {
                 strain, material, ..
             }| (material == target_material).then_some(strain);
        let max_strain = self
            .interval_values()
            .filter_map(choose_target)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0);
        let min_strain = self
            .interval_values()
            .filter_map(choose_target)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        (min_strain, max_strain)
    }

    pub fn _equalize_strain(&mut self, target_material: Material) {
        let mut total_strain = 0.0;
        let mut count = 0;
        for (_key, interval) in self.intervals.iter() {
            if interval.material == target_material {
                total_strain += interval.strain;
                count += 1;
            }
        }
        let average_strain = total_strain / (count as f32);
        for (_key, interval) in self.intervals.iter_mut() {
            if interval.material == target_material {
                match interval.span {
                    Span::Fixed {
                        length: start_length,
                    } => {
                        let slack_length = start_length * (1.0 - interval.strain);
                        let target_length = slack_length * (1.0 + average_strain);
                        interval.span = Approaching {
                            target_length,
                            start_length,
                        }
                    }
                    _ => {}
                }
            }
        }
        self.progress.start(Seconds(10.0));
    }

    fn pair_generator(&self) -> PairGenerator {
        PairGenerator::new(self.joint_incidents(), self.interval_map())
    }

    fn interval_map(&self) -> HashMap<(JointKey, JointKey), Interval> {
        self.interval_values()
            .map(|interval| (interval.key(), interval.clone()))
            .collect()
    }

    pub fn _correct_folded_pulls(&mut self, minimum_dot_product: f32) {
        let folded: Vec<_> = self
            .joint_incidents()
            .into_values()
            .filter(|joint| joint.push().is_some())
            .flat_map(|joint| {
                let key = joint.key;
                let pulls = joint.pulls();
                pulls
                    .into_iter()
                    .tuple_windows()
                    .map(|(a, b)| {
                        let ray_a = a.1.ray_from(key);
                        let ray_b = b.1.ray_from(key);
                        let (short_pull, long_pull) = if a.1.ideal() < b.1.ideal() {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        let dot_product = ray_a.dot(ray_b);
                        FoldedPull {
                            joint_key: key,
                            short_pull,
                            long_pull,
                            dot_product,
                        }
                    })
                    .max_by(|a, b| a.dot_product.total_cmp(&b.dot_product))
            })
            .filter(|&FoldedPull { dot_product, .. }| dot_product > minimum_dot_product)
            .collect();
        println!(
            "Folded Pulls: {:?}",
            folded.iter().map(|p| p.dot_product).collect::<Vec<f32>>()
        );
        // for to_remove in folded.iter().map(|FoldedPull { long_pull, .. }| long_pull.0) {
        //     println!("Remove {:?}", to_remove);
        //     self.remove_interval(to_remove);
        // }
        for FoldedPull {
            joint_key,
            short_pull: (_, short_interval),
            long_pull: (_, long_interval),
            dot_product,
        } in folded
        {
            let middle_joint = joint_key;
            let far_joint = long_interval.other_joint(middle_joint);
            let missing_length = long_interval.ideal() - short_interval.ideal();
            println!("Folded pull at joint {:?}: short {:?} (ideal {:.3}), long {:?} (ideal {:.3}), dot {:.3}",
                     joint_key,
                     short_interval.role,
                     short_interval.ideal(),
                     long_interval.role,
                     long_interval.ideal(),
                     dot_product);
            let id =
                self.create_interval(middle_joint, far_joint, missing_length, short_interval.role);
            println!("  -> Added interval {:?}", self.interval(id));
        }
    }
}

#[derive(Debug, Clone)]
struct FoldedPull {
    joint_key: JointKey,
    short_pull: (IntervalKey, Interval),
    long_pull: (IntervalKey, Interval),
    dot_product: f32,
}

#[derive(Debug)]
struct Pair {
    alpha_key: JointKey,
    omega_key: JointKey,
    length: f32,
}

impl Pair {
    fn key(&self) -> (JointKey, JointKey) {
        if self.alpha_key < self.omega_key {
            (self.alpha_key, self.omega_key)
        } else {
            (self.omega_key, self.alpha_key)
        }
    }
}

struct PairGenerator {
    joints: HashMap<JointKey, JointIncident>,
    intervals: HashMap<(JointKey, JointKey), Interval>,
    pairs: HashMap<(JointKey, JointKey), Pair>,
}

impl PairGenerator {
    fn new(
        joints: HashMap<JointKey, JointIncident>,
        intervals: HashMap<(JointKey, JointKey), Interval>,
    ) -> Self {
        Self {
            joints,
            intervals,
            pairs: HashMap::new(),
        }
    }

    fn bow_tie_pulls(mut self, joints: &Joints) -> impl Iterator<Item = Pair> {
        let push_intervals: Vec<_> = self
            .intervals
            .values()
            .filter(|interval| interval.role == Pushing)
            .cloned()
            .collect();

        for interval in push_intervals {
            let meeting_pairs = self.find_meeting_pairs(&interval);
            match meeting_pairs.as_slice() {
                [(6, alpha1, omega1), (6, alpha2, omega2), ..] => {
                    self.handle_bridge_meeting(alpha1, omega1, alpha2, omega2, joints);
                }
                [(8, alpha1, omega1), (8, alpha2, omega2)] => {
                    self.handle_joint_meeting(alpha1, omega1, alpha2, omega2, &interval);
                }
                _ => {}
            }
        }

        self.pairs.into_values()
    }

    fn find_meeting_pairs(&self, interval: &Interval) -> Vec<(usize, Path, Path)> {
        let mut meeting_pairs = vec![];
        for alpha_path in self.paths_for(interval.alpha_key, 2) {
            for omega_path in self.paths_for(interval.omega_key, 2) {
                if alpha_path.last_interval().key() == omega_path.last_interval().key() {
                    // second interval is the bridge
                    meeting_pairs.push((6, alpha_path.clone(), omega_path.clone()));
                }
                if alpha_path.last_joint() == omega_path.last_joint() {
                    meeting_pairs.push((8, alpha_path.clone(), omega_path.clone()));
                }
            }
        }
        meeting_pairs.sort_by_key(|(size, _, _)| *size);
        meeting_pairs
    }
    fn handle_bridge_meeting(
        &mut self,
        alpha1: &Path,
        omega1: &Path,
        alpha2: &Path,
        omega2: &Path,
        joints: &Joints,
    ) {
        let diagonals = [
            (alpha1.last_joint(), omega2.last_joint()),
            (alpha2.last_joint(), omega1.last_joint()),
        ];
        let cross_twist_diagonals: Vec<_> = diagonals
            .iter()
            .filter_map(|&(a, b)| {
                if self.interval_exists(
                    self.joints[&a].across_push()?,
                    self.joints[&b].across_push()?,
                ) {
                    return None;
                }
                Some((a, b))
            })
            .collect();
        if let &[(alpha_key, omega_key)] = cross_twist_diagonals.as_slice() {
            let alpha_pt = joints[alpha_key].location;
            let omega_pt = joints[omega_key].location;
            let distance = alpha_pt.distance(omega_pt);
            let pair = Pair {
                alpha_key,
                omega_key,
                length: distance * BOW_TIE_SHORTEN,
            };
            self.pairs.insert(pair.key(), pair);
        } else {
            let candidate_completions = [
                (alpha1, alpha2),
                (alpha2, alpha1),
                (omega1, omega2),
                (omega2, omega1),
            ];
            let triangle_completions: Vec<_> = candidate_completions
                .iter()
                .filter_map(|&(path, other_path)| {
                    if self.joints[&other_path.joint_keys[1]].push().is_some() {
                        return None;
                    }
                    Some((path.joint_keys[0], path.last_joint()))
                })
                .collect();
            if let &[(alpha_key, omega_key)] = triangle_completions.as_slice() {
                let alpha_pt = joints[alpha_key].location;
                let omega_pt = joints[omega_key].location;
                let distance = alpha_pt.distance(omega_pt);
                let pair = Pair {
                    alpha_key,
                    omega_key,
                    length: distance * BOW_TIE_SHORTEN,
                };
                self.pairs.insert(pair.key(), pair);
            }
        }
    }
    fn handle_joint_meeting(
        &mut self,
        alpha1: &Path,
        omega1: &Path,
        alpha2: &Path,
        omega2: &Path,
        interval: &Interval,
    ) {
        let candidates = [
            (alpha1, alpha2.last_joint()),
            (alpha2, alpha1.last_joint()),
            (omega1, omega2.last_joint()),
            (omega2, omega1.last_joint()),
        ];
        for (path, omega_key) in candidates {
            let alpha_key = path.joint_keys[1];
            if self.joints[&alpha_key].push().is_none() {
                continue;
            }
            let pair = Pair {
                alpha_key,
                omega_key,
                length: interval.ideal() / 4.0,
            };
            self.pairs.insert(pair.key(), pair);
        }
    }

    fn interval_exists(&self, a: JointKey, b: JointKey) -> bool {
        if a < b {
            self.intervals.contains_key(&(a, b))
        } else {
            self.intervals.contains_key(&(b, a))
        }
    }
    fn paths_for(&self, joint_key: JointKey, max_size: usize) -> Vec<Path> {
        let paths: Vec<_> = self.joints[&joint_key]
            .pulls()
            .iter()
            .map(|(_, pull)| Path::new(joint_key, pull.clone()))
            .collect();
        self.paths_via_pulls(&paths, 1, max_size)
    }

    fn paths_via_pulls(&self, paths: &[Path], size: usize, max_size: usize) -> Vec<Path> {
        if size == max_size {
            paths.to_vec()
        } else {
            let bigger: Vec<_> = paths
                .iter()
                .flat_map(|path| self.joints[&path.last_joint()].extended_paths(path))
                .collect();
            self.paths_via_pulls(&bigger, size + 1, max_size)
        }
    }
}
