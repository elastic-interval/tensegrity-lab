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
use crate::fabric::interval::{Role, Span};
use crate::fabric::physics::Physics;
use crate::fabric::{BendableCable, Fabric, IntervalKey, JointKey, Level};
use crate::units::{Grams, Meters, Percent, Seconds, Unit};
use crate::{Age, Radio, StateChange};
use glam::Vec3;

/// Hold still this long at the very start (structure standing) before the
/// disassembly begins — time to start a screen recording.
const START_PAUSE: Seconds = Seconds(5.0);

/// How long to hold still and let the structure settle after each strut removal.
/// Also the window during which the next strut's name is shown before it descends,
/// so each removal is a clear, watchable interval (and pausable in a future movie).
const SETTLE_SECONDS: Seconds = Seconds(3.0);

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

/// The low mark: the initial Raise clears the structure to here, and it's the
/// *lower* edge of the corrective-lift hysteresis (start lifting when any strut
/// dips below here).
const MIN_MARK: Meters = Meters(0.50);

/// Upper edge of the corrective-lift hysteresis: once lifting (triggered at
/// `MIN_MARK`), keep going until the lowest strut clears this, so the lift is one
/// smooth move instead of chattering on/off at the threshold.
const CLEAR_TARGET: Meters = Meters(1.00);

/// Hook rise/descend speed for the haul-up and lower-down interlude between
/// teardown and rebuild.
const LIFT_SPEED: f32 = 1.0;

/// Between teardown and rebuild the crane hauls the whole de-strutted mess up
/// until its lowest joint clears this height, then lowers it back down.
const LIFT_CLEAR: Meters = Meters(1.0);

/// During the lower-down, the splay ropes are re-added once a 52 foot comes back
/// within this height of the surface (so the legs are pulled apart again before
/// the first strut goes back in).
const FOOT_TOUCH: Meters = Meters(0.05);

/// After the rebuild, release the hook and let the structure settle onto the
/// surface for this long before returning to viewing.
const DROP_SECONDS: Seconds = Seconds(4.0);

/// Surface hexagon corner radius = this × fabric bounding radius (matches the
/// rendered surface, `surface_renderer.rs`).
const SURFACE_HEX_FACTOR: f32 = 1.5;

/// Corner tethers: a *visible* `Pulling` cable runs from a pinned anchor at each
/// hexagon corner to that leg's 52 foot joint. While the foot is slack (its strut
/// gone) the cable is held at `TETHER_STRAIN` strain, a steady pull drawing the
/// leg out toward the corner; while still strutted it's slack (no tension). Three
/// symmetric corners → three balanced pulls → no tipping. Tunable: bigger strain
/// or stiffness = firmer pull.
const TETHER_STRAIN: f32 = 0.02;
const TETHER_STIFFNESS: Percent = Percent(0.1);
/// Mass tag on the pinned corner-anchor joints. Any `Some` keeps them out of the
/// centroid (so the camera ignores them); the joints are pinned, so it's otherwise
/// unused.
const ANCHOR_MASS: Grams = Grams(1.0);

/// Once the rebuild is done and the splay ropes are gone, a triangle of tension is
/// added between the three 52 feet, easing (Approach) to this ideal side length.
const TRIANGLE_LENGTH: Meters = Meters(5.4);
const TRIANGLE_SECONDS: Seconds = Seconds(3.0);

