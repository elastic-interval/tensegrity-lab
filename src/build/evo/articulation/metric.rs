//! Rigid-motion-invariant shape comparison.
//!
//! The articulating brick floats free, so absolute joint positions drift
//! and rotate between trial phases. We measure *shape change* via the
//! pairwise distance matrix: the multiset of inter-joint distances is
//! invariant to translation, rotation, and reflection, yet changes the
//! moment the structure actually deforms. No SVD, no correspondence beyond
//! the shared joint ordering.

use crate::fabric::{Fabric, JointKey};
use glam::Vec3;

/// Positions of the given joints (the brick's own structural joints,
/// excluding face-middle joints, so the actuator's commanded stroke
/// doesn't masquerade as structural articulation).
pub fn pose(fabric: &Fabric, joints: &[JointKey]) -> Vec<Vec3> {
    joints
        .iter()
        .map(|&key| fabric.joints[key].location)
        .collect()
}

/// RMS of the change in every inter-joint distance between two poses.
/// Both slices must list the same joints in the same order. Units: metres.
pub fn shape_distance(a: &[Vec3], b: &[Vec3]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    if n < 2 {
        return 0.0;
    }
    let mut sum_sq = 0.0;
    let mut count = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            let da = a[i].distance(a[j]);
            let db = b[i].distance(b[j]);
            let d = da - db;
            sum_sq += d * d;
            count += 1;
        }
    }
    (sum_sq / count as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    fn sample() -> Vec<Vec3> {
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]
    }

    #[test]
    fn rigid_motion_is_invisible() {
        let a = sample();
        let rot = Quat::from_rotation_z(0.9);
        let shift = Vec3::new(3.0, -2.0, 5.0);
        let b: Vec<Vec3> = a.iter().map(|p| rot * *p + shift).collect();
        assert!(
            shape_distance(&a, &b) < 1e-5,
            "rigid motion registered as deformation: {}",
            shape_distance(&a, &b)
        );
    }

    #[test]
    fn deformation_is_visible() {
        let a = sample();
        let mut b = a.clone();
        b[1] += Vec3::new(0.5, 0.0, 0.0); // stretch one joint out
        assert!(
            shape_distance(&a, &b) > 0.1,
            "deformation went unmeasured: {}",
            shape_distance(&a, &b)
        );
    }
}
