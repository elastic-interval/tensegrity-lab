//! Disassembly visualisation: a crane hooks the top of the structure and takes
//! it down one strut at a time. Each cycle: hold still for a few seconds so the
//! structure settles, then lower quickly until the lowest remaining strut
//! touches the ground, then remove that strut. The connector caps (joint mass)
//! and cables remain, so the de-strutted structure sags under gravity. When all
//! struts are gone, the hook lifts the cable-and-cap form clear of the ground.
//!
//! Paced by *simulated* time, so it is independent of frame rate; speed the
//! playback up or down with the time-scale control.

use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::physics::Physics;
use crate::fabric::{Fabric, JointKey, Level};
use crate::units::{Meters, Seconds, Unit};
use crate::{Age, Radio, StateChange};
use glam::Vec3;

/// How long to hold still and let the structure settle after each strut removal.
const SETTLE_SECONDS: Seconds = Seconds(1.0);

/// Give up lowering a strut to the ground after this long and remove it anyway,
/// so a strut resting on the draped pile can't stall the teardown.
const LOWER_TIMEOUT: Seconds = Seconds(5.0);

/// Segment spacing when converting cables to bendable chains (≈20 cm pushes).
const CABLE_SEGMENT_LENGTH: Meters = Meters(0.10);

/// Moderate damping so the soft, light cable chains drape calmly without ringing,
/// while the stiff structure still moves naturally (not honey).
const DISASSEMBLY_DRAG: f32 = 0.5;
const DISASSEMBLY_VISCOSITY: f32 = 10.0;

/// Top descent speed (metres per simulated second) while bringing the next strut
/// down — eased to a much slower touchdown near the ground (see `SLOWDOWN_HEIGHT`).
const LOWER_SPEED: f32 = 0.6;

/// Start easing the descent toward a gentle touchdown once the lowest strut is
/// within this height of the ground, so it sets down without shock.
const SLOWDOWN_HEIGHT: Meters = Meters(0.5);

/// A strut counts as "on the ground" when its lower end is within this height.
const GROUND_TOUCH: Meters = Meters(0.05);

/// Once all struts are gone, lift the cable-and-cap form until its lowest joint
/// clears this height off the ground.
const LIFT_CLEARANCE: Meters = Meters(0.10);

/// Hook rise during the final lift, in metres per simulated second.
const LIFT_SPEED: f32 = 1.0;

#[derive(PartialEq)]
enum Phase {
    /// Holding still after a removal, letting the structure settle.
    Settling,
    /// Lowering quickly until the lowest strut touches the ground.
    Lowering,
}

pub struct Disassembler {
    pub fabric: Fabric,
    pub physics: Physics,
    radio: Radio,
    /// The joint the crane holds (the topmost one).
    anchor: JointKey,
    /// Fixed x/z of the hook.
    anchor_base: Vec3,
    /// Current commanded hook height.
    anchor_y: f32,
    phase: Phase,
    last_removal_age: Age,
    /// Set once the final lift has reached clearance, so the hook then holds
    /// still and the swing damps out instead of being re-triggered.
    lifted: bool,
}

impl Disassembler {
    pub fn new(fabric: Fabric, physics: Physics, radio: Radio) -> Self {
        // Cables are converted to bendable chains lazily, as they go slack during
        // teardown (see iterate) — taut cables stay stiff and hold the shape.

        // Moderate damping keeps the light cable joints calm (no jitter/ringing).
        let mut physics = physics;
        physics.drag = DISASSEMBLY_DRAG;
        physics.viscosity = DISASSEMBLY_VISCOSITY;

        // The crane hooks the topmost joint.
        let anchor = fabric
            .joints
            .iter()
            .max_by(|(_, a), (_, b)| a.location.y.partial_cmp(&b.location.y).unwrap())
            .map(|(key, _)| key)
            .expect("fabric has joints");
        let anchor_base = fabric.joints[anchor].location;
        Self {
            anchor_y: anchor_base.y,
            phase: Phase::Lowering,
            last_removal_age: fabric.age,
            lifted: false,
            anchor,
            anchor_base,
            fabric,
            physics,
            radio,
        }
    }

    pub fn copy_physics_into(&self, context: &mut CrucibleContext) {
        *context.physics = self.physics.clone();
    }

