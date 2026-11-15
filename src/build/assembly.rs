//! Headless Hug taken apart strut by strut, and put together again.
//!
//! Put together, it grows lying on its belly from the pelvis outwards,
//! without arms. When the shoulders are complete the crane raises them by
//! L25 and R25, the arms are built hanging, each strut within reach of the
//! ground, the figure is stood on its feet,
//! the hook brought back over it and the guy lines put on. Taking it apart
//! is the same backwards, and comes first.
//!
//! Twins come out and go in together. The hook stays on throughout: with
//! the last strut out it lifts the net clear of the ground, and putting
//! together starts by letting that net down while lines from stakes in the
//! ground to its feet and hands are reeled in, so that it comes down drawn
//! out.
//!
//! The ground is slippery throughout. The feet are held only while they are
//! on their mounts: let go once the hook carries the figure, and drawn back
//! to where they stood before the hook is brought back over it. The guy
//! lines go back on the anchors they came off.
//!
//! Only struts come out: the cables and node plates stay, with the struts'
//! caps on them, a connected net that lies slack where its struts are gone. A strut is drawn in until it
//! carries nothing, then taken out; it goes back in at that length, where it
//! came out, and is lengthened to its own. Each stage is the whole figure
//! with the struts not yet in taken out, so parts keep their keys. The
//! ground and the hook are the building site's (`fabric/site.rs`). Results
//! are in `docs/crane-lift.md`.

use crate::fabric::interval::{Role, Span};
use crate::fabric::joint::JointLabel;
use crate::fabric::physics::Physics;
use crate::fabric::site::{Ground, Hook, Site};
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Percent, Seconds, Unit};
use crate::Age;
use glam::Vec3;
use slotmap::SecondaryMap;
use std::collections::VecDeque;

const EYES: [&str; 2] = ["L25", "R25"];
const FEET: [&str; 2] = ["L54", "R54"];
const LEG: Meters = Meters(4.0);
/// In metres per second.
const HOOK_SPEED: f32 = 0.25;
/// A new cable is drawn to its length in this long, and the figure is given
/// `SETTLING` in all before the next strut.
const FITTING: Seconds = Seconds(1.5);
const SETTLING: Seconds = Seconds(3.0);
/// A strut coming out is drawn in to this much of its length, over this long
/// at most.
const SHORTENED: f32 = 0.9;
const SHORTENING: Seconds = Seconds(4.0);
const SPREADING: Seconds = Seconds(30.0);
const EASING: Seconds = Seconds(5.0);
/// The legs are hauled out first, over this long, by lines from stakes this
/// far beyond where the feet lay: the net then comes down with its legs
/// already apart. The lines to the hand tips, from stakes this far beyond,
/// are only reeled in, at this many metres per second, while they are
/// stretched less than `JUST_TAUT`.
const HAULING: Seconds = Seconds(12.0);
const FOOT_LINE: Meters = Meters(8.0);
const HAND_LINE: Meters = Meters(3.0);
const REELING: f32 = 0.5;
const JUST_TAUT: f32 = 0.0005;
const RESTING: Seconds = Seconds(10.0);
const SWINGING: Seconds = Seconds(30.0);
/// An arm strut goes on or comes off with its lower end this near the ground.
const WITHIN_REACH: Meters = Meters(0.15);
const STANDING_HEIGHT: Meters = Meters(7.0);
/// A joint this high is off the ground.
const CLEAR: Meters = Meters(0.05);
/// The hook stops looking for the height or the ground after this long.
const HOOK_TRAVEL: Seconds = Seconds(48.0);
/// The legs are slack once they have carried nothing for this long.
const SLACK: Seconds = Seconds(0.2);

type Pose = SecondaryMap<JointKey, Vec3>;

fn ticks(seconds: Seconds) -> usize {
    (seconds / Age::iteration_duration()) as usize
}

fn joint(fabric: &Fabric, name: &str) -> JointKey {
    fabric.joint_key_by_label(name).unwrap_or_else(|| panic!("no joint {name}"))
}

fn middle(fabric: &Fabric, names: [&str; 2]) -> Vec3 {
    (fabric.location(joint(fabric, names[0])) + fabric.location(joint(fabric, names[1]))) / 2.0
}

fn is_arm(fabric: &Fabric, key: JointKey) -> bool {
    let path = fabric.joints[key].path.to_string();
    path.starts_with("CX1ACX") || path.starts_with("DX1ACX")
}

