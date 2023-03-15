use crate::fabric::{Fabric, UniqueId};

// how to do this?
pub trait PostIterate {
    fn post_iterate(&self, fabric: &mut Fabric);
}

#[derive(Debug, Clone, Default)]
pub struct InsideOutDonut {
    some_state: Vec<UniqueId>,
}

impl InsideOutDonut {
    pub fn post_iterate(&self, fabric: &mut Fabric) {
        println!("post-iterate");
        for interval in fabric.intervals.values_mut() {
            if !fabric.joints[interval.alpha_index].location_fixed {
                continue;
            }
            // todo: adjust
        }
    }
}
