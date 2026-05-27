//! Trivial joint labeller for fabrics built by the DSL.
//!
//! Reads the `JointLabel` set on each joint during the build phase (see
//! `build_phase.rs`'s `label_seed_joints` and `label_off_joints`). Joints
//! without a label fall back to `JointPath::Display`.

use crate::fabric::{Fabric, JointKey, JointLabeller};

#[derive(Debug)]
pub struct SymmetricOrbitLabeller;

impl JointLabeller for SymmetricOrbitLabeller {
    fn label(&self, fabric: &Fabric, key: JointKey) -> Option<String> {
        fabric.joints.get(key)?.label.map(|l| l.to_string())
    }
}