/// State of the crane's height-correction: it nudges the lowest strut back toward
/// the middle of the [`MIN_MARK`, `CLEAR_TARGET`] band — raising when a strut dips
/// below, lowering when the (shrinking) structure floats above. Hysteresis: once
/// moving, it continues to the band's middle before stopping, so it doesn't
/// chatter at an edge.
#[derive(Clone, Copy, PartialEq)]
enum Clearing {
    Idle,
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    /// Hold still at the start (structure standing) so a screen recording can be
    /// started before anything moves.
    Begin,
    /// Lift the standing structure clear of the surface (above the mark) before
    /// removing any struts, so the teardown begins from a clean hang.
    Raise,
    /// Disassembly: hold the structure aloft, settle, then remove the next strut
    /// in the air (the crane keeps every strut above the mark — no lowering).
    Settle,
    /// All struts gone (splay ropes removed): haul the whole cable-and-cap mess up
    /// clear of the surface.
    Lift,
    /// Pause at the hauled-up hang.
    Hold,
    /// Lower the mess back down to the surface; once the 52 feet touch down the
    /// splay ropes are re-added (legs pulled apart).
    Descend,
    /// Lift the collapsed, splayed mess back up until the next strut to re-insert
    /// clears the mark, so the first strut isn't formed on the ground.
    Mount,
    /// Reassembly (in the air): a just-inserted strut grows to length and takes
    /// shape; when it's done the next strut goes in, the crane keeping all aloft.
    Shape,
    /// Rebuilt: release the hook and let the structure settle onto the surface.
    Drop,
    /// Settled — signal the crucible to return to viewing.
    Done,
}

/// A corner tether: a pinned anchor joint at a visible-hexagon corner, the 52
/// foot joint it pulls on, the corner location (to re-pin the anchor each tick),
/// and the visible `Pulling` interval between them.
#[derive(Clone, Copy)]
struct Tether {
    anchor: JointKey,
    foot: JointKey,
    corner: Vec3,
    interval: IntervalKey,
}

/// A removed strut: its two cap joints and ideal length, kept on a stack so
/// reassembly can re-insert it (in reverse order).
struct Strut {
    alpha: JointKey,
    omega: JointKey,
    ideal: Meters,
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
    /// Struts in logical take-down order — by structural level (feet first, apex
    /// last) with each level's A/B/C triple consecutive, derived from the
    /// symmetric joint labels so the sequence is perfectly regular rather than
    /// following the jittery raw altitude. Reassembly reverses it.
    order: Vec<IntervalKey>,
    /// Index into `order` of the strut currently being taken down.
    next: usize,
    /// Whether the crane is currently nudging its height to keep the lowest strut
    /// in the [`MIN_MARK`, `CLEAR_TARGET`] band.
    clearing: Clearing,
    /// Removed struts, in removal order; popped (reverse order) to re-insert.
    struts: Vec<Strut>,
    /// Cables currently in bendable-chain form (managed by `reconcile_cables`).
    chains: Vec<BendableCable>,
    /// Each 52 foot joint and its fixed hexagon corner — kept so the splay ropes
    /// can be removed (for the haul-up) and re-made (when the feet touch down).
    foot_corners: Vec<(JointKey, Vec3)>,
    /// The corner tethers — visible pull cables from pinned hexagon-corner anchors
    /// to the 52 feet — that draw the slack legs out into three 120°-apart corners.
    /// Empty while the mess is hauled up.
    tethers: Vec<Tether>,
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

