use cgmath::InnerSpace;
use crate::fabric::Fabric;

#[derive(Clone)]
pub struct FailureTest {
    pub fabric: Fabric,
    pub interval_missing: Option<(usize, usize)>,
    pub finished: bool,
}

impl FailureTest {
    pub fn damage(&self, default_fabric: &Fabric) -> f32 {
        let mut damage = 0.0;
        for joint_id in 0..default_fabric.joints.len() {
            let default_location = default_fabric.location(joint_id);
            let new_location = self.fabric.location(joint_id);
            damage += (default_location - new_location).magnitude();
        }
        damage
    }
}
