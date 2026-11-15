//! A slack cable hung as a chain, so that it drapes.
//!
//! The cable itself stays in the fabric, out of it (`Interval::is_out`), so
//! it keeps its key and its place on the connectors. Between its ends hang
//! short cable segments with a joint between each two. Taken down again the
//! cable is itself once more, drawn to its length.

use crate::fabric::interval::{Role, Span};
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Grams, Meters, Percent, Seconds};

/// Softer than the cable it stands in for: its joints are light, and a
/// stiffer segment would ring faster than the tick can follow.
const CHAIN_STIFFNESS: Percent = Percent(25.0);

/// The chain a cable hangs as, and what brings the cable back.
#[derive(Clone, Debug)]
pub struct Chain {
    pub cable: IntervalKey,
    stiffness: Percent,
    joints: Vec<JointKey>,
}

impl Fabric {
    /// Hang `cable` as a chain of segments about `segment` long, along the
    /// line between its ends. Its end fittings' mass goes to the chain's
    /// first and last joints.
    pub fn drape(&mut self, cable: IntervalKey, segment: Meters) -> Chain {
        let interval = &mut self.intervals[cable];
        let (alpha, omega, length, stiffness) = (interval.alpha_key, interval.omega_key, interval.ideal(), interval.stiffness);
        interval.stiffness = Percent(0.0);
        interval.span = Span::Fixed { length };
        let (from, to) = (self.location(alpha), self.location(omega));
        let segments = ((length / segment).round() as usize).max(2);
        let fitting = self.dimensions.hardware.map_or(Grams(0.0), |hardware| hardware.cable_end);
        let joints: Vec<JointKey> = (1..segments)
            .map(|index| {
                let joint = self.create_joint(from.lerp(to, index as f32 / segments as f32));
                let fittings = (index == 1) as u8 + (index == segments - 1) as u8;
                self.joints[joint].point_mass = Some(fitting * fittings as f32);
                joint
            })
            .collect();
        let ends: Vec<JointKey> = std::iter::once(alpha).chain(joints.iter().copied()).chain([omega]).collect();
        for pair in ends.windows(2) {
            let link = self.create_fixed_interval(pair[0], pair[1], Role::Pulling, length / segments as f32);
            self.intervals[link].chain = true;
            self.intervals[link].stiffness = CHAIN_STIFFNESS;
        }
        Chain { cable, stiffness, joints }
    }

    /// Take `chain` down: its cable back in the fabric, drawn to its length
    /// over `duration` if its ends are further apart than that.
    pub fn undrape(&mut self, chain: Chain, duration: Seconds) {
        for joint in chain.joints {
            self.remove_joint(joint);
        }
        let interval = &mut self.intervals[chain.cable];
        interval.stiffness = chain.stiffness;
        let (length, spanned) = (interval.ideal(), interval.length(&self.joints));
        if spanned > length {
            self.intervals[chain.cable].span = Span::Fixed { length: spanned };
            self.extend_interval(chain.cable, length, duration);
        }
    }
}
