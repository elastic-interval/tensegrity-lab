use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, UniqueId};
use crate::messages::StateChange::SetIntervalColor;
use crate::messages::{PhysicsFeature, PhysicsTesterAction, Radio};

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

pub struct PhysicsTester {
    pub fabric: Fabric,
    pub physics: Physics,
    pub failed_intervals: Vec<UniqueId>,
    radio: Radio,
}

impl PhysicsTester {
    pub fn new(fabric: &Fabric, physics: Physics, radio: Radio) -> Self {
        let mut fabric = fabric.clone();
        fabric.activate_muscles(true);
        Self {
            fabric,
            physics,
            failed_intervals: Vec::new(),
            radio,
        }
    }

    pub fn iterate(&mut self) {
        self.fabric.iterate(&self.physics);
        self.failed_intervals = self.fabric.failed_intervals(self.physics.strain_limit);
        if !self.failed_intervals.is_empty() {
            for failed in self.failed_intervals.iter() {
                let key = self.fabric.interval(*failed).key();
                SetIntervalColor { key, color: GREEN }.send(&self.radio)
            }
        }
    }

    pub fn action(&mut self, action: PhysicsTesterAction) {
        match action {
            PhysicsTesterAction::SetPhysicalParameter(parameter) => {
                if matches!(parameter.feature, PhysicsFeature::Pretenst) {
                    self.fabric.set_pretenst(parameter.value, 100);
                }
                self.physics.accept(parameter);
            }
            PhysicsTesterAction::DumpPhysics => {
                println!("{:?}", self.physics);
            }
        }
    }
}
