//! A figure taken apart strut by strut, and put together again: Headless
//! Hug, and Open Claw.
//!
//! Open Claw is built hanging from the eye on its top, Z2: the strut on its
//! axis first, then outwards and downwards, the three of a turn together,
//! each strut within reach of the ground, its feet last. Headless Hug has
//! more to it. Put together, it grows lying on its belly from the pelvis outwards,
//! without arms. When the shoulders are complete the crane raises them by
//! L25 and R25, the arms are built hanging, each strut within reach of the
//! ground, the figure is stood on its feet,
//! the hook brought back over it and the guy lines put on. Taking it apart
//! is the same backwards, and comes first.
//!
//! Twins come out and go in together. The hook stays on throughout: with
//! the last strut out it lifts the net clear of the ground, and putting
//! together starts by letting that net down. Dragging a net over the ground
//! hardly works, so it is drawn apart while it hangs, by lines from stakes
//! to its feet and hands, and laid down where it is to lie; what the lines
//! leave undone the crew carries to its place.
//!
//! The ground is slippery throughout. The feet are held only while they are
//! on their mounts: let go once the hook carries the figure, and drawn back
//! to where they stood before the hook is brought back over it. The guy
//! lines go back on the anchors they came off.
//!
//! Only struts come out: the cables and node plates stay, with the struts'
//! caps on them, a connected net that lies slack where its struts are gone.
//! A cable none of whose caps holds a strut hangs as a chain and drapes
//! (`fabric/chain.rs`); with a strut back at either end it is a cable again. A strut is drawn in until it
//! carries nothing, then taken out; it goes back in at that length, where it
//! came out, and is lengthened to its own. Each stage is the whole figure
//! with the struts not yet in taken out, so parts keep their keys. The
//! ground and the hook are the building site's (`fabric/site.rs`). Results
//! are in `docs/crane-lift.md`.

use crate::build::dsl::fabric_library::FabricName;
use crate::fabric::chain::Chain;
use crate::fabric::interval::{Role, Span};
use crate::fabric::joint::JointLabel;
use crate::fabric::physics::Physics;
use crate::fabric::site::{Ground, Hook, Site};
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::{Meters, Percent, Seconds, Unit};
use crate::Age;
use glam::{Quat, Vec3};
use slotmap::SecondaryMap;
use std::collections::VecDeque;

const LEG: Meters = Meters(4.0);
/// In metres per second.
const HOOK_SPEED: f32 = 0.25;
/// A new cable is drawn to its length in this long, and the figure is given
/// `SETTLING` in all before the next strut.
const FITTING: Seconds = Seconds(4.0);
const SETTLING: Seconds = Seconds(6.0);
/// A strut coming out is drawn in to this much of its length, over this long
/// at most.
const SHORTENED: f32 = 0.9;
const SHORTENING: Seconds = Seconds(4.0);
const LAYING: Seconds = Seconds(30.0);
/// What the lines cannot do the crew does: every cap not yet where it lay
/// is carried there, lifted this high at most, in this long.
const EASING: Seconds = Seconds(8.0);
const CARRIED: Meters = Meters(0.4);
/// Without its struts the net is far longer than the figure, and dragging
/// it over the ground hardly works, so it is drawn apart while it is let
/// down, by lines from stakes this far beyond where each foot lay, out
/// along the leg (Headless Hug), or stood (Open Claw). A line is reeled in,
/// at so many metres per second, while it is stretched less than its share
/// and paid out when stretched twice it. Headless Hug's hook travels to
/// where the shoulders lay as it comes down, and its foot lines stop with
/// the feet `FEET_PAST` beyond their marks: without its struts a leg is
/// longer than it lay, and drawn that far it lies straight where it lay,
/// so the net is laid from the feet forwards;
/// lines to its hand tips are kept barely taut. Open Claw's legs go three
/// ways and are drawn out wider than they stood, to `STAKE_REACHED`. With
/// the net on the ground the lines come off.
const NEAR_STAKE: Meters = Meters(4.0);
const STAKE_TAUT: f32 = 0.001;
const STAKE_REACHED: Meters = Meters(0.5);
const FEET_PAST: Meters = Meters(0.75);
/// In square metres: caps further from the upright than this show how the
/// figure has turned.
const TURN_SHOWN: f32 = 0.05;
const BRINGING: Seconds = Seconds(3.0);
const HAND_LINE: Meters = Meters(3.0);
const FOOT_REELING: f32 = 1.5;
const HAND_REELING: f32 = 0.5;
const JUST_TAUT: f32 = 0.0005;
/// A slack cable hangs as a chain of segments about this long.
const CHAIN_SEGMENT: Meters = Meters(0.35);
const RESTING: Seconds = Seconds(10.0);
const SWINGING: Seconds = Seconds(30.0);
/// An arm strut goes on or comes off with its lower end this near the ground.
const WITHIN_REACH: Meters = Meters(0.15);
const HEADLESS_HUG_STANDING: Meters = Meters(7.0);
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