        let mut fabric = fabric;
        // The crane hooks the topmost joint.
        let anchor = fabric
            .joints
            .iter()
            .max_by(|(_, a), (_, b)| a.location.y.partial_cmp(&b.location.y).unwrap())
            .map(|(key, _)| key)
            .expect("fabric has joints");
        let anchor_base = fabric.joints[anchor].location;
        let order = logical_strut_order(&fabric);
        let foot_corners = find_feet(&fabric, anchor_base);
        let tethers = make_tethers(&mut fabric, &foot_corners);
        Self {
            anchor_y: anchor_base.y,
            phase: Phase::Begin,
            phase_age: fabric.age,
            order,
            next: 0,
            clearing: Clearing::Idle,
            struts: Vec::new(),
            chains: Vec::new(),
            foot_corners,
            tethers,
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
            Phase::Begin => {
                if elapsed >= START_PAUSE.f32() {
                    self.set_phase(Phase::Raise);
                }
            }
            Phase::Raise => {
                // Lift until the structure clears the mark. Ignore the pinned corner
                // anchors (they sit on the surface, so they'd say "always too low").
                if self.lowest_held_joint() < MIN_MARK.f32() {
                    self.anchor_y += LIFT_SPEED * frame_seconds;
                } else {
                    self.set_phase(Phase::Settle);
                }
            }
            Phase::Settle => {
                // Teardown in the air: settle, then remove the next strut without
                // lowering it (the corrective lift keeps every strut off the ground).
                if elapsed >= SETTLE_SECONDS.f32() {
                    self.remove_target_strut();
                    if Self::struts_in(&self.fabric) == 0 {
                        // Splay ropes off, then haul the whole mess up.
                        self.remove_tethers();
                        self.set_phase(Phase::Lift);
                        StateChange::SetStageLabel("Packed — lifting".to_string())
                            .send(&self.radio);
                    } else {
                        self.set_phase(Phase::Settle);
                    }
                }
            }
            Phase::Lift => {
                // Haul the de-strutted mess up clear of the surface (no ropes now,
                // so the feet aren't pegged to the corners).
                if self.lowest_held_joint() < LIFT_CLEAR.f32() {
                    self.anchor_y += LIFT_SPEED * frame_seconds;
                } else {
                    self.set_phase(Phase::Hold);
                }
            }
            Phase::Hold => {
                // Pause at the hauled-up hang, then lower it back down.
                if elapsed >= HOLD_SECONDS.f32() {
                    self.set_phase(Phase::Descend);
                }
            }
            Phase::Descend => {
                // Lower the mess back to the surface. Re-add the splay ropes the
                // moment a foot touches down (legs pulled apart), but keep lowering
                // the crane until the WHOLE mess has collapsed onto the surface
                // (the hook itself down to the mark) before starting reassembly —
                // otherwise the cables are still up in the air when struts go in.
                if self.tethers.is_empty() && self.lowest_foot_y() <= FOOT_TOUCH.f32() {
                    self.add_tethers();
                }
                if self.anchor_y > MIN_MARK.f32() {
                    self.anchor_y -= LIFT_SPEED * frame_seconds;
                } else {
                    if self.tethers.is_empty() {
                        self.add_tethers();
                    }
                    self.set_phase(Phase::Mount);
                    StateChange::SetStageLabel("Rebuilding".to_string()).send(&self.radio);
                }
            }
            Phase::Mount => {
                // Lift the collapsed mess until the next (apex) strut to re-insert
                // clears the mark, so it isn't formed with a joint on the ground.
                if self.next_strut_low_y() < LIFT_CLEAR.f32() {
                    self.anchor_y += LIFT_SPEED * frame_seconds;
                } else {
                    self.set_phase(Phase::Shape);
                }
            }
            Phase::Shape => {
                // Reassembly in the air: wait (the first wait lets the re-added ropes
                // splay the legs; later waits let each inserted strut take shape),
                // then insert the next strut. When the stack is empty, swap the splay
                // ropes for the base triangle of tension and drop to the surface.
                if elapsed >= INSERT_SECONDS.f32() {
                    if self.insert_next_strut() {
                        self.set_phase(Phase::Shape);
                    } else {
                        self.finish_splay();
                        self.set_phase(Phase::Drop);
                        StateChange::SetStageLabel("Rebuilt — lowering to surface".to_string())
                            .send(&self.radio);
                    }
                }
            }
            Phase::Drop => {
                if elapsed >= DROP_SECONDS.f32() {
                    self.set_phase(Phase::Done);
                }
            }
            Phase::Done => {}
        }

