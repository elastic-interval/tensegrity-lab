//! Packing for transport: a crane disassembles the structure and reassembles it
//! again, visualising how it packs down and sets back up. It hooks the top of the
//! structure and takes it down one strut at a time (lowest first), recording
//! each removed strut (with the hook height at removal) on a stack. To rebuild,
//! the crane retraces its descent — rising back through those heights — and the
//! struts are popped off and re-inserted in reverse order.
//!
//! Cables are not scripted at all: their state is a *law of nature* derivative of
//! the struts. A cable is a stiff cable only while both its caps still hold a
//! strut; lose a strut at either end and it becomes a bendable chain that drapes;
//! get both struts back and it reverts. So removing struts frees the cables to
//! hang, and re-inserting them pulls the cables taut again — automatically.
//!
//!   Settle → Lower → (pop struts onto stack) … → Lift → Hold → Descend
//!   → Replay (re-insert struts in reverse) … → Done
//!
//! Paced by simulated time — independent of frame rate (use the time-scale).

use crate::crucible_context::CrucibleContext;
use crate::fabric::interval::Role;
use crate::fabric::physics::Physics;
use crate::fabric::{BendableCable, Fabric, JointKey, Level};
use crate::units::{Meters, Seconds, Unit};
use crate::{Age, Radio, StateChange};
use glam::Vec3;

/// How long to hold still and let the structure settle after each strut removal.
/// Also the window during which the next strut's name is shown before it descends,
/// so each removal is a clear, watchable interval (and pausable in a future movie).
const SETTLE_SECONDS: Seconds = Seconds(3.0);

/// Give up lowering a strut to the ground after this long and remove it anyway,
/// so a strut resting on the draped pile can't stall the teardown.
const LOWER_TIMEOUT: Seconds = Seconds(5.0);

/// Pause at the fully-disassembled hang before putting it back together.
const HOLD_SECONDS: Seconds = Seconds(3.0);

/// Time for a re-inserted strut to grow from its current length to full ideal
/// length (the Approach), during which the hook pauses while it takes shape.
const INSERT_SECONDS: Seconds = Seconds(3.0);

/// Time for a revived cable to ease from its current span to rest length.
const REVIVE_SECONDS: Seconds = Seconds(3.0);

/// Segment spacing when converting cables to bendable chains (≈20 cm pushes).
const CABLE_SEGMENT_LENGTH: Meters = Meters(0.10);

/// Moderate damping so the soft, light cable chains drape calmly without ringing,
/// while the stiff structure still moves naturally (not honey).
const DISASSEMBLY_DRAG: f32 = 0.5;
const DISASSEMBLY_VISCOSITY: f32 = 10.0;

/// Top hook speed (metres per simulated second) while lowering/raising — eased to
/// a much slower approach near the target (the ground, or a recorded height).
const HOOK_SPEED: f32 = 0.6;

/// Start easing the hook within this height of its target, for a gentle approach.
const SLOWDOWN_HEIGHT: Meters = Meters(0.5);

/// A strut counts as "on the ground" when its lower end is within this height.
const GROUND_TOUCH: Meters = Meters(0.05);

/// The low mark: the final lift clears the form's lowest joint to here, and the
/// crane descends back to here before replaying the build.
const MIN_MARK: Meters = Meters(0.10);

/// Hook rise speed for the final lift of the hanging form.
const LIFT_SPEED: f32 = 1.0;

/// After the rebuild, release the hook and let the structure settle onto the
/// surface for this long before returning to viewing.
const DROP_SECONDS: Seconds = Seconds(4.0);

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    /// Disassembly: hold still after a removal, letting the structure settle.
    Settle,
    /// Disassembly: lower until the lowest strut touches the ground, then remove.
    Lower,
    /// All struts gone: lift the cable-and-cap form clear of the ground.
    Lift,
    /// Pause at the hang before reassembling.
    Hold,
    /// Lower the hook back down to the mark (the form collapses to the ground).
    Descend,
    /// Reassembly: rise back through the recorded heights, re-inserting struts.
    Replay,
    /// Pause while a re-inserted strut grows to full length and takes shape.
    Shape,
    /// Rebuilt: release the hook and let the structure settle onto the surface.
    Drop,
    /// Settled — signal the crucible to return to viewing.
    Done,
}