fn middle(fabric: &Fabric, joints: &[JointKey]) -> Vec3 {
    joints.iter().map(|&key| fabric.location(key)).sum::<Vec3>() / joints.len() as f32
}

/// How far `points` are from their middle.
fn aside(points: &[Vec3]) -> f32 {
    let middle = points.iter().sum::<Vec3>() / points.len() as f32;
    points.iter().map(|point| point.distance(middle)).sum::<f32>() / points.len() as f32
}

fn twin(fabric: &Fabric, key: JointKey) -> Option<(char, u16)> {
    match fabric.joints[key].label {
        Some(JointLabel::Twin { letter, number }) => Some((letter, number)),
        _ => None,
    }
}

fn is_arm(fabric: &Fabric, key: JointKey) -> bool {
    let path = fabric.joints[key].path.to_string();
    path.starts_with("CX1ACX") || path.starts_with("DX1ACX")
}

/// Open Claw's struts in the order they go in, hanging from its top: the
/// one on the axis, then by joint number, which follows the build outwards,
/// the three of a turn together. None goes in on the ground.
fn hung_order(whole: &Fabric) -> (Vec<IntervalKey>, usize, Vec<usize>) {
    let mut struts: Vec<((u16, char), IntervalKey)> = whole
        .intervals
        .iter()
        .filter(|(_, interval)| interval.has_role(Role::Pushing))
        .map(|(key, interval)| {
            let ends = [interval.alpha_key, interval.omega_key].map(|end| twin(whole, end));
            let number = ends.iter().flatten().map(|&(_, number)| number).min().unwrap_or(0);
            let letter = ends.iter().flatten().map(|&(letter, _)| letter).next().unwrap_or(' ');
            ((number, letter), key)
        })
        .collect();
    struts.sort_by_key(|(order, _)| *order);
    let stops =
        (1..=struts.len()).filter(|&count| count == struts.len() || struts[count].0 .0 != struts[count - 1].0 .0).collect();
    (struts.into_iter().map(|(_, key)| key).collect(), 0, stops)
}