/// The struts in the order they go in: the pelvis, then outwards brick by
/// brick, legs before shoulders at the same distance, twins together; then
/// the arms the same way. With how many come before the arms, and how many
/// are in once each strut or pair of twins is.
fn build_order(whole: &Fabric) -> (Vec<IntervalKey>, usize, Vec<usize>) {
    let mut struts: Vec<((bool, usize, u8, u16, u8), IntervalKey)> = whole
        .intervals
        .iter()
        .filter(|(_, interval)| interval.has_role(Role::Pushing))
        .map(|(key, interval)| {
            let ends = [interval.alpha_key, interval.omega_key];
            let rank = |end: JointKey| (is_arm(whole, end), whole.joints[end].path.branches.len());
            let outer = if rank(ends[0]) >= rank(ends[1]) { ends[0] } else { ends[1] };
            let limb = whole.joints[outer].path.branches.first().map_or(0, |&branch| branch / 2);
            let labels = ends.map(|end| whole.joints[end].label);
            let number = labels
                .iter()
                .filter_map(|label| match label {
                    Some(JointLabel::Twin { number, .. }) => Some(*number),
                    _ => None,
                })
                .min()
                .unwrap_or(1000);
            let right = labels.iter().any(|label| matches!(label, Some(JointLabel::Twin { letter: 'R', .. }))) as u8;
            let (arm, depth) = rank(outer);
            ((arm, depth, limb, number, right), key)
        })
        .collect();
    struts.sort_by_key(|(order, _)| *order);
    let before_arms = struts.iter().filter(|((arm, ..), _)| !arm).count();
    let together = |index: usize| {
        let ((arm, depth, limb, number, _), _) = struts[index];
        (arm, depth, limb, number)
    };
    // A strut on the mirror plane has no twin.
    let twins = |count: usize| together(count) == together(count - 1) && together(count).3 != 1000;
    let stops = (1..=struts.len()).filter(|&count| count == struts.len() || !twins(count)).collect();
    (struts.into_iter().map(|(_, key)| key).collect(), before_arms, stops)
}

/// The whole figure with only the struts `placed`: every cable and node
/// plate stays, a slack net where its struts are out, and so do the caps
/// of the struts that are out (`Interval::is_out`).
fn partial(whole: &Fabric, placed: &[IntervalKey]) -> Fabric {
    let mut fabric = whole.clone();
    for (key, interval) in fabric.intervals.iter_mut() {
        if interval.has_role(Role::Pushing) && !placed.contains(&key) {
            interval.stiffness = Percent(0.0);
        }
    }
    fabric
}

/// Put `next` where `before` is, or where `pose` has it: a slack net on
/// the ground does not show which side of a cable a strut goes, so a strut
/// goes back in where it came out. A strut `before` lacks goes in at the
/// length its ends span and is brought to its own over `FITTING`.
fn carry_over(before: &Fabric, next: &mut Fabric, pose: Option<&Pose>) {
    next.age = before.age;
    for (key, joint) in next.joints.iter_mut() {
        joint.velocity = Vec3::ZERO;
        joint.location = pose.and_then(|pose| pose.get(key).copied()).unwrap_or(before.location(key));
    }
    let new: Vec<IntervalKey> = next
        .intervals
        .iter()
        .filter(|(key, interval)| !interval.is_out() && before.intervals.get(*key).is_none_or(|was| was.is_out()))
        .map(|(key, _)| key)
        .collect();
    for key in new {
        let (ideal, spanned) = (next.intervals[key].ideal(), next.intervals[key].length(&next.joints));
        next.intervals[key].span = Span::Fixed { length: spanned };
        next.extend_interval(key, ideal, FITTING);
    }
}

fn pose_of(fabric: &Fabric) -> Pose {
    fabric.joints.iter().map(|(key, joint)| (key, joint.location)).collect()
}

/// The feet, held where they are: on their mounts.
fn feet_held(fabric: &Fabric) -> Vec<(JointKey, Vec3)> {
    FEET.iter().map(|name| joint(fabric, name)).map(|foot| (foot, fabric.location(foot))).collect()
}

fn eyes(fabric: &Fabric) -> Vec<JointKey> {
    EYES.iter().map(|name| joint(fabric, name)).collect()
}