    pub fn iterate(&mut self, context: &mut CrucibleContext, iterations_per_frame: usize) {
        self.fabric = context.fabric.clone();
        // A transient spike (e.g. a heavy cap snapping a slack cable taut) can trip
        // the fabric's safety freeze, which would otherwise halt the teardown for
        // good. The freeze already zeroed velocities, so just clear it and resume.
        self.fabric.frozen = false;

        let struts = Self::struts_in(&self.fabric);
        let frame_seconds = iterations_per_frame as f32 * Age::iteration_duration();

        if struts == 0 {
            // All struts gone: lift the cable-and-cap form clear of the ground,
            // then latch and hold still so the swing damps out (chasing the
            // swinging lowest joint would keep pumping energy into it).
            if !self.lifted {
                if self.fabric.altitude_range().0 < LIFT_CLEARANCE.f32() {
                    self.anchor_y += LIFT_SPEED * frame_seconds;
                } else {
                    self.lifted = true;
                }
            }
        } else {
            match self.phase {
                Phase::Settling => {
                    let settled = self.fabric.age.elapsed_since(self.last_removal_age);
                    if settled.f32() >= SETTLE_SECONDS.f32() {
                        self.phase = Phase::Lowering;
                    }
                }
                Phase::Lowering => {
                    // Ease toward a soft touchdown: full speed when far, slowing as
                    // the strut nears the ground so it sets down gently, no shock.
                    let nearness = (self.lowest_strut_gap() / SLOWDOWN_HEIGHT.f32()).clamp(0.1, 1.0);
                    self.anchor_y -= LOWER_SPEED * nearness * frame_seconds;
                }
            }
        }

        let target = Vec3::new(self.anchor_base.x, self.anchor_y, self.anchor_base.z);
        for _ in 0..iterations_per_frame {
            self.fabric.iterate(&self.physics);
            if let Some(joint) = self.fabric.joints.get_mut(self.anchor) {
                joint.location = target;
                joint.velocity = Vec3::ZERO;
            }
        }

        // Cables freed by strut removal slacken — convert those to bendable chains
        // so they drape, leaving the still-loaded cables stiff.
        self.fabric.make_slack_cables_bendable(CABLE_SEGMENT_LENGTH);

        // Remove the strut once it's lowered onto the ground — or after a timeout,
        // so a strut that can't reach the ground (resting on the draped pile)
        // doesn't stall the teardown. Then go back to settling.
        let lowering_time = self.fabric.age.elapsed_since(self.last_removal_age).f32()
            - SETTLE_SECONDS.f32();
        let touched = self.lowest_strut_gap() <= GROUND_TOUCH.f32();
        let timed_out = lowering_time >= LOWER_TIMEOUT.f32();
        if struts > 0 && self.phase == Phase::Lowering && (touched || timed_out) {
            self.remove_lowest_strut();
            self.last_removal_age = self.fabric.age;
            self.phase = Phase::Settling;
            if Self::struts_in(&self.fabric) == 0 {
                StateChange::SetStageLabel("Disassembling — lifting".to_string())
                    .send(&self.radio);
            }
        }

        context.replace_fabric(self.fabric.clone());
        *context.physics = self.physics.clone();
    }

    /// Count real load-bearing struts — excluding the cables' bracing pushes,
    /// which are also `Role::Pushing` but `Level::Cable`.
    fn struts_in(fabric: &Fabric) -> usize {
        fabric
            .intervals
            .values()
            .filter(|iv| iv.role == Role::Pushing && iv.level == Level::Structural)
            .count()
    }

    /// Height of the lowest strut's lower end above the ground (0 if none/already down).
    fn lowest_strut_gap(&self) -> f32 {
        let joints = &self.fabric.joints;
        let gap = self
            .fabric
            .intervals
            .values()
            .filter(|iv| iv.role == Role::Pushing && iv.level == Level::Structural)
            .map(|iv| {
                joints[iv.alpha_key]
                    .location
                    .y
                    .min(joints[iv.omega_key].location.y)
            })
            .fold(f32::INFINITY, f32::min);
        if gap.is_finite() {
            gap.max(0.0)
        } else {
            0.0
        }
    }

    /// Remove the strut whose lower end sits closest to the ground.
    fn remove_lowest_strut(&mut self) {
        let joints = &self.fabric.joints;
        let lowest = self
            .fabric
            .intervals
            .iter()
            .filter(|(_, iv)| iv.role == Role::Pushing && iv.level == Level::Structural)
            .min_by(|(_, a), (_, b)| {
                let ay = joints[a.alpha_key]
                    .location
                    .y
                    .min(joints[a.omega_key].location.y);
                let by = joints[b.alpha_key]
                    .location
                    .y
                    .min(joints[b.omega_key].location.y);
                ay.partial_cmp(&by).unwrap()
            })
            .map(|(key, _)| key);
        if let Some(key) = lowest {
            self.fabric.remove_interval(key);
            StateChange::SetStageLabel(format!(
                "Disassembling — {} struts left",
                Self::struts_in(&self.fabric)
            ))
            .send(&self.radio);
        }
    }
}