/// A removed strut: its two cap joints, ideal length, and the hook height at
/// removal (so reassembly can rise back to the same height before re-inserting).
struct Strut {
    alpha: JointKey,
    omega: JointKey,
    ideal: Meters,
    hook_y: f32,
}

pub struct Packer {
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
    /// Simulated age at which the current phase began (for settle/hold/shape timing).
    phase_age: Age,
    /// Removed struts, in removal order; popped (reverse order) to re-insert.
    struts: Vec<Strut>,
    /// Cables currently in bendable-chain form (managed by `reconcile_cables`).
    chains: Vec<BendableCable>,
    /// Name of the strut currently being re-inserted (shown while it takes shape).
    shaping_label: Option<String>,
    /// Last action label sent, so we only broadcast on change.
    last_label: Option<String>,
}

impl Packer {
    pub fn new(fabric: Fabric, physics: Physics, radio: Radio) -> Self {
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
            phase: Phase::Lower,
            phase_age: fabric.age,
            struts: Vec::new(),
            chains: Vec::new(),
            shaping_label: None,
            last_label: None,
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
        // the fabric's safety freeze, which would otherwise halt for good. The
        // freeze already zeroed velocities, so just clear it and resume.
        self.fabric.frozen = false;

        let frame_seconds = iterations_per_frame as f32 * Age::iteration_duration();
        let elapsed = self.fabric.age.elapsed_since(self.phase_age).f32();

        // Hook motion / phase timing for this frame.
        match self.phase {
            Phase::Settle => {
                if elapsed >= SETTLE_SECONDS.f32() {
                    self.set_phase(Phase::Lower);
                }
            }
            Phase::Lower => {
                let nearness = (self.lowest_strut_gap() / SLOWDOWN_HEIGHT.f32()).clamp(0.1, 1.0);
                self.anchor_y -= HOOK_SPEED * nearness * frame_seconds;
            }
            Phase::Lift => {
                if self.fabric.altitude_range().0 < MIN_MARK.f32() {
                    self.anchor_y += LIFT_SPEED * frame_seconds;
                } else {
                    self.set_phase(Phase::Hold);
                }
            }
            Phase::Hold => {
                if elapsed >= HOLD_SECONDS.f32() {
                    self.set_phase(Phase::Descend);
                }
            }
            Phase::Descend => {
                if self.anchor_y > MIN_MARK.f32() {
                    self.anchor_y -= LIFT_SPEED * frame_seconds;
                } else {
                    self.set_phase(Phase::Replay);
                }
            }
            Phase::Replay => {
                // Rise fast toward the next strut's removal height, easing in.
                if let Some(strut) = self.struts.last() {
                    let gap = (strut.hook_y - self.anchor_y).max(0.0);
                    let nearness = (gap / SLOWDOWN_HEIGHT.f32()).clamp(0.1, 1.0);
                    self.anchor_y += HOOK_SPEED * nearness * frame_seconds;
                }
            }
            Phase::Shape => {
                if elapsed >= INSERT_SECONDS.f32() {
                    self.set_phase(Phase::Replay);
                }
            }
            Phase::Drop => {
                if elapsed >= DROP_SECONDS.f32() {
                    self.set_phase(Phase::Done);
                }
            }
            Phase::Done => {}
        }

        // While rebuilding, never let a re-forming strut grow against the ground:
        // if any strut's lower end dips below the mark, raise the hook so it lifts
        // clear (in real life only struts change length, and they can't push into
        // the floor). This also leaves the finished structure hanging ~MIN_MARK
        // above the surface, so the final release is a clean, gentle drop.
        if matches!(self.phase, Phase::Replay | Phase::Shape)
            && Self::struts_in(&self.fabric) > 0
            && self.lowest_strut_gap() < MIN_MARK.f32()
        {
            self.anchor_y += LIFT_SPEED * frame_seconds;
        }

        // Hold the hook everywhere except the final Drop, where it's released so
        // the rebuilt structure falls and settles onto the surface.
        let pin = !matches!(self.phase, Phase::Drop | Phase::Done);
        let target = Vec3::new(self.anchor_base.x, self.anchor_y, self.anchor_base.z);
        for _ in 0..iterations_per_frame {
            self.fabric.iterate(&self.physics);
            if pin {
                if let Some(joint) = self.fabric.joints.get_mut(self.anchor) {
                    joint.location = target;
                    joint.velocity = Vec3::ZERO;
                }
            }
        }

        // Law of nature: cables follow the struts. Removing a strut frees its
        // cables to become draping chains; re-inserting struts reverts them.
        self.fabric
            .reconcile_cables(&mut self.chains, CABLE_SEGMENT_LENGTH, REVIVE_SECONDS);

        // Disassembly: remove the lowest strut once it's on the ground, or after a
        // timeout (so a strut resting on the draped pile can't stall the teardown).
        if self.phase == Phase::Lower {
            let touched = self.lowest_strut_gap() <= GROUND_TOUCH.f32();
            let timed_out = elapsed >= LOWER_TIMEOUT.f32();
            if touched || timed_out {
                self.remove_lowest_strut();
                if Self::struts_in(&self.fabric) > 0 {
                    self.set_phase(Phase::Settle);
                } else {
                    self.set_phase(Phase::Lift);
                    StateChange::SetStageLabel("Packed — lifting".to_string())
                        .send(&self.radio);
                }
            }
        }

        // Reassembly: once the hook reaches the next strut's removal height,
        // re-insert it (growing to ideal length) and pause to take shape. Cables
        // revert to stiff form on their own as their struts return.
        if self.phase == Phase::Replay {
            match self.struts.last() {
                Some(strut) if self.anchor_y >= strut.hook_y => {
                    let strut = self.struts.pop().expect("strut present");
                    self.fabric.create_approaching_interval(
                        strut.alpha,
                        strut.omega,
                        strut.ideal,
                        Role::Pushing,
                        INSERT_SECONDS,
                    );
                    self.shaping_label = Some(self.strut_name(strut.alpha, strut.omega));
                    self.set_phase(Phase::Shape);
                    StateChange::SetStageLabel(format!(
                        "Rebuilding — {} struts left",
                        self.struts.len()
                    ))
                    .send(&self.radio);
                }
                Some(_) => {}
                None => {
                    self.set_phase(Phase::Drop);
                    StateChange::SetStageLabel("Rebuilt — lowering to surface".to_string())
                        .send(&self.radio);
                }
            }
        }

        // Show the busy strut's name beside the structure: the one being settled
        // and lowered to removal, or the one being re-inserted and taking shape.
        // Settle and Lower both name the lowest (next-to-go) strut, so the name
        // stays up for the whole removal interval rather than flashing past.
        let label = match self.phase {
            Phase::Settle | Phase::Lower => self.lowest_strut_name(),
            Phase::Shape => self.shaping_label.clone(),
            _ => None,
        };
        if label != self.last_label {
            StateChange::ShowActionLabel(label.clone()).send(&self.radio);
            self.last_label = label;
        }

        context.replace_fabric(self.fabric.clone());
        *context.physics = self.physics.clone();
    }