/// Where the hook holds the figure as upright as it was built. The figure
/// balances on its feet, so straight above the eyes won't do: it can lean
/// several degrees before the legs come taut. On the line from the feet
/// through the eyes, the legs are shortest when it is upright and hold it
/// there both ways.
fn hook_upright(whole: &Fabric, fabric: &Fabric) -> Vec3 {
    let axis = (middle(whole, EYES) - middle(whole, FEET)).normalize();
    let feet = middle(fabric, FEET);
    let eyes = eyes(fabric);
    let aside = fabric.location(eyes[0]).distance(fabric.location(eyes[1])) / 2.0;
    let reach = middle(fabric, EYES).distance(feet) + (LEG.f32() * LEG.f32() - aside * aside).sqrt();
    feet + axis * reach
}

/// How high the lower end of `strut` is.
fn lower_end(fabric: &Fabric, strut: IntervalKey) -> f32 {
    let strut = &fabric.intervals[strut];
    fabric.location(strut.alpha_key).y.min(fabric.location(strut.omega_key).y)
}

/// What is done to the figure when a step begins.
enum Change {
    Nothing,
    /// The hook on, held where its legs come taut, the guy lines still on.
    HookOn,
    GuysOff,
    /// The hook taken, over the step, to where its legs hold the figure as
    /// upright as it was built.
    Upright,
    /// The feet off their mounts, free to slide, and the hook kept straight
    /// above the eyes.
    FeetOff,
    /// The feet drawn, over the step, to the mounts they stood on, and
    /// fixed there.
    FeetOn,
    /// These struts drawn in, until they carry nothing and can come out.
    Shorten(usize, usize),
    /// Down to this many struts.
    Take(usize),
    /// The hanging net let down while lines from stakes in the ground to
    /// its feet and hands are reeled in, kept just taut, so that it comes
    /// down drawn out, legs and arms apart.
    Spread,
    /// The lines off, and the net eased the rest of the way to where it lay
    /// when the last strut came out.
    LaidOut,
    /// Up to this many struts.
    Add(usize),
    /// The guy lines on and drawn in, still on the hook.
    GuysOn,
    HookOff,
    /// The site left: back on the frozen surface.
    Stand,
}

enum Until {
    After(Seconds),
    /// The eyes this high above the ground.
    EyesAt(Meters),
    /// The hook has let this strut down to where its lower end is
    /// `WITHIN_REACH` of the ground, or has gone slack.
    InReach(usize),
    LegsSlack,
    /// These struts carry nothing.
    StrutSlack(usize, usize),
    /// Nothing touches the ground.
    OffTheGround,
}

struct Step {
    label: String,
    change: Change,
    /// How fast the hook rises, in metres per second.
    climb: f32,
    until: Until,
}

/// A line from a stake to a foot or a hand, laying the net out.
struct Line {
    key: IntervalKey,
    /// In metres, as it starts and as it ends.
    from: f32,
    to: f32,
    /// Hauled in steadily over `HAULING`, whatever it takes; if not, reeled
    /// in only while barely taut.
    hauled: bool,
}

struct Running {
    label: String,
    until: Until,
    ticks: usize,
    slack_ticks: usize,
    /// The hook's way from where it was to where it is going.
    sweep: Option<(Vec3, Vec3)>,
    /// The net eased from here to where it lay.
    spread: Option<Pose>,
    /// Lines being reeled in, each with the length it ends at.
    lines: Vec<Line>,
    /// The feet drawn from here to their mounts.
    mounting: Option<Vec<(JointKey, Vec3)>>,
    /// A strut drawn in: where everything then is is kept for putting it back.
    kept: Option<usize>,
}

pub struct Assembly {
    /// As built, standing with its guy lines.
    stood: Fabric,
    /// The same without the guy lines and their ground anchors.
    whole: Fabric,
    anchors: Vec<(JointKey, Vec3)>,
    order: Vec<IntervalKey>,
    before_arms: usize,
    /// How many struts are in once each strut or pair of twins is.
    stops: Vec<usize>,
    /// Where everything was when the strut of this index, drawn in, came out.
    poses: Vec<Pose>,
    placed: usize,
    steps: VecDeque<Step>,
    running: Option<Running>,
}