        // Keep the lowest strut inside the [MIN_MARK, CLEAR_TARGET] band, but with a
        // one-way motion per phase: in teardown (Settle) the crane only ever
        // descends (the shrinking structure floats up; follow it down); in
        // reassembly (Shape) it only ever rises (the growing structure reaches down;
        // keep its struts off the ground). Hysteresis: once moving, continue to the
        // band's middle before stopping, so it doesn't chatter.
        if matches!(self.phase, Phase::Settle | Phase::Shape)
            && Self::struts_in(&self.fabric) > 0
        {
            let gap = self.lowest_strut_gap();
            let mid = 0.5 * (MIN_MARK.f32() + CLEAR_TARGET.f32());
            match self.phase {
                Phase::Settle => {
                    if gap > CLEAR_TARGET.f32() {
                        self.clearing = Clearing::Down;
                    }
                    if self.clearing == Clearing::Down {
                        self.anchor_y -= LIFT_SPEED * frame_seconds;
                        if gap <= mid {
                            self.clearing = Clearing::Idle;
                        }
                    }
                }
                Phase::Shape => {
                    if gap < MIN_MARK.f32() {
                        self.clearing = Clearing::Up;
                    }
                    if self.clearing == Clearing::Up {
                        self.anchor_y += LIFT_SPEED * frame_seconds;
                        if gap >= mid {
                            self.clearing = Clearing::Idle;
                        }
                    }
                }
                _ => {}
            }
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
            // Hold each corner anchor fixed at its hexagon corner so the tether
            // pulls only its foot, not itself.
            for idx in 0..self.tethers.len() {
                let t = self.tethers[idx];
                if let Some(joint) = self.fabric.joints.get_mut(t.anchor) {
                    joint.location = t.corner;
                    joint.velocity = Vec3::ZERO;
                }
            }
        }

        // Keep the corner tethers at the right tension: a steady pull on each slack
        // foot toward its corner, none while the foot is still strutted (rigid).
        self.update_tethers();

        // Law of nature: cables follow the struts. Removing a strut frees its
        // cables to become draping chains; re-inserting struts reverts them.
        self.fabric
            .reconcile_cables(&mut self.chains, CABLE_SEGMENT_LENGTH, REVIVE_SECONDS);

