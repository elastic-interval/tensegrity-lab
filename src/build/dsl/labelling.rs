//! Trivial joint labeller for fabrics built by the DSL, plus the fabric's
//! label-level symmetry group.
//!
//! Reads the `JointLabel` set on each joint during the build phase (see
//! `build_phase.rs`'s `label_seed_joints` and `label_off_joints`). Joints
//! without a label fall back to `JointPath::Display`.

use crate::fabric::joint::JointLabel;
use crate::fabric::{Fabric, JointKey, JointLabeller};

#[derive(Debug)]
pub struct SymmetricOrbitLabeller;

impl JointLabeller for SymmetricOrbitLabeller {
    fn label(&self, fabric: &Fabric, key: JointKey) -> Option<String> {
        fabric.joints.get(key)?.label.map(|l| l.to_string())
    }
}

/// The fabric's label-level symmetry group, derived from the seed at build
/// time (see `build_phase.rs`'s `seed_label_symmetry`). The group acts on
/// `JointLabel` by pure letter permutation; `Axial` labels (`Z<n>`) are
/// fixed points under every operation. See `docs/labeling-symmetries.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabelSymmetry {
    /// n-fold rotation: `letters[0] → letters[1] → … → letters[0]`.
    /// Letter order follows the seed brick's `cyclic_axes` declaration,
    /// not plan declaration order.
    Cyclic { letters: Vec<char> },
    /// Mirror reflection: each pair swaps (involution).
    Mirror { pairs: Vec<(char, char)> },
}

impl LabelSymmetry {
    /// Symmetry order: applying the generator this many times is identity.
    pub fn order(&self) -> usize {
        match self {
            LabelSymmetry::Cyclic { letters } => letters.len(),
            LabelSymmetry::Mirror { .. } => 2,
        }
    }

    /// One generator step. Letters outside the group map to themselves.
    pub fn map_letter(&self, letter: char) -> char {
        match self {
            LabelSymmetry::Cyclic { letters } => letters
                .iter()
                .position(|&c| c == letter)
                .map(|i| letters[(i + 1) % letters.len()])
                .unwrap_or(letter),
            LabelSymmetry::Mirror { pairs } => pairs
                .iter()
                .find_map(|&(a, b)| {
                    (a == letter)
                        .then_some(b)
                        .or((b == letter).then_some(a))
                })
                .unwrap_or(letter),
        }
    }

    /// One generator step on a label. `OffAxis` letters permute; `Axial`
    /// labels are fixed points.
    pub fn map_label(&self, label: JointLabel) -> JointLabel {
        match label {
            JointLabel::OffAxis {
                letter,
                brick,
                position,
            } => JointLabel::OffAxis {
                letter: self.map_letter(letter),
                brick,
                position,
            },
            axial @ JointLabel::Axial { .. } => axial,
        }
    }

    /// The full orbit of a label under the generator, starting from `label`.
    /// Fixed points (axial labels, letters outside the group) give a
    /// singleton orbit.
    pub fn orbit(&self, label: JointLabel) -> Vec<JointLabel> {
        let mut orbit = vec![label];
        let mut current = self.map_label(label);
        while current != label {
            orbit.push(current);
            current = self.map_label(current);
        }
        orbit
    }

    /// Canonical representative of a label's orbit (its minimum member) —
    /// an engraving-friendly orbit key: every member of {A03.2, B03.2, C03.2}
    /// keys to `A03.2`.
    pub fn orbit_key(&self, label: JointLabel) -> JointLabel {
        self.orbit(label).into_iter().min().unwrap_or(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};

    fn build(name: FabricName) -> Fabric {
        let plan = fabric_library::get_fabric_plan(name);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() == ExecutorStage::Building {
            let _ = executor.iterate();
        }
        executor.fabric
    }

    fn off_axis(letter: char, brick: u8, position: u8) -> JointLabel {
        JointLabel::OffAxis {
            letter,
            brick,
            position,
        }
    }

    #[test]
    fn open_claw_is_threefold_cyclic() {
        let fabric = build(FabricName::OpenClaw);
        let symmetry = fabric
            .label_symmetry
            .as_ref()
            .expect("OpenClaw should have a label symmetry");
        assert_eq!(
            *symmetry,
            LabelSymmetry::Cyclic {
                letters: vec!['A', 'B', 'C']
            }
        );
        assert_eq!(symmetry.order(), 3);

        // The orbit of a real seed joint visits all three legs, and every
        // member exists on the built fabric.
        let orbit = symmetry.orbit(off_axis('A', 0, 2));
        assert_eq!(
            orbit,
            vec![off_axis('A', 0, 2), off_axis('B', 0, 2), off_axis('C', 0, 2)]
        );
        for label in &orbit {
            assert!(
                fabric.joint_key_by_label(&label.to_string()).is_some(),
                "expected joint labelled {label} on OpenClaw"
            );
        }
        // Every member keys to the same canonical representative.
        for label in &orbit {
            assert_eq!(symmetry.orbit_key(*label), off_axis('A', 0, 2));
        }
        // Axial labels are fixed points.
        let axial = JointLabel::Axial { index: 1 };
        assert_eq!(symmetry.orbit(axial), vec![axial]);
    }

    #[test]
    fn headless_hug_is_mirror() {
        let fabric = build(FabricName::HeadlessHug);
        let symmetry = fabric
            .label_symmetry
            .as_ref()
            .expect("HeadlessHug should have a label symmetry");
        assert_eq!(
            *symmetry,
            LabelSymmetry::Mirror {
                pairs: vec![('A', 'B'), ('C', 'D')]
            }
        );
        assert_eq!(symmetry.order(), 2);

        // The C03.10 ↔ D03.10 pair that the fabric plan itself relies on
        // (the added chest cable in fabric_library.rs).
        let orbit = symmetry.orbit(off_axis('C', 3, 10));
        assert_eq!(orbit, vec![off_axis('C', 3, 10), off_axis('D', 3, 10)]);
        for label in &orbit {
            assert!(
                fabric.joint_key_by_label(&label.to_string()).is_some(),
                "expected joint labelled {label} on HeadlessHug"
            );
        }
    }

    #[test]
    fn halo_by_crane_has_no_symmetry() {
        let fabric = build(FabricName::HaloByCrane);
        assert!(fabric.label_symmetry.is_none());
    }
}