impl Assembly {
    /// Of the figure `stood` as built.
    pub fn of(stood: &Fabric) -> Assembly {
        let mut stood = stood.clone();
        (stood.wind, stood.quake, stood.site) = (None, None, None);
        stood.zero_velocities();
        let anchors: Vec<(JointKey, Vec3)> = stood
            .intervals
            .values()
            .filter(|interval| interval.has_role(Role::GuyLine))
            .flat_map(|interval| [interval.alpha_key, interval.omega_key])
            .filter(|&end| stood.push_at(end).is_none())
            .map(|anchor| (anchor, stood.location(anchor)))
            .collect();
        let mut whole = stood.clone();
        for &(anchor, _) in &anchors {
            whole.remove_joint(anchor);
        }
        let (order, before_arms, stops) = build_order(&whole);
        let poses = vec![Pose::new(); order.len()];
        let placed = order.len();
        Assembly { stood, whole, anchors, order, before_arms, stops, poses, placed, steps: VecDeque::new(), running: None }
    }

    /// Heavier drag than viewing's: a hanging figure swings for a long time.
    pub fn physics(viewing: &Physics) -> Physics {
        Physics { drag: 3.0, ..viewing.clone() }
    }

    pub fn strut_count(&self) -> usize {
        self.order.len()
    }

    /// Nothing left to do: taken apart, or standing again.
    pub fn is_idle(&self) -> bool {
        self.running.is_none() && self.steps.is_empty()
    }

    /// Every strut out.
    pub fn is_apart(&self) -> bool {
        self.is_idle() && self.placed == 0
    }

    /// What is being done now.
    pub fn label(&self) -> Option<&str> {
        self.running.as_ref().map(|running| running.label.as_str())
    }

    /// How many ticks the step under way still takes, if it takes a set time.
    pub fn remaining_ticks(&self) -> Option<usize> {
        let running = self.running.as_ref()?;
        match running.until {
            Until::After(seconds) => Some(ticks(seconds).saturating_sub(running.ticks)),
            _ => None,
        }
    }

    fn step(&mut self, label: impl Into<String>, change: Change, until: Until) {
        self.steps.push_back(Step { label: label.into(), change, climb: 0.0, until });
    }

    fn hoist(&mut self, label: &str, change: Change, climb: f32, until: Until) {
        self.steps.push_back(Step { label: label.to_string(), change, climb, until });
    }

    /// The struts `from..to`, twins together.
    fn strut_label(&self, sign: char, from: usize, to: usize) -> String {
        let names: Vec<String> = self.order[from..to]
            .iter()
            .map(|&key| {
                let strut = &self.whole.intervals[key];
                format!("{}–{}", self.whole.joint_label(strut.alpha_key), self.whole.joint_label(strut.omega_key))
            })
            .collect();
        format!("{sign} {}", names.join(", "))
    }

    /// The tips of the hands: the outer ends of the last struts to go in.
    fn hand_tips(&self) -> Vec<JointKey> {
        let (from, to) = *self.groups().last().expect("struts");
        let number = |key: JointKey| match self.whole.joints[key].label {
            Some(JointLabel::Twin { number, .. }) => number,
            _ => 0,
        };
        self.order[from..to]
            .iter()
            .map(|&hand| {
                let strut = &self.whole.intervals[hand];
                if number(strut.alpha_key) > number(strut.omega_key) { strut.alpha_key } else { strut.omega_key }
            })
            .collect()
    }

    /// Each strut or pair of twins, as the indices `from..to`, in the order
    /// they go in.
    fn groups(&self) -> Vec<(usize, usize)> {
        let from = std::iter::once(0).chain(self.stops.iter().copied());
        from.zip(self.stops.iter().copied()).collect()
    }

    /// From standing on its guy lines down to the net alone, lifted clear of
/// the ground on the hook.
    pub fn take_apart(&mut self) {
        use Until::*;
        self.step("hook on", Change::HookOn, After(Seconds(2.0)));
        self.step("guy lines off", Change::GuysOff, After(RESTING));
        self.step("feet let go", Change::FeetOff, After(Seconds(1.0)));
        let (groups, before_arms) = (self.groups(), self.before_arms);
        for &(from, to) in groups.iter().rev().filter(|group| group.0 >= before_arms) {
            self.hoist("let down", Change::Nothing, -HOOK_SPEED, InReach(from));
            self.step("drawn in", Change::Shorten(from, to), StrutSlack(from, to));
            self.step(self.strut_label('−', from, to), Change::Take(from), After(SETTLING));
        }
        self.hoist("shoulders let down", Change::Nothing, -HOOK_SPEED, LegsSlack);
        self.step("lying", Change::Nothing, After(RESTING));
        for &(from, to) in groups.iter().rev().filter(|group| group.0 < before_arms) {
            self.step("drawn in", Change::Shorten(from, to), StrutSlack(from, to));
            self.step(self.strut_label('−', from, to), Change::Take(from), After(SETTLING));
        }
        self.hoist("net lifted", Change::Nothing, HOOK_SPEED, OffTheGround);
        self.step("net hanging", Change::Nothing, After(SETTLING));
    }

