use cgmath::InnerSpace;
use itertools::Itertools;

use crate::fabric::interval::Interval;
use crate::fabric::joint_incident::JointIncident;
use crate::fabric::{Fabric, UniqueId};

#[derive(Debug, Clone)]
struct FoldedPull {
    joint_index: usize,
    short_pull: (UniqueId, Interval),
    long_pull: (UniqueId, Interval),
    dot_product: f32,
}

impl Fabric {
    pub fn correct_folded_pulls(&mut self, minimum_dot_product: f32) {
        let folded: Vec<_> = self
            .joint_incidents()
            .into_iter()
            .filter(|joint| joint.push.is_some())
            .flat_map(|JointIncident { index, pulls, .. }| {
                pulls
                    .into_iter()
                    .tuple_windows()
                    .map(|(a, b)| {
                        let ray_a = a.1.ray_from(index);
                        let ray_b = b.1.ray_from(index);
                        let (short_pull, long_pull) = if a.1.ideal() < b.1.ideal() {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        let dot_product = ray_a.dot(ray_b);
                        FoldedPull {
                            joint_index: index,
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
            joint_index,
            short_pull: (_, short_interval),
            long_pull: (_, long_interval),
            ..
        } in folded
        {
            let middle_joint = short_interval.other_joint(joint_index);
            let far_joint = long_interval.other_joint(joint_index);
            let missing_length = long_interval.ideal() - short_interval.ideal();
            let id = self.create_interval(
                middle_joint,
                far_joint,
                missing_length,
                short_interval.material,
            );
            println!("Add {:?}", self.interval(id));
        }
    }
}
