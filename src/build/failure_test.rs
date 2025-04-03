use crate::application::AppStateChange::SetIntervalColor;
use crate::crucible::CrucibleAction::TesterDo;
use crate::crucible::TesterAction;
use crate::fabric::interval::Interval;
use crate::fabric::material::Material;
use crate::fabric::Fabric;
use crate::messages::{LabEvent, Scenario};
use cgmath::InnerSpace;
use winit::event_loop::EventLoopProxy;

#[derive(Clone)]
pub struct FailureTest {
    pub test_number: usize,
    pub fabric: Fabric,
    pub finished: bool,
    interval_missing: Option<(usize, usize)>,
}

impl FailureTest {
    pub fn title(&self) -> String {
        match self.interval_missing {
            None => {
                format!("Test #{}", self.test_number)
            }
            Some(pair) => {
                format!("Test #{} {pair:?}", self.test_number)
            }
        }
    }

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
                test_number: 0,
                fabric: default_fabric.clone(),
                interval_missing: None,
                finished: false,
            };
            interval_keys.len()
        ];
        for index in 0..interval_keys.len() {
            test_cases[index].test_number = index;
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

    pub fn finish(
        &mut self,
        default_fabric: &Fabric,
        min_damage: f32,
        max_damage: f32,
        event_loop_proxy: EventLoopProxy<LabEvent>,
    ) {
        if self.finished {
            return
        }
        self.finished = true;
        let key = self.interval_missing.unwrap();
        let clamped = self.damage(default_fabric).clamp(min_damage, max_damage);
        let redness = (clamped - min_damage) / (max_damage - min_damage);
        let color = [redness, 0.01, 0.01, 1.0];
        let send = |lab_event: LabEvent| event_loop_proxy.send_event(lab_event).unwrap();
        send(LabEvent::AppStateChanged(SetIntervalColor { key, color }));
        send(LabEvent::Crucible(TesterDo(TesterAction::NextExperiment)));
    }
}