/// Headless Hug's struts in the order they go in: the pelvis, then outwards brick by
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
    /// The feet off their mounts, free to slide, and the hook taken over the
    /// step to straight above the eyes.
    FeetOff,
    /// The feet drawn, over the step, to the mounts they stood on, and
    /// fixed there.
    FeetOn,
    /// The caps of the struts that go in to make this many carried, over
    /// the step, to where the pose they came out in has them (`caps_brought`).
    Bring(usize),
    /// These struts drawn in, until they carry nothing and can come out.
    Shorten(usize, usize),
    /// Down to this many struts.
    Take(usize),
    /// Lines from far stakes to the feet, kept taut from here on.
    LinesOn,
    LinesOff,
    /// The hook taken to where it held the eyes when the last strut came
    /// out, lines to the hands reeled in: the net laid down.
    LayDown,
    /// The net eased the rest of the way to where it lay when the last
    /// strut came out.
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
    /// Neither end of any of these struts touches the ground.
    StrutsClear(usize, usize),
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
    stake: JointKey,
    /// Let out again when stretched twice `taut`.
    paid_out: bool,
    /// In metres, as it ends.
    to: f32,
    /// Reeled in at this many metres per second while stretched less than
    /// `taut`.
    reeling: f32,
    taut: f32,
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
    /// The feet drawn from here to their mounts.
    mounting: Option<Vec<(JointKey, Vec3)>>,
    /// Caps carried from here to there.
    bringing: Option<Vec<(JointKey, Vec3, Vec3)>>,
    /// A strut drawn in: where everything then is is kept for putting it back.
    kept: Option<usize>,
}

pub struct Assembly {
    /// As built, standing with its guy lines.
    stood: Fabric,
    /// The same without the guy lines and their ground anchors.
    whole: Fabric,
    anchors: Vec<(JointKey, Vec3)>,
    /// The lifting eyes the hook's legs go to.
    eyes: Vec<JointKey>,
    feet: Vec<JointKey>,
    /// The hand tips, drawn out by lines as the feet are when the net is
    /// let down.
    hands: Vec<JointKey>,
    /// How high the eyes are when it is stood up.
    standing: Meters,
    order: Vec<IntervalKey>,
    /// How many struts go in on the ground, before the hook raises the rest.
    before_arms: usize,
    /// How many struts are in once each strut or pair of twins is.
    stops: Vec<usize>,
    /// Where everything was when the strut of this index, drawn in, came out.
    poses: Vec<Pose>,
    placed: usize,
    /// The slack cables hanging as chains.
    chains: Vec<Chain>,
    /// Lines to the feet and hands, each with the length it ends at.
    lines: Vec<Line>,
    steps: VecDeque<Step>,
    running: Option<Running>,
}

impl Assembly {
    pub fn suits(name: FabricName) -> bool {
        matches!(name, FabricName::HeadlessHug | FabricName::OpenClaw)
    }

    /// Of the figure `stood` as built from the plan `name`.
    pub fn of(stood: &Fabric, name: FabricName) -> Assembly {
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
        let named = |names: &[&str]| names.iter().map(|name| joint(&whole, name)).collect::<Vec<JointKey>>();
        let (eyes, feet, standing, (order, before_arms, stops)) = match name {
            FabricName::HeadlessHug => (named(&["L25", "R25"]), named(&["L54", "R54"]), HEADLESS_HUG_STANDING, build_order(&whole)),
            FabricName::OpenClaw => {
                let top = joint(&whole, "Z2");
                (vec![top], named(&["A30", "B30", "C30"]), Meters(whole.location(top).y), hung_order(&whole))
            }
            _ => panic!("{name} is not taken apart"),
        };
        let poses = vec![Pose::new(); order.len()];
        let placed = order.len();
        let mut assembly = Assembly {
            stood,
            whole,
            anchors,
            eyes,
            feet,
            hands: Vec::new(),
            standing,
            order,
            before_arms,
            stops,
            poses,
            placed,
            chains: Vec::new(),
            lines: Vec::new(),
            steps: VecDeque::new(),
            running: None,
        };
        if before_arms > 0 {
            assembly.hands = assembly.hand_tips();
        }
        assembly
    }

    /// The feet, held where they are: on their mounts.
    fn feet_held(&self, fabric: &Fabric) -> Vec<(JointKey, Vec3)> {
        self.feet.iter().map(|&foot| (foot, fabric.location(foot))).collect()
    }