    /// From the net alone, where taking apart left it, to standing.
    pub fn put_together(&mut self) {
        use Until::*;
        self.step("net let down, drawn out", Change::Spread, After(SPREADING));
        self.step("net laid out", Change::LaidOut, After(EASING));
        let (groups, before_arms) = (self.groups(), self.before_arms);
        for &(from, to) in groups.iter().filter(|group| group.0 < before_arms) {
            self.step(self.strut_label('+', from, to), Change::Add(to), After(SETTLING));
        }
        self.step("shoulders complete", Change::Nothing, After(RESTING));
        for &(from, to) in groups.iter().filter(|group| group.0 >= before_arms) {
            let eyes = eyes(&self.whole).iter().map(|&eye| self.poses[to - 1][eye].y).sum::<f32>() / EYES.len() as f32;
            self.hoist("raised", Change::Nothing, HOOK_SPEED, EyesAt(Meters(eyes)));
            self.step(self.strut_label('+', from, to), Change::Add(to), After(SETTLING));
        }
        self.step("arms complete", Change::Nothing, After(RESTING));
        self.hoist("stood up", Change::Nothing, HOOK_SPEED, EyesAt(STANDING_HEIGHT));
        self.step("feet to their mounts", Change::FeetOn, After(RESTING));
        self.step("hook brought back", Change::Upright, After(SWINGING));
        self.step("upright", Change::Nothing, After(RESTING));
        self.step("guy lines on", Change::GuysOn, After(SWINGING));
        self.step("hook off", Change::HookOff, After(RESTING));
        self.step("standing", Change::Stand, After(Seconds(2.0)));
    }

    /// `next` in place of `fabric`, on the same hook if it is on one.
    fn replace(&self, fabric: &mut Fabric, mut next: Fabric) {
        let hook = fabric.site.take().and_then(|site| site.hook);
        next.age = fabric.age;
        let hook = hook.map(|hook| Hook::at(&mut next, hook.eyes, LEG, hook.height, hook.place));
        next.site = Some(Site { ground: Ground::Sliding, hook, anchors: Vec::new() });
        *fabric = next;
    }