    /// Name of a strut, from the labels of its two cap joints.
    fn strut_name(&self, alpha: JointKey, omega: JointKey) -> String {
        format!(
            "{}-{}",
            self.fabric.joint_label(alpha),
            self.fabric.joint_label(omega)
        )
    }

    /// Name of the lowest remaining strut (the next to be removed), if any.
    fn lowest_strut_name(&self) -> Option<String> {
        let joints = &self.fabric.joints;
        self.fabric
            .intervals
            .values()
            .filter(|iv| iv.role == Role::Pushing && iv.level == Level::Structural)
            .min_by(|a, b| {
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
            .map(|iv| self.strut_name(iv.alpha_key, iv.omega_key))
    }

    /// True once the rebuilt structure has settled — time to return to viewing.
    pub fn is_complete(&self) -> bool {
        self.phase == Phase::Done
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_age = self.fabric.age;
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

    /// Remove the strut whose lower end sits closest to the ground, pushing it
    /// (caps + ideal length + current hook height) onto the stack for replay.
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
            let interval = &self.fabric.intervals[key];
            let strut = Strut {
                alpha: interval.alpha_key,
                omega: interval.omega_key,
                ideal: interval.ideal(),
                hook_y: self.anchor_y,
            };
            self.fabric.remove_interval(key);
            self.struts.push(strut);
            StateChange::SetStageLabel(format!(
                "Packing — {} struts left",
                Self::struts_in(&self.fabric)
            ))
            .send(&self.radio);
        }
    }
}