    /// Where the hook holds the figure as upright as it was built. The figure
    /// balances on its feet, so straight above the eyes won't do: it can lean
    /// several degrees before the legs come taut. On the line from the feet
    /// through the eyes, the legs are shortest when it is upright and hold it
    /// there both ways.
    fn hook_upright(&self, fabric: &Fabric) -> Vec3 {
        let axis = (middle(&self.whole, &self.eyes) - middle(&self.whole, &self.feet)).normalize();
        let feet = middle(fabric, &self.feet);
        let aside = aside(&self.eyes.iter().map(|&eye| fabric.location(eye)).collect::<Vec<Vec3>>());
        let reach = middle(fabric, &self.eyes).distance(feet) + (LEG.f32() * LEG.f32() - aside * aside).sqrt();
        feet + axis * reach
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
        let number = |key: JointKey| twin(&self.whole, key).map_or(0, |(_, number)| number);
        self.order[from..to]
            .iter()
            .map(|&hand| {
                let strut = &self.whole.intervals[hand];
                if number(strut.alpha_key) > number(strut.omega_key) { strut.alpha_key } else { strut.omega_key }
            })
            .collect()
    }

    /// Hang as a chain every cable none of whose caps holds a strut, and
    /// take the chain down where a strut is back. Fork rods stay as they
    /// are, and so does a cable between two node plates.
    fn drape_slack_cables(&mut self, fabric: &mut Fabric) {
        let held = |fabric: &Fabric, cable: IntervalKey| {
            let cable = &self.whole.intervals[cable];
            let caps: Vec<IntervalKey> =
                [cable.alpha_key, cable.omega_key].iter().filter_map(|&end| self.whole.push_at(end)).collect();
            caps.is_empty() || caps.iter().any(|&strut| !fabric.intervals[strut].is_out())
        };
        let (back, hanging): (Vec<Chain>, Vec<Chain>) =
            std::mem::take(&mut self.chains).into_iter().partition(|chain| held(fabric, chain.cable));
        self.chains = hanging;
        for chain in back {
            fabric.undrape(chain, FITTING);
        }
        let rod = |cable: IntervalKey| self.whole.connector.as_ref().is_some_and(|connector| connector.link_length(cable).is_some());
        let slack: Vec<IntervalKey> = self
            .whole
            .intervals
            .iter()
            .filter(|(key, interval)| interval.role.is_pull_like() && !interval.has_role(Role::GuyLine) && !rod(*key))
            .map(|(key, _)| key)
            .filter(|&cable| !fabric.intervals[cable].is_out() && !held(fabric, cable))
            .collect();
        for cable in slack {
            self.chains.push(fabric.drape(cable, CHAIN_SEGMENT));
        }
    }

    /// Where the caps of the struts going in to make `count` go, from where
    /// they hang. Hanging, the legs are drawn out as they were not when the
    /// struts came out, so the figure hangs shifted and turned; and going in
    /// where its caps hang a ring of struts can twist the figure the wrong
    /// way, into its mirror image. So the pose they came out in is moved and
    /// turned about the upright to lie best on the part already in and the
    /// eyes, and the caps go where it has them.
    ///
    /// Headless Hug's struts go back where they came out, and so does
    /// everything else: lengthening struts in a slack net on the ground
    /// tangles it, so the whole net is brought back to the pose.
    fn caps_brought(&self, fabric: &Fabric, count: usize) -> Vec<(JointKey, Vec3, Vec3)> {
        let mut brought = self.pose_brought(fabric, count);
        // Slack, a strut's caps can drift further apart than its length; it
        // could not go in between them.
        let going_in = self.order[..count].iter().filter(|&&strut| fabric.intervals[strut].is_out());
        for &strut in going_in {
            let (alpha, omega) = (self.whole.intervals[strut].alpha_key, self.whole.intervals[strut].omega_key);
            let at = |key: JointKey, brought: &[(JointKey, Vec3, Vec3)]| brought.iter().position(|&(cap, ..)| cap == key);
            let (Some(a), Some(o)) = (at(alpha, &brought), at(omega, &brought)) else { continue };
            let (from, to) = (brought[a].2, brought[o].2);
            let most = self.whole.intervals[strut].ideal().f32();
            if from.distance(to) > most {
                let (middle, half) = ((from + to) / 2.0, (to - from).normalize() * most / 2.0);
                (brought[a].2, brought[o].2) = (middle - half, middle + half);
            }
        }
        brought
    }

