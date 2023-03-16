use crate::fabric::{Fabric, UniqueId};

pub trait FabricHook {
    fn init(&mut self, fabric: &mut Fabric);
    fn on_frame(&self, fabric: &mut Fabric);
}

#[derive(Debug, Clone, Default)]
pub struct InsideOutDonut {
    some_state: Vec<UniqueId>,
}

impl FabricHook for InsideOutDonut {

    fn init(&mut self, fabric: &mut Fabric) {
        println!("init");
        for interval in fabric.intervals.values_mut() {
            if !fabric.joints[interval.alpha_index].location_fixed {
                continue;
            }
            // todo: adjust
        }
    }

    fn on_frame(&self, fabric: &mut Fabric) {
        println!("frame");
        for interval in fabric.intervals.values_mut() {
            if !fabric.joints[interval.alpha_index].location_fixed {
                continue;
            }
            // todo: adjust
        }
    }
}