    fn begin(&mut self, step: Step, fabric: &mut Fabric) {
        let mut sweep = None;
        let mut spread = None;
        let mut mounting = None;
        let mut lines = Vec::new();
        let kept = match step.change {
            Change::Shorten(_, to) => Some(to - 1),
            _ => None,
        };
        match step.change {
            Change::Nothing => {}
            Change::HookOn => {
                let mut hook = Hook::taut(fabric, eyes(fabric), LEG);
                hook.place = Some(fabric.location(hook.joint));
                let anchors = self.anchors.iter().copied().chain(feet_held(fabric)).collect();
                fabric.site = Some(Site { ground: Ground::Sliding, hook: Some(hook), anchors });
            }
            Change::GuysOff => {
                let mut next = partial(&self.whole, &self.order);
                carry_over(fabric, &mut next, None);
                self.replace(fabric, next);
                fabric.site.as_mut().expect("a site").anchors = feet_held(fabric);
            }
            Change::Upright => {
                let hook = fabric.site.as_ref().and_then(|site| site.hook.as_ref()).expect("on the hook");
                sweep = Some((fabric.location(hook.joint), hook_upright(&self.whole, fabric)));
            }
            Change::FeetOff => {
                let site = fabric.site.as_mut().expect("a site");
                site.anchors.clear();
                if let Some(hook) = site.hook.as_mut() {
                    hook.place = None;
                }
            }
            Change::FeetOn => {
                let feet = feet_held(fabric);
                fabric.site.as_mut().expect("a site").anchors = feet.clone();
                mounting = Some(feet);
            }
            Change::Take(count) => {
                let mut next = partial(&self.whole, &self.order[..count]);
                carry_over(fabric, &mut next, None);
                self.replace(fabric, next);
                self.placed = count;
            }
            Change::Shorten(from, to) => {
                self.poses[to - 1] = pose_of(fabric);
                for &strut in &self.order[from..to] {
                    let shorter = fabric.intervals[strut].ideal() * SHORTENED;
                    fabric.extend_interval(strut, shorter, SHORTENING);
                }
            }
            Change::Spread => {
                let site = fabric.site.as_ref().expect("a site");
                let hook = site.hook.as_ref().expect("on the hook");
                let lain = &self.poses[self.stops[0] - 1];
                let eyes: Vec<Vec3> = hook.eyes.iter().map(|&eye| lain[eye]).collect();
                let aside = eyes[0].distance(eyes[1]) / 2.0;
                let taut = (eyes[0] + eyes[1]) / 2.0 + Vec3::Y * (LEG.f32() * LEG.f32() - aside * aside).sqrt();
                sweep = Some((fabric.location(hook.joint), taut));
                let middle = lain.values().sum::<Vec3>() / lain.len() as f32;
                let feet = FEET.map(|name| (joint(&self.whole, name), FOOT_LINE, true));
                let hands = self.hand_tips().into_iter().map(|tip| (tip, HAND_LINE, false));
                for (end, left, hauled) in feet.into_iter().chain(hands) {
                    let outwards = Vec3::new(lain[end].x - middle.x, 0.0, lain[end].z - middle.z).normalize();
                    let stake = Vec3::new(lain[end].x, 0.0, lain[end].z) + outwards * left.f32();
                    let anchor = fabric.create_joint(stake);
                    let length = fabric.location(end).distance(stake);
                    let line = fabric.create_fixed_interval(end, anchor, Role::GuyLine, Meters(length));
                    fabric.site.as_mut().expect("a site").anchors.push((anchor, stake));
                    lines.push(Line { key: line, from: length, to: left.f32(), hauled });
                }
            }
            Change::LaidOut => {
                let site = fabric.site.as_mut().expect("a site");
                let stakes: Vec<JointKey> = site.anchors.drain(..).map(|(stake, _)| stake).collect();
                for stake in stakes {
                    fabric.remove_joint(stake);
                }
                spread = Some(pose_of(fabric));
            }
            Change::Add(count) => {
                if let Some(hook) = fabric.site.as_mut().and_then(|site| site.hook.as_mut()) {
                    hook.place = None;
                }
                let mut next = partial(&self.whole, &self.order[..count]);
                carry_over(fabric, &mut next, Some(&self.poses[count - 1]));
                self.replace(fabric, next);
                self.placed = count;
            }
            Change::GuysOn => {
                let anchors = self.anchors.clone();
                let mut next = self.stood.clone();
                for (key, joint) in next.joints.iter_mut() {
                    joint.velocity = Vec3::ZERO;
                    if let Some(now) = fabric.joints.get(key) {
                        joint.location = now.location;
                    }
                }
                for &(anchor, at) in &anchors {
                    next.joints[anchor].location = at;
                }
                next.age = fabric.age;
                let guys: Vec<IntervalKey> =
                    next.intervals.iter().filter(|(_, interval)| interval.has_role(Role::GuyLine)).map(|(key, _)| key).collect();
                for key in guys {
                    let (ideal, spanned) = (next.intervals[key].ideal(), next.intervals[key].length(&next.joints));
                    if spanned > ideal {
                        next.intervals[key].span = Span::Fixed { length: spanned };
                        next.extend_interval(key, ideal, RESTING);
                    }
                }
                self.replace(fabric, next);
                fabric.site.as_mut().expect("a site").anchors = anchors.into_iter().chain(feet_held(fabric)).collect();
            }
            Change::HookOff => {
                let site = fabric.site.as_mut().expect("a site");
                if let Some(hook) = site.hook.take() {
                    fabric.remove_joint(hook.joint);
                }
            }
            Change::Stand => fabric.site = None,
        }
        if let Some(hook) = fabric.site.as_mut().and_then(|site| site.hook.as_mut()) {
            hook.climb = step.climb;
        }
        fabric.unfreeze();
        self.running = Some(Running { label: step.label, until: step.until, ticks: 0, slack_ticks: 0, sweep, spread, mounting, lines, kept });
    }