    fn pose_brought(&self, fabric: &Fabric, count: usize) -> Vec<(JointKey, Vec3, Vec3)> {
        let pose = &self.poses[count - 1];
        if self.before_arms > 0 {
            return pose.iter().map(|(joint, &to)| (joint, fabric.location(joint), to)).collect();
        }
        let ends = |strut: IntervalKey| [self.whole.intervals[strut].alpha_key, self.whole.intervals[strut].omega_key];
        let (going_in, already): (Vec<IntervalKey>, Vec<IntervalKey>) =
            self.order[..count].iter().partition(|&&strut| fabric.intervals[strut].is_out());
        let fitted: Vec<JointKey> = already.into_iter().flat_map(ends).chain(self.eyes.iter().copied()).collect();
        let middle = |at: &dyn Fn(JointKey) -> Vec3| fitted.iter().map(|&key| at(key)).sum::<Vec3>() / fitted.len() as f32;
        let (was, now) = (middle(&|key| pose[key]), middle(&|key| fabric.location(key)));
        let (mut across, mut along) = (0.0, 0.0);
        for &key in &fitted {
            let (a, b) = (pose[key] - was, fabric.location(key) - now);
            across += a.z * b.x - a.x * b.z;
            along += a.x * b.x + a.z * b.z;
        }
        // Caps on the axis show no turn; the lines held the legs as they stood.
        let turned = if across.hypot(along) > TURN_SHOWN { f32::atan2(across, along) } else { 0.0 };
        let turn = Quat::from_rotation_y(turned);
        going_in
            .into_iter()
            .flat_map(ends)
            .map(|cap| (cap, fabric.location(cap), now + turn * (pose[cap] - was)))
            .collect()
    }

    /// A line from a stake driven in at `stake` to `end`, reeled in to `to`.
    #[allow(clippy::too_many_arguments)]
    fn line(&mut self, fabric: &mut Fabric, end: JointKey, stake: Vec3, to: Meters, reeling: f32, taut: f32, paid_out: bool) {
        let anchor = fabric.create_joint(stake);
        let length = fabric.location(end).distance(stake);
        let key = fabric.create_fixed_interval(end, anchor, Role::GuyLine, Meters(length));
        fabric.site.as_mut().expect("a site").anchors.push((anchor, stake));
        self.lines.push(Line { key, stake: anchor, paid_out, to: to.f32(), reeling, taut });
    }