        // Show the busy strut's name beside the structure: the one about to be
        // taken down (during its settle), or the one being re-inserted.
        let label = match self.phase {
            Phase::Settle => self.removing_name(),
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

    /// Set each tether's rest length: a slack foot's cable is held shorter than its
    /// actual length (a constant `TETHER_STRAIN` → a steady pull toward the corner);
    /// a still-strutted foot's cable is left at its actual length (no tension), so
    /// the rope only pulls once there's slack to take up. The three symmetric
    /// corners give three balanced pulls, so the structure isn't shoved sideways.
    fn update_tethers(&mut self) {
        let strutted: Vec<bool> = self
            .tethers
            .iter()
            .map(|t| self.foot_is_strutted(t.foot))
            .collect();
        for idx in 0..self.tethers.len() {
            let t = self.tethers[idx];
            let actual = self.fabric.distance(t.anchor, t.foot).f32();
            let rest = if strutted[idx] {
                actual
            } else {
                actual / (1.0 + TETHER_STRAIN)
            };
            if let Some(interval) = self.fabric.intervals.get_mut(t.interval) {
                interval.span = Span::Fixed {
                    length: Meters(rest),
                };
            }
        }
    }

    /// (Re)make the corner splay ropes from the stored feet+corners.
    fn add_tethers(&mut self) {
        self.tethers = make_tethers(&mut self.fabric, &self.foot_corners);
    }

    /// Remove the corner splay ropes (anchors + their pull cables).
    fn remove_tethers(&mut self) {
        for t in std::mem::take(&mut self.tethers) {
            self.fabric.remove_joint(t.anchor); // cascades the rope's pull cable too
        }
    }

    /// Lowest joint of the next strut to be re-inserted (top of the stack), or
    /// +∞ if the stack is empty — used to lift the mess clear before re-inserting.
    fn next_strut_low_y(&self) -> f32 {
        match self.struts.last() {
            Some(strut) => {
                let joints = &self.fabric.joints;
                joints[strut.alpha]
                    .location
                    .y
                    .min(joints[strut.omega].location.y)
            }
            None => f32::INFINITY,
        }
    }

    /// Lowest of the three 52 feet (used to tell when the lowered mess touches down).
    fn lowest_foot_y(&self) -> f32 {
        self.foot_corners
            .iter()
            .map(|&(foot, _)| self.fabric.joints[foot].location.y)
            .fold(f32::INFINITY, f32::min)
    }

    /// Once the rebuild is complete: remove the splay ropes and, in their place, add
    /// a triangle of tension between the three 52 feet, easing (Approach) to
    /// `TRIANGLE_LENGTH`.
    fn finish_splay(&mut self) {
        self.remove_tethers();
        let feet: Vec<JointKey> = self.foot_corners.iter().map(|&(foot, _)| foot).collect();
        for i in 0..feet.len() {
            let a = feet[i];
            let b = feet[(i + 1) % feet.len()];
            self.fabric.create_approaching_interval(
                a,
                b,
                TRIANGLE_LENGTH,
                Role::Pulling,
                TRIANGLE_SECONDS,
            );
        }
    }

    /// True while the given foot joint still carries a structural strut (rigid,
    /// not slack).
    fn foot_is_strutted(&self, key: JointKey) -> bool {
        self.fabric.intervals.values().any(|iv| {
            iv.role == Role::Pushing
                && iv.level == Level::Structural
                && (iv.alpha_key == key || iv.omega_key == key)
        })
    }

    /// Name of a strut, from the labels of its two cap joints.
    fn strut_name(&self, alpha: JointKey, omega: JointKey) -> String {
        format!(
            "{}-{}",
            self.fabric.joint_label(alpha),
            self.fabric.joint_label(omega)
        )
    }

    /// The strut currently being taken down: the next one in the logical order.
    fn removing_key(&self) -> Option<IntervalKey> {
        self.order.get(self.next).copied()
    }

    /// Name of the strut currently being taken down, if any.
    fn removing_name(&self) -> Option<String> {
        let key = self.removing_key()?;
        let iv = &self.fabric.intervals[key];
        let (alpha, omega) = (iv.alpha_key, iv.omega_key);
        Some(self.strut_name(alpha, omega))
    }

    /// True once the rebuilt structure has settled — time to return to viewing.
    pub fn is_complete(&self) -> bool {
        self.phase == Phase::Done
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_age = self.fabric.age;
        self.clearing = Clearing::Idle;
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

    /// Lowest joint altitude, ignoring the pinned corner anchors (which sit on the
    /// surface and would otherwise peg the minimum at zero forever).
    fn lowest_held_joint(&self) -> f32 {
        self.fabric
            .joints
            .iter()
            .filter(|(key, _)| !self.tethers.iter().any(|t| t.anchor == *key))
            .map(|(_, joint)| joint.location.y)
            .fold(f32::INFINITY, f32::min)
    }

    /// Pop the next strut off the stack and re-insert it (growing to ideal length
    /// over `INSERT_SECONDS`); returns false when the stack is empty. Reassembly is
    /// the reverse of take-down, so cables revive as their struts return.
    fn insert_next_strut(&mut self) -> bool {
        let Some(strut) = self.struts.pop() else {
            return false;
        };
        self.fabric.create_approaching_interval(
            strut.alpha,
            strut.omega,
            strut.ideal,
            Role::Pushing,
            INSERT_SECONDS,
        );
        self.shaping_label = Some(self.strut_name(strut.alpha, strut.omega));
        StateChange::SetStageLabel(format!("Rebuilding — {} struts left", self.struts.len()))
            .send(&self.radio);
        true
    }

    /// Remove the current target strut, pushing it onto the stack for replay, and
    /// advance to the next strut in the logical order.
    fn remove_target_strut(&mut self) {
        if let Some(key) = self.removing_key() {
            let interval = &self.fabric.intervals[key];
            let strut = Strut {
                alpha: interval.alpha_key,
                omega: interval.omega_key,
                ideal: interval.ideal(),
            };
            self.fabric.remove_interval(key);
            self.struts.push(strut);
            self.next += 1;
            StateChange::SetStageLabel(format!(
                "Packing — {} struts left",
                Self::struts_in(&self.fabric)
            ))
            .send(&self.radio);
        }
    }
}

/// Make the corner splay ropes: for each (foot, corner) pair, a pinned anchor
/// joint at the corner and a `Pulling` cable from it to the foot.
fn make_tethers(fabric: &mut Fabric, foot_corners: &[(JointKey, Vec3)]) -> Vec<Tether> {
    foot_corners
        .iter()
        .map(|&(foot, corner)| {
            let anchor = fabric.create_joint(corner);
            fabric.joints[anchor].point_mass = Some(ANCHOR_MASS);
            let length = fabric.distance(anchor, foot);
            let interval = fabric.create_fixed_interval(anchor, foot, Role::Pulling, length);
            if let Some(iv) = fabric.intervals.get_mut(interval) {
                iv.stiffness = TETHER_STIFFNESS;
                iv.level = Level::Cable; // so `reconcile_cables` leaves it alone
            }
            Tether {
                anchor,
                foot,
                corner,
                interval,
            }
        })
        .collect()
}

/// Find each leg's foot joint (the lowest structural joint per leg letter in the
/// standing pose, e.g. `A51`/`A52`, whichever is lower) and the actual visible-
/// hexagon corner it should be pulled to: the nearest of the six surface-hexagon
/// corners (k·60° at the corner radius about the central `axis`). Symmetry-
/// enforced, so the three feet snap to three alternating corners, 120° apart.
fn find_feet(fabric: &Fabric, axis: Vec3) -> Vec<(JointKey, Vec3)> {
    use std::collections::HashMap;
    let corner_radius = SURFACE_HEX_FACTOR * fabric.bounding_radius();
    // Per leg letter, the lowest structural joint.
    let mut lowest: HashMap<char, (JointKey, f32)> = HashMap::new();
    for (key, joint) in fabric.joints.iter() {
        if joint.point_mass.is_some() {
            continue; // cable-chain joint (none exist yet at construction)
        }
        let label = fabric.joint_label(key);
        let Some(leg) = label.chars().next().filter(|c| ('A'..='C').contains(c)) else {
            continue; // on-axis (Z…) or unlabelled — not a leg foot
        };
        let y = joint.location.y;
        lowest
            .entry(leg)
            .and_modify(|best| {
                if y < best.1 {
                    *best = (key, y);
                }
            })
            .or_insert((key, y));
    }
    // The six actual corners of the visible surface hexagon (same geometry as
    // `surface_vertex.rs`: angle = k·60°, point = R·(cos, sin), centred on the
    // structure's vertical axis). Each foot is paired with its nearest one so the
    // legs land on the corners you can see, not on the edges between them.
    let corners: [Vec3; 6] = std::array::from_fn(|k| {
        let angle = k as f32 * std::f32::consts::PI / 3.0;
        Vec3::new(
            axis.x + corner_radius * angle.cos(),
            0.0,
            axis.z + corner_radius * angle.sin(),
        )
    });
    lowest
        .into_values()
        .map(|(key, _)| {
            let loc = fabric.joints[key].location;
            let foot_xz = Vec3::new(loc.x, 0.0, loc.z);
            let corner = *corners
                .iter()
                .min_by(|a, b| {
                    a.distance_squared(foot_xz)
                        .partial_cmp(&b.distance_squared(foot_xz))
                        .unwrap()
                })
                .unwrap();
            (key, corner)
        })
        .collect()
}

/// Order the struts for take-down: by structural level (feet first, apex last)
/// and, within a level, by position with the three A/B/C copies consecutive.
/// Derived purely from the symmetric joint labels (`<leg><brick><position>`), so
/// it's a clean regular sequence rather than the jittery raw-altitude order.
/// Falls back to altitude order if the labels don't follow that scheme (e.g. a
/// non-OpenClaw fabric).
fn logical_strut_order(fabric: &Fabric) -> Vec<IntervalKey> {
    struct Keyed {
        key: IntervalKey,
        order: Option<(u8, u32, u32, u8)>,
        low_y: f32,
    }
    let joints = &fabric.joints;
    let mut struts: Vec<Keyed> = fabric
        .intervals
        .iter()
        .filter(|(_, iv)| iv.role == Role::Pushing && iv.level == Level::Structural)
        .map(|(key, iv)| {
            let label_a = fabric.joint_label(iv.alpha_key);
            let label_b = fabric.joint_label(iv.omega_key);
            let low_y = joints[iv.alpha_key]
                .location
                .y
                .min(joints[iv.omega_key].location.y);
            Keyed {
                key,
                order: strut_order_key(&label_a, &label_b),
                low_y,
            }
        })
        .collect();

    if struts.iter().all(|s| s.order.is_some()) {
        struts.sort_by(|a, b| a.order.unwrap().cmp(&b.order.unwrap()));
    } else {
        // Labels don't follow the symmetric scheme — keep the old behaviour.
        struts.sort_by(|a, b| a.low_y.partial_cmp(&b.low_y).unwrap());
    }
    struts.into_iter().map(|s| s.key).collect()
}

/// Sort key for a strut from its two joint labels:
/// `(level, min_pos, max_pos, leg)` where `level` puts the feet (high brick
/// number) first and the on-axis apex last, and `leg` (0/1/2 = A/B/C) keeps a
/// level's three rotational copies consecutive. `None` if the labels don't parse
/// as a within-leg, within-brick strut.
fn strut_order_key(a: &str, b: &str) -> Option<(u8, u32, u32, u8)> {
    // On-axis apex struts (labels like "Z1"): always last.
    if a.starts_with('Z') || b.starts_with('Z') {
        return Some((u8::MAX, 0, 0, 0));
    }
    let (leg_a, brick_a, pos_a) = parse_off_axis_label(a)?;
    let (leg_b, brick_b, pos_b) = parse_off_axis_label(b)?;
    if leg_a != leg_b || brick_a != brick_b {
        return None;
    }
    // Feet have the highest brick number and come down first; subtract from a
    // value above any real brick so larger bricks sort earlier.
    let level = 200u8.saturating_sub(brick_a);
    let (lo, hi) = (pos_a.min(pos_b), pos_a.max(pos_b));
    // The seed hub (brick 0) numbers positions by altitude with 1 = top, so its
    // lower ring (higher position numbers) must come down before its upper ring.
    // Column bricks number by build order, where ascending reads naturally.
    let (lo, hi) = if brick_a == 0 {
        (u32::MAX - lo, u32::MAX - hi)
    } else {
        (lo, hi)
    };
    Some((level, lo, hi, leg_a))
}

/// Parse an off-axis label `<leg><brick><position>` (e.g. `A41`) into
/// `(leg, brick, position)`; `None` if it isn't that shape.
fn parse_off_axis_label(label: &str) -> Option<(u8, u8, u32)> {
    let bytes = label.as_bytes();
    if bytes.len() < 3 {
        return None;
    }
    let leg = match bytes[0] {
        b'A' => 0,
        b'B' => 1,
        b'C' => 2,
        _ => return None,
    };
    let brick = (bytes[1] as char).to_digit(10)? as u8;
    let position: u32 = label[2..].parse().ok()?;
    Some((leg, brick, position))
}