    /// One tick. Gives the label of a step that ended with it.
    pub fn iterate(&mut self, fabric: &mut Fabric, physics: &Physics) -> Option<String> {
        if self.running.is_none() {
            let step = self.steps.pop_front()?;
            self.begin(step, fabric);
        }
        let running = self.running.as_mut().expect("a step under way");
        let hook = fabric.site.as_mut().and_then(|site| site.hook.as_mut());
        if let (Some((from, to)), Until::After(seconds), Some(hook)) = (running.sweep, &running.until, hook) {
            let at = from.lerp(to, (running.ticks + 1) as f32 / ticks(*seconds) as f32);
            (hook.height, hook.place) = (at.y, Some(at));
        }
        if let (Some(feet), Until::After(seconds), Some(site)) = (&running.mounting, &running.until, fabric.site.as_mut()) {
            let along = (running.ticks + 1) as f32 / ticks(*seconds) as f32;
            site.anchors = feet.iter().map(|&(foot, from)| (foot, from.lerp(self.stood.location(foot), along))).collect();
        }
        for line in &running.lines {
            let interval = &mut fabric.intervals[line.key];
            let length = if line.hauled {
                let along = ((running.ticks + 1) as f32 / ticks(HAULING) as f32).min(1.0);
                line.from + (line.to - line.from) * along
            } else if interval.strain < JUST_TAUT {
                (interval.ideal().f32() - REELING / ticks(Seconds(1.0)) as f32).max(line.to)
            } else {
                continue;
            };
            interval.span = Span::Fixed { length: Meters(length) };
        }
        if let (Some(hung), Until::After(seconds)) = (&running.spread, &running.until) {
            let along = (running.ticks + 1) as f32 / ticks(*seconds) as f32;
            for (key, joint) in fabric.joints.iter_mut() {
                if let (Some(&from), Some(&to)) = (hung.get(key), self.poses[self.stops[0] - 1].get(key)) {
                    joint.location = from.lerp(to, along);
                    joint.velocity = Vec3::ZERO;
                }
            }
        }
        fabric.iterate(physics);
        running.ticks += 1;
        let hook = fabric.site.as_ref().and_then(|site| site.hook.as_ref());
        let slack = |hook: Option<&Hook>| {
            hook.is_none_or(|hook| {
                fabric.intervals.values().filter(|interval| interval.touches(hook.joint)).all(|leg| leg.strain <= 0.0)
            })
        };
        let over = match running.until {
            Until::After(seconds) => running.ticks >= ticks(seconds),
            Until::EyesAt(height) => {
                let reached = hook.is_none_or(|hook| Hook::above(fabric, &hook.eyes).y >= height.f32());
                reached || running.ticks >= ticks(HOOK_TRAVEL)
            }
            Until::InReach(index) => {
                running.slack_ticks = if slack(hook) { running.slack_ticks + 1 } else { 0 };
                lower_end(fabric, self.order[index]) <= WITHIN_REACH.f32()
                    || running.slack_ticks >= ticks(SLACK)
                    || running.ticks >= ticks(HOOK_TRAVEL)
            }
            Until::StrutSlack(from, to) => {
                let slack = self.order[from..to].iter().all(|&strut| fabric.intervals[strut].strain >= 0.0);
                running.slack_ticks = if slack { running.slack_ticks + 1 } else { 0 };
                running.slack_ticks >= ticks(SLACK) || running.ticks >= ticks(SHORTENING)
            }
            Until::OffTheGround => {
                fabric.joints.values().all(|joint| joint.location.y > CLEAR.f32()) || running.ticks >= ticks(HOOK_TRAVEL)
            }
            Until::LegsSlack => {
                running.slack_ticks = if slack(hook) { running.slack_ticks + 1 } else { 0 };
                running.slack_ticks >= ticks(SLACK) || running.ticks >= ticks(HOOK_TRAVEL)
            }
        };
        if !over {
            return None;
        }
        let running = self.running.take()?;
        // A strut still carrying when drawn in goes back as it was before.
        if let Some(index) = running.kept.filter(|_| running.slack_ticks >= ticks(SLACK)) {
            self.poses[index] = pose_of(fabric);
        }
        Some(running.label)
    }
}