    fn lines_off(&mut self, fabric: &mut Fabric) {
        for line in std::mem::take(&mut self.lines) {
            fabric.site.as_mut().expect("a site").anchors.retain(|&(anchor, _)| anchor != line.stake);
            fabric.remove_joint(line.stake);
        }
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
        let (groups, before_arms, guyed) = (self.groups(), self.before_arms, !self.anchors.is_empty());
        self.step("hook on", Change::HookOn, After(Seconds(2.0)));
        if guyed {
            self.step("guy lines off", Change::GuysOff, After(RESTING));
        }
        self.step("feet let go", Change::FeetOff, After(Seconds(1.0)));
        if before_arms == 0 {
            self.hoist("lifted", Change::Nothing, HOOK_SPEED, OffTheGround);
        }
        for &(from, to) in groups.iter().rev().filter(|group| group.0 >= before_arms) {
            self.hoist("let down", Change::Nothing, -HOOK_SPEED, InReach(from));
            self.step("drawn in", Change::Shorten(from, to), StrutSlack(from, to));
            self.step(self.strut_label('−', from, to), Change::Take(from), After(SETTLING));
        }
        if before_arms > 0 {
            self.hoist("shoulders let down", Change::Nothing, -HOOK_SPEED, LegsSlack);
            self.step("lying", Change::Nothing, After(RESTING));
        }
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
        let (groups, before_arms, guyed) = (self.groups(), self.before_arms, !self.anchors.is_empty());
        if before_arms > 0 {
            self.step("let down, legs drawn apart", Change::LayDown, After(LAYING));
            self.step("net laid out", Change::LaidOut, After(EASING));
        } else {
            self.hoist("let down, legs drawn apart", Change::LinesOn, -HOOK_SPEED, InReach(0));
            self.step("lines off", Change::LinesOff, After(SETTLING));
        }
        for &(from, to) in groups.iter().filter(|group| group.0 < before_arms) {
            self.step("net brought into place", Change::Bring(to), After(BRINGING));
            self.step(self.strut_label('+', from, to), Change::Add(to), After(SETTLING));
        }
        if before_arms > 0 {
            self.step("shoulders complete", Change::Nothing, After(RESTING));
        }
        // A strut going in hanging goes in with both its ends off the ground.
        for &(from, to) in groups.iter().filter(|group| group.0 >= before_arms) {
            if before_arms > 0 {
                let eyes = self.eyes.iter().map(|&eye| self.poses[to - 1][eye].y).sum::<f32>() / self.eyes.len() as f32;
                self.hoist("raised", Change::Nothing, HOOK_SPEED, EyesAt(Meters(eyes)));
            }
            self.hoist("raised clear", Change::Nothing, HOOK_SPEED, StrutsClear(from, to));
            let brought = if before_arms > 0 { "net brought into place" } else { "caps brought into place" };
            self.step(brought, Change::Bring(to), After(BRINGING));
            self.step(self.strut_label('+', from, to), Change::Add(to), After(SETTLING));
        }
        self.step("every strut in", Change::Nothing, After(RESTING));
        self.hoist("stood up", Change::Nothing, HOOK_SPEED, EyesAt(self.standing));
        self.step("feet to their mounts", Change::FeetOn, After(RESTING));
        if guyed {
            self.step("hook brought back", Change::Upright, After(SWINGING));
            self.step("upright", Change::Nothing, After(RESTING));
            self.step("guy lines on", Change::GuysOn, After(SWINGING));
        }
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
        let mut bringing = None;
        let kept = match step.change {
            Change::Shorten(_, to) => Some(to - 1),
            _ => None,
        };
        match step.change {
            Change::Nothing => {}
            Change::HookOn => {
                let mut hook = Hook::taut(fabric, self.eyes.clone(), LEG);
                hook.place = Some(fabric.location(hook.joint));
                let anchors = self.anchors.iter().copied().chain(self.feet_held(fabric)).collect();
                fabric.site = Some(Site { ground: Ground::Sliding, hook: Some(hook), anchors });
            }
            Change::GuysOff => {
                let mut next = partial(&self.whole, &self.order);
                carry_over(fabric, &mut next, None);
                self.replace(fabric, next);
                fabric.site.as_mut().expect("a site").anchors = self.feet_held(fabric);
            }
            Change::Upright => {
                let hook = fabric.site.as_ref().and_then(|site| site.hook.as_ref()).expect("on the hook");
                sweep = Some((fabric.location(hook.joint), self.hook_upright(fabric)));
            }
            Change::FeetOff => {
                fabric.site.as_mut().expect("a site").anchors.clear();
                if let Some(hook) = fabric.site.as_ref().and_then(|site| site.hook.as_ref()) {
                    let (at, above) = (fabric.location(hook.joint), Hook::above(fabric, &hook.eyes));
                    sweep = Some((at, Vec3::new(above.x, at.y, above.z)));
                }
            }
            Change::Bring(count) => {
                let caps = self.caps_brought(fabric, count);
                // The hook goes along, to where its legs are taut over where the
                // eyes are going, or lower if it was: a sling leg stretched past
                // its length would snap the eye when the net is let go.
                if let Some(hook) = fabric.site.as_ref().and_then(|site| site.hook.as_ref()) {
                    let at = fabric.location(hook.joint);
                    let eyes: Vec<Vec3> =
                        hook.eyes.iter().map(|eye| caps.iter().find(|(cap, ..)| cap == eye).map_or(fabric.location(*eye), |&(.., to)| to)).collect();
                    let aside = aside(&eyes);
                    let taut = eyes.iter().sum::<Vec3>() / eyes.len() as f32 + Vec3::Y * (LEG.f32() * LEG.f32() - aside * aside).sqrt();
                    sweep = Some((at, Vec3::new(taut.x, taut.y.min(at.y), taut.z)));
                }
                bringing = Some(caps);
            }
            Change::FeetOn => {
                let feet = self.feet_held(fabric);
                fabric.site.as_mut().expect("a site").anchors = feet.clone();
                mounting = Some(feet);
            }
            Change::Take(count) => {
                for &strut in &self.order[count..] {
                    let length = self.whole.intervals[strut].ideal();
                    let strut = &mut fabric.intervals[strut];
                    strut.stiffness = Percent(0.0);
                    strut.span = Span::Fixed { length };
                }
                self.placed = count;
                self.drape_slack_cables(fabric);
            }
            Change::Shorten(from, to) => {
                self.poses[to - 1] = pose_of(fabric);
                for &strut in &self.order[from..to] {
                    let shorter = fabric.intervals[strut].ideal() * SHORTENED;
                    fabric.extend_interval(strut, shorter, SHORTENING);
                }
            }
            Change::LinesOn => {
                let stood = middle(&self.whole, &self.eyes);
                for foot in self.feet.clone() {
                    let at = self.whole.location(foot);
                    let outwards = Vec3::new(at.x - stood.x, 0.0, at.z - stood.z).normalize();
                    let stake = Vec3::new(at.x, 0.0, at.z) + outwards * NEAR_STAKE.f32();
                    self.line(fabric, foot, stake, STAKE_REACHED, FOOT_REELING, STAKE_TAUT, true);
                }
            }
            Change::LinesOff => self.lines_off(fabric),
            Change::LayDown => {
                let lain = &self.poses[self.stops[0] - 1];
                let hook = fabric.site.as_ref().and_then(|site| site.hook.as_ref()).expect("on the hook");
                let eyes: Vec<Vec3> = hook.eyes.iter().map(|&eye| lain[eye]).collect();
                let aside = aside(&eyes);
                let taut = eyes.iter().sum::<Vec3>() / eyes.len() as f32 + Vec3::Y * (LEG.f32() * LEG.f32() - aside * aside).sqrt();
                sweep = Some((fabric.location(hook.joint), taut));
                let middle = lain.values().sum::<Vec3>() / lain.len() as f32;
                let feet: Vec<(JointKey, Vec3)> = self.feet.iter().map(|&foot| (foot, lain[foot])).collect();
                let marks: Vec<(JointKey, Vec3)> = self.hands.iter().map(|&hand| (hand, lain[hand])).collect();
                for (foot, mark) in feet {
                    let outwards = Vec3::new(mark.x - middle.x, 0.0, mark.z - middle.z).normalize();
                    let stake = Vec3::new(mark.x, 0.0, mark.z) + outwards * NEAR_STAKE.f32();
                    self.line(fabric, foot, stake, Meters(NEAR_STAKE.f32() - FEET_PAST.f32()), FOOT_REELING, STAKE_TAUT, true);
                }
                for (hand, mark) in marks {
                    let outwards = Vec3::new(mark.x - middle.x, 0.0, mark.z - middle.z).normalize();
                    let stake = Vec3::new(mark.x, 0.0, mark.z) + outwards * HAND_LINE.f32();
                    self.line(fabric, hand, stake, HAND_LINE, HAND_REELING, JUST_TAUT, false);
                }
            }
            Change::LaidOut => {
                if self.before_arms > 0 {
                    self.lines_off(fabric);
                }
                spread = Some(pose_of(fabric));
            }
            Change::Add(count) => {
                let going_in: Vec<IntervalKey> =
                    self.order[..count].iter().copied().filter(|&strut| fabric.intervals[strut].is_out()).collect();
                for strut in going_in {
                    let spanned = Meters(fabric.intervals[strut].length(&fabric.joints).f32().min(self.whole.intervals[strut].ideal().f32()));
                    fabric.intervals[strut].stiffness = Percent(100.0);
                    fabric.intervals[strut].span = Span::Fixed { length: spanned };
                    fabric.extend_interval(strut, self.whole.intervals[strut].ideal(), FITTING);
                }
                self.placed = count;
                self.drape_slack_cables(fabric);
                // Everything was moved: each line as taut as it is kept.
                for line in &self.lines {
                    let spanned = fabric.intervals[line.key].length(&fabric.joints);
                    fabric.intervals[line.key].span = Span::Fixed { length: spanned / (1.0 + line.taut) };
                }
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
                fabric.site.as_mut().expect("a site").anchors = anchors.into_iter().chain(self.feet_held(fabric)).collect();
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
        self.running = Some(Running { label: step.label, until: step.until, ticks: 0, slack_ticks: 0, sweep, spread, mounting, bringing, kept });
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
        if let (Some(caps), Until::After(seconds)) = (&running.bringing, &running.until) {
            let along = (running.ticks + 1) as f32 / ticks(*seconds) as f32;
            for &(cap, from, to) in caps {
                fabric.joints[cap].location = from.lerp(to, along);
                fabric.joints[cap].velocity = Vec3::ZERO;
            }
        }
        if let (Some(feet), Until::After(seconds), Some(site)) = (&running.mounting, &running.until, fabric.site.as_mut()) {
            let along = (running.ticks + 1) as f32 / ticks(*seconds) as f32;
            site.anchors = feet.iter().map(|&(foot, from)| (foot, from.lerp(self.stood.location(foot), along))).collect();
        }
        for line in &self.lines {
            let interval = &mut fabric.intervals[line.key];
            let reeled = line.reeling / ticks(Seconds(1.0)) as f32;
            let length = if interval.strain < line.taut {
                (interval.ideal().f32() - reeled).max(line.to)
            } else if line.paid_out && interval.strain > 2.0 * line.taut {
                interval.ideal().f32() + reeled
            } else {
                continue;
            };
            interval.span = Span::Fixed { length: Meters(length) };
        }
        if let (Some(hung), Until::After(seconds)) = (&running.spread, &running.until) {
            let along = (running.ticks + 1) as f32 / ticks(*seconds) as f32;
            for (key, joint) in fabric.joints.iter_mut() {
                if let (Some(&from), Some(&to)) = (hung.get(key), self.poses[self.stops[0] - 1].get(key)) {
                    let lifted = from.distance(to).min(CARRIED.f32()) * (along * std::f32::consts::PI).sin();
                    joint.location = from.lerp(to, along) + Vec3::Y * lifted;
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
                let stake = |key: JointKey| self.lines.iter().any(|line| line.stake == key);
                fabric.joints.iter().all(|(key, joint)| joint.location.y > CLEAR.f32() || stake(key))
                    || running.ticks >= ticks(HOOK_TRAVEL)
            }
            Until::StrutsClear(from, to) => {
                let clear = self.order[from..to].iter().all(|&strut| lower_end(fabric, strut) > CLEAR.f32());
                clear || running.ticks >= ticks(HOOK_TRAVEL)
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
