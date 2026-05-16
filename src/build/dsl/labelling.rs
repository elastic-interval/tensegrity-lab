//! Joint-labelling glue between the DSL and the generic fabric layer.
//!
//! The fabric layer defines a `JointLabeller` trait (see `fabric/mod.rs`) but
//! has no DSL types in scope. Concrete implementations live here.

use crate::build::dsl::brick::Axis;
use crate::build::dsl::brick_dsl::{BrickName, BrickRole, JointName};
use crate::build::dsl::brick_library;
use crate::fabric::{Fabric, JointKey, JointLabeller};

/// Renders the 12 joints of a 3-fold-symmetric Omni-shaped seed brick (under
/// an orientation that has declared a cyclic axis order) with symbolic
/// 3-character labels like `BAA`, `TOC`. Non-seed joints (`branches` not
/// empty) get `None` so the fabric falls back to the default `Display`.
#[derive(Debug)]
pub struct OmniSeedLabeller {
    pub brick_name: BrickName,
    pub brick_role: BrickRole,
}

impl JointLabeller for OmniSeedLabeller {
    fn label(&self, fabric: &Fabric, joint_key: JointKey) -> Option<String> {
        let joint = fabric.joints.get(joint_key)?;
        if !joint.path.branches.is_empty() {
            return None;
        }

        let proto = brick_library::get_prototype(self.brick_name);
        let cyclic_axes = proto.cyclic_axes_for(self.brick_role)?;
        if cyclic_axes.len() != 3 {
            return None;
        }

        let idx = joint.path.local_index as usize;
        // Local indices: first `proto.joints.len()` are explicit joints, then
        // each push contributes (alpha, omega) in order.
        let explicit_n = proto.joints.len();
        let name: JointName = if idx < explicit_n {
            proto.joints[idx]
        } else {
            let push_offset = idx - explicit_n;
            let push_idx = push_offset / 2;
            let push = proto.pushes.get(push_idx)?;
            if push_offset % 2 == 0 {
                push.alpha
            } else {
                push.omega
            }
        };

        let (category, axis): (_, Axis) = name.omni_decode()?;
        let pos = cyclic_axes.iter().position(|a| *a == axis)?;
        let axis_letter = match pos {
            0 => 'A',
            1 => 'B',
            2 => 'C',
            _ => return None,
        };
        Some(format!("{}{}", category.short(), axis_letter))
    }
}
