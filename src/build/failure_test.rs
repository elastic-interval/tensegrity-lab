use cgmath::InnerSpace;
use crate::fabric::Fabric;
use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::messages::Scenario;

#[derive(Clone)]
pub struct FailureTest {
    pub fabric: Fabric,
    pub interval_missing: Option<(usize, usize)>,
    pub finished: bool,
}

impl FailureTest {
    pub fn generate(default_fabric: &Fabric, scenario: Scenario) -> Vec<FailureTest> {
        let interval_keys: Vec<_> = default_fabric
            .intervals
            .iter()
            .flat_map(|(id, interval)| match (interval.material, &scenario) {
                (Material::PullMaterial, Scenario::TensionTest) => Some(*id),
                (Material::PushMaterial, Scenario::CompressionTest) => Some(*id),
                (Material::GuyLineMaterial, Scenario::TensionTest) => Some(*id),
                _ => None,
            })
            .collect();
        let mut test_cases = vec![
            FailureTest {
                fabric: default_fabric.clone(),
                interval_missing: None,
                finished: false,
            };
            interval_keys.len()
        ];
        for index in 0..interval_keys.len() {
            let missing_key = interval_keys[index];
            let &Interval {
                alpha_index,
                omega_index,
                ..
            } = default_fabric.interval(missing_key);
            test_cases[index].fabric.remove_interval(missing_key);
            test_cases[index].interval_missing = Some(if alpha_index < omega_index {
                (alpha_index, omega_index)
            } else {
                (omega_index, alpha_index)
            });
        }
        test_cases
    }

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
