//! The wobble table a Medium piece stands on: a hexagonal deck hung on three
//! short ties, at alternate corners, from arms rising off a fixed hexagonal
//! base. Hung so, the deck can't tilt; it sways on its ties like a pendulum
//! and twists about its middle. At the other three corners a stepper on
//! the base turns a drum, and a cable loop from the drum, through a pulley
//! either side, to a lug under the deck draws the deck along its edge, one
//! way or the other. All three drawing the same way twist the deck; drawing
//! so that they add up to nothing, they sway it. The piece's feet stand on
//! the deck and go with it, and pull on it in turn.
//!
//! The steppers play cues (`Cue`), DMX loops of the drums' positions,
//! evolved out of sight: each tried on a copy of the piece as it stood when Quake
//! was pressed, `ROUND` at a time side by side. A try watches every joint
//! of the piece swing against the deck, scoring the cue for the joint that
//! swings most swinging expressively and for how little the drums work,
//! and the best of each kind of swing, the whole piece's way through the
//! loop, is kept, unlike one another. Evolving goes on until Quake is
//! pressed again. The piece itself plays what is kept: each new find a
//! moment, else each kept in turn a moment; Demo plays them all in turn,
//! longer, evolution waiting meanwhile.

use crate::fabric::cue::{Cue, PER_VALUE, TOP_SPEED, ZERO};
use crate::fabric::dimensions::Hardware;
use crate::fabric::{Fabric, JointKey};
use crate::units::{Seconds, Unit, EARTH_GRAVITY};
use crate::Age;
use glam::{Quat, Vec3};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use crate::fabric::physics::Physics;
use slotmap::SecondaryMap;
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The deck's top, where the piece stands, above the floor.
pub const DECK_HEIGHT: f32 = 0.45;
pub const DECK_THICKNESS: f32 = 0.04;
pub const BASE_THICKNESS: f32 = 0.05;
/// Between the deck's underside and an arm's head.
pub const HEAD_CLEARANCE: f32 = 0.03;
/// Each tie, from the arm's head down to the hook's foot: the deck sways
/// once a second on ties this long.
pub const TIE_LENGTH: f32 = 0.25;
/// The ties hang this far out from the middle, as part of the deck's radius.
pub const TIE_RADIUS: f32 = 0.75;
/// How far to each side of its tie a hook drops and an arm rises.
pub const HOOK_REACH: f32 = 0.1;
/// The hooks and the arms stand at these corners of the hexagon: one behind
/// the piece and two before it, either side, so the table has the piece's
/// own mirror symmetry and the piece faces between two of them.
pub const HANG_CORNERS: [usize; 3] = [0, 2, 4];
/// The steppers stand at the other corners: one before the piece and two
/// behind it, either side.
pub const DRUM_CORNERS: [usize; 3] = [1, 3, 5];
/// From a lug to the pulley either side of it, along the deck's edge.
pub const LOOP_LEG: f32 = 0.15;
/// Deck and frame, per square metre.
const DECK_DENSITY: f32 = 15.0;
/// The deck is this much larger across than the smallest that holds every
/// foot and guy anchor: just big enough to look good.
pub const ROOM: f32 = 1.3;
/// A joint this low stands on the deck.
const FOOTING: f32 = 0.02;
/// The smallest table, across its corners: room for its hooks and arms.
pub const SMALLEST: f32 = 0.4;
/// How much of its swing the deck loses each cycle, as a damping ratio.
const DAMPING: f32 = 0.05;
/// A spring in each cable loop, in series with its cord, so the deck
/// follows the drums smoothly instead of with every jerk of a stepper: each
/// table's springs are chosen for its deck to sway on them, by itself, this
/// many times a second, well above the piece's own swings.
const DECK_SWAY: f32 = 7.0;
/// How far the joint swinging most should swing against the deck:
/// expressive, no more.
const EXPRESSIVE: f32 = 0.04;
/// A cue is tried this long, listened to after it has had time to build.
const TRYING: Seconds = Seconds(12.0);
const SETTLING_IN: Seconds = Seconds(4.0);
/// A cue kept is played this long, and one just found this long.
const PLAYING: Seconds = Seconds(15.0);
const SHOWING: Seconds = TRYING;
/// Stopping, the drums go back to their middles this fast, metres a second.
const RETURNING: f32 = 0.05;
/// With nothing kept yet, it looks again this often.
const WAITING: Seconds = Seconds(0.25);
/// Cues tried side by side, a round of them.
const ROUND: usize = 8;
/// A kept cue bettered by this much is a find.
const NOTABLE: f32 = 0.05;
/// It keeps at most this many cues.
const KEPT: usize = 6;
/// How much the drums' work counts against a cue, against its swing:
/// drums drawing `EFFORT_SPEED` on average cost this much.
const EFFORT: f32 = 0.3;
const EFFORT_SPEED: f32 = 0.1;
/// How much a quick sway counts against a cue: this much for each sway a
/// second, so that of two cues swinging as far the slower sway wins.
const HASTE: f32 = 0.15;
/// How a cue tried is scored: how far the joint swinging most should
/// swing, and how much the drums' work and a quick sway count against it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scoring {
    pub expressive: f32,
    pub effort: f32,
    pub haste: f32,
}

impl Scoring {
    /// What the Quake button scores by.
    pub const QUAKE: Scoring = Scoring { expressive: EXPRESSIVE, effort: EFFORT, haste: HASTE };
}

/// Two swings less alike than this (`Quake::alike`) are of different kinds.
pub const DISTINCT: f32 = 0.6;
/// A swing's shape is followed in this many steps through its cue's loop.
const BINS: usize = 12;
/// How long the swing is remembered, as it dies away, and how long
/// where they stand is averaged over.
const RESPONSE_MEMORY: Seconds = Seconds(1.0);
const STANCE_MEMORY: Seconds = Seconds(3.0);

/// Which way the hexagon's corner `index` lies, from the middle, measured
/// from +x toward +z, the way the piece faces: corners 0, 2 and 4 at 30°,
/// 150° and 270°.
pub fn corner_angle(index: usize) -> f32 {
    (index as f32 * 60.0 + 30.0).to_radians()
}

/// The radius across the corners of the smallest table that holds every
/// one of `places`, on it: each must be inside every edge.
pub fn smallest_hexagon(places: impl Iterator<Item = Vec3>) -> f32 {
    places.fold(0.0, |radius: f32, place| {
        let toward = place.z.atan2(place.x).rem_euclid(PI / 3.0);
        let off_normal = toward.min(PI / 3.0 - toward);
        radius.max(place.x.hypot(place.z) * off_normal.cos() / (PI / 6.0).cos())
    })
}

/// The y of a hook's foot, where its tie and its cable are tied.
pub fn hook_foot() -> f32 {
    -DECK_THICKNESS - HEAD_CLEARANCE - TIE_LENGTH
}

/// Which way drum `drum` draws its lug when its DMX value rises: along
/// the deck's edge, all three the same way round.
pub fn drum_along(drum: usize) -> Vec3 {
    let angle = corner_angle(DRUM_CORNERS[drum]);
    Vec3::new(-angle.sin(), 0.0, angle.cos())
}

/// Where each drum's lug is, at its foot, where its cable loop is tied, on
/// the deck at rest.
pub fn lugs(radius: f32) -> [Vec3; 3] {
    DRUM_CORNERS.map(|corner| {
        let angle = corner_angle(corner);
        Vec3::new(angle.cos(), 0.0, angle.sin()) * (TIE_RADIUS * radius - HOOK_REACH) + Vec3::Y * hook_foot()
    })
}

/// A cue kept: how well it did, how far the joint that swung most swung,
/// which joint that was and which way it went most (`Swing`), and the shape
/// of the whole piece's swing (`Quake::alike`).
#[derive(Clone, Debug)]
pub struct Kept {
    pub cue: Cue,
    pub score: f32,
    pub swing: f32,
    pub most: String,
    /// How quickly the piece sways: its joints' speed against the deck,
    /// over how far they move, as cycles a second.
    pub quickness: f32,
    shape: Vec<f32>,
}

/// Which way a joint's swing mostly goes, against the deck: side to side,
/// up and down, fore and aft (the way the piece faces), or two of them
/// when no one is most of it.
fn way(motion: &[Vec3]) -> String {
    let energy: [f32; 3] = [0, 1, 2].map(|axis| motion.iter().map(|v| v[axis] * v[axis]).sum());
    let total = energy.iter().sum::<f32>().max(1e-12);
    let names = ["side", "up", "fore"];
    let mut order = [0, 1, 2];
    order.sort_by(|&a, &b| energy[b].total_cmp(&energy[a]));
    if energy[order[0]] / total >= 0.7 {
        names[order[0]].to_string()
    } else {
        format!("{}+{}", names[order[0]], names[order[1]])
    }
}

/// What the steppers do: try a cue, on a copy of the piece, play one kept
/// for so long, or wait in their middles for one to be found.
#[derive(Clone, Debug, PartialEq)]
enum Task {
    Try(Cue),
    Play(usize, Cue, Seconds),
    Wait,
}

/// The evolution: the seeds still to try, the cues kept, unlike one another,
/// each named by where it is kept, and how many have been tried.
#[derive(Clone, Debug)]
struct Evolution {
    scoring: Scoring,
    seeds: VecDeque<Cue>,
    kept: Vec<Kept>,
    /// How many times what is kept has changed.
    changes: u32,
    tries: u32,
    random: ChaCha8Rng,
}

impl Evolution {
    /// Evolving from `remembered` cues, tried first, and then the seeds.
    fn new(remembered: Vec<Cue>, scoring: Scoring) -> Evolution {
        let seeds = remembered.into_iter().chain(Cue::seeds()).collect();
        Evolution { scoring, seeds, kept: Vec::new(), changes: 0, tries: 0, random: ChaCha8Rng::seed_from_u64(1) }
    }

    /// The next cue to try: a seed, or a child of those kept.
    fn next(&mut self) -> Cue {
        self.seeds.pop_front().unwrap_or_else(|| self.child())
    }

    /// A child of the cues kept: of the better of a pair drawn, sometimes
    /// crossed with another; with none kept, of a seed.
    fn child(&mut self) -> Cue {
        if self.kept.is_empty() {
            let seeds = Cue::seeds();
            let seed = &seeds[self.random.gen_range(0..seeds.len())];
            return seed.mutated(None, &mut self.random);
        }
        let draw = |random: &mut ChaCha8Rng| {
            let (a, b) = (random.gen_range(0..self.kept.len()), random.gen_range(0..self.kept.len()));
            if self.kept[a].score >= self.kept[b].score { a } else { b }
        };
        let parent = draw(&mut self.random);
        let other = draw(&mut self.random);
        let other = (other != parent && self.random.gen_bool(0.3)).then(|| self.kept[other].cue.clone());
        self.kept[parent].cue.mutated(other.as_ref(), &mut self.random)
    }

    /// A cue tried: kept if the piece swung, not wildly, and no kept cue's
    /// swing is like it, or in place of the one whose is (only one) but did
    /// less well, or of the worst when there are enough; and where, if it
    /// is a find: a new kind, or `NOTABLE` better.
    fn keep(&mut self, found: Kept) -> Option<usize> {
        self.tries += 1;
        if found.swing <= self.scoring.expressive / 2.0 || found.score <= 0.0 {
            return None;
        }
        let alike: Vec<usize> =
            (0..self.kept.len()).filter(|&index| Quake::alike(&self.kept[index].shape, &found.shape) >= DISTINCT).collect();
        match alike[..] {
            [index] if found.score > self.kept[index].score => {
                let better = found.score - self.kept[index].score;
                self.kept[index] = found;
                self.changes += 1;
                (better >= NOTABLE).then_some(index)
            }
            [_, ..] => None,
            [] if self.kept.len() < KEPT => {
                self.kept.push(found);
                self.changes += 1;
                Some(self.kept.len() - 1)
            }
            [] => {
                let worst = (0..self.kept.len()).min_by(|&a, &b| self.kept[a].score.total_cmp(&self.kept[b].score))?;
                (found.score > self.kept[worst].score).then(|| {
                    self.kept[worst] = found;
                    self.changes += 1;
                    worst
                })
            }
        }
    }
}

/// The evolution, shared with the rounds run out of sight: where each find
/// was kept, in turn, whether it is still evolving, whether it waits for a
/// demo or should stop, and where what it keeps is remembered.
#[derive(Debug)]
struct Shared {
    evolution: Evolution,
    finds: Vec<usize>,
    evolving: bool,
    paused: bool,
    stop: bool,
    /// A round is being tried.
    trying: bool,
    memory: Option<PathBuf>,
    /// Rounds run, and how many since the last find.
    rounds: u32,
    stale: u32,
}

/// Where the cues kept for the piece `name` are remembered between runs.
pub fn cues_file(name: &str) -> PathBuf {
    PathBuf::from("cues").join(format!("{}.txt", name.to_lowercase().replace(' ', "-")))
}

/// The cues remembered in `path`: each line a kept cue, what the steppers
/// are told after `|`.
pub fn remembered(path: &Path) -> Vec<Cue> {
    std::fs::read_to_string(path).map(|text| kept_cues(&text)).unwrap_or_default()
}

/// The cues kept in `text`, as written in a cues file.
pub fn kept_cues(text: &str) -> Vec<Cue> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| Cue::parse(line.rsplit_once('|')?.1))
        .collect()
}

/// Remember `kept` in `path`, A first: its name, score, how far the piece
/// swung, what it is (`Cue::describe`), and after `|` its DMX frames.
fn remember(path: &Path, kept: &[Kept]) {
    let mut text = String::from("# Wobble table cues, kept by Quake and tried first next time.\n");
    text += "# name score swing, the joint moved most and which way | the deck's ways, mm | DMX frames, 44 a second,\n";
    text += "# each the three drums' values, 128 their middles: the one before the piece, then the others round the way they draw\n";
    for (index, kept) in kept.iter().enumerate() {
        text += &format!(
            "{} {:.2} {:.0}mm {} | {} | {}\n",
            letter(index),
            kept.score,
            kept.swing * 1000.0,
            kept.most,
            kept.cue.describe(),
            kept.cue.written()
        );
    }
    let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(path, text));
    if let Err(error) = written {
        eprintln!("Couldn't remember the cues in {}: {error}", path.display());
    }
}

/// The piece's link to its evolution. A copy of the piece has none, and when
/// the piece goes, its evolution stops.
#[derive(Debug, Default)]
struct Search(Option<Arc<Mutex<Shared>>>);

impl Clone for Search {
    fn clone(&self) -> Self {
        Search(None)
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        if let Some(shared) = &self.0 {
            shared.lock().unwrap().stop = true;
        }
    }
}

/// Each cue tried on its own copy of `piece`, side by side.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn trials(piece: &Fabric, physics: &Physics, cues: Vec<Cue>) -> Vec<Option<Kept>> {
    std::thread::scope(|scope| {
        let tries: Vec<_> = cues.into_iter().map(|cue| scope.spawn(move || trial(piece, physics, cue))).collect();
        tries.into_iter().map(|try_| try_.join().unwrap()).collect()
    })
}

/// `cue` tried on a copy of `piece`, for as long as a try lasts.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn trial(piece: &Fabric, physics: &Physics, cue: Cue) -> Option<Kept> {
    let mut fabric = piece.clone();
    let quake = fabric.quake.as_mut()?;
    quake.next = Some(Task::Try(cue));
    quake.begin(fabric.age);
    loop {
        fabric.iterate(physics);
        if let Some(found) = fabric.quake.as_mut()?.found.take() {
            return Some(found);
        }
    }
}

/// Evolve in rounds, each cue tried on its own copy of `piece`, until told
/// to stop, waiting while a demo plays.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn evolve(shared: Arc<Mutex<Shared>>, piece: Fabric, physics: Physics) {
    let mut stale = 0;
    loop {
        let cues: Vec<Cue> = {
            let mut shared = shared.lock().unwrap();
            if shared.stop {
                break;
            }
            if shared.paused {
                drop(shared);
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
            shared.trying = true;
            (0..ROUND).map(|_| shared.evolution.next()).collect()
        };
        let found = trials(&piece, &physics, cues);
        let mut shared = shared.lock().unwrap();
        shared.trying = false;
        let (before, changes) = (shared.finds.len(), shared.evolution.changes);
        for found in found.into_iter().flatten() {
            if let Some(index) = shared.evolution.keep(found) {
                shared.finds.push(index);
            }
        }
        stale = if shared.finds.len() > before { 0 } else { stale + 1 };
        (shared.rounds, shared.stale) = (shared.rounds + 1, stale);
        if let (Some(memory), true) = (&shared.memory, shared.evolution.changes > changes) {
            remember(memory, &shared.evolution.kept);
        }
    }
    shared.lock().unwrap().evolving = false;
}

/// A kept cue's name: A to F, by where it is kept.
fn letter(index: usize) -> char {
    (b'A' + index as u8) as char
}

/// The piece and its table at rest, as they stood when Quake was pressed,
/// for each cue played to start from: every joint where it was and how it
/// moved, and the deck.
#[derive(Clone, Debug)]
struct Rest {
    joints: Vec<(JointKey, Vec3, Vec3)>,
    deck: Deck,
    velocity: Vec3,
    spin: f32,
    standing: SecondaryMap<JointKey, (Vec3, Vec3)>,
    watched: Vec<Watched>,
}

/// A joint the motor watches, its name, and where it has stood on the
/// deck lately.
#[derive(Clone, Debug)]
struct Watched {
    key: JointKey,
    name: String,
    stance: Vec3,
}

/// What it hears while a cue plays, once it has settled in: the swing of
/// the joint swinging most, the drums' work, and every joint's way from
/// where it stands through the loop, `BINS` steps of it for each.
#[derive(Clone, Debug, Default)]
struct Hearing {
    ticks: u32,
    swing: f32,
    work: f32,
    /// Every joint's distance from where it stands, and its speed, against
    /// the deck, squared and summed.
    moved: f32,
    speed: f32,
    shape: Vec<Vec3>,
    counts: Vec<u32>,
}

#[derive(Clone, Debug)]
struct Motor {
    since: Age,
    /// Bringing its drums back to their middles, to stop.
    stopping: bool,
    /// How far each drum has drawn its cable from its middle, and how fast
    /// it drew last tick, metres and metres a second.
    drums: [f32; 3],
    speeds: [f32; 3],
    hearing: Hearing,
}

/// Where the deck is: moved this far from where it hangs still and turned
/// this far about its middle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Deck {
    pub offset: Vec3,
    pub turn: f32,
    pub radius: f32,
}

impl Deck {
    /// Where a point resting on the deck at rest is now.
    pub fn carry(&self, point: Vec3) -> Vec3 {
        Quat::from_rotation_y(self.turn) * point + self.offset
    }

    /// Where a point is on the deck, as if it were at rest.
    fn on_deck(&self, point: Vec3) -> Vec3 {
        Quat::from_rotation_y(-self.turn) * (point - self.offset)
    }
}

#[derive(Clone, Debug)]
pub struct Quake {
    deck: Deck,
    velocity: Vec3,
    spin: f32,
    /// The deck's own mass, kg.
    mass: f32,
    /// What hangs on the ties: the deck and the piece, newtons.
    weight: f32,
    /// Each cable loop's stiffness along the deck's edge, newtons a metre.
    loop_stiffness: f32,
    motor: Option<Motor>,
    /// What the piece's feet pulled on the deck with this tick, and the
    /// twist it gave.
    pull: Vec3,
    pull_twist: f32,
    /// The joints it watches.
    watched: Vec<Watched>,
    /// How far they have swung against the deck lately, metres.
    response: f32,
    /// Each joint standing on the deck: where on the deck, and where it was
    /// put.
    standing: SecondaryMap<JointKey, (Vec3, Vec3)>,
    /// What the steppers play, and what comes once they are back in their
    /// middles.
    task: Option<Task>,
    next: Option<Task>,
    /// The evolution, how many of its finds have been played, and the kept
    /// cue a demo plays.
    search: Search,
    played: usize,
    demo: Option<usize>,
    /// A cue tried on a copy of the piece, once its try is done.
    found: Option<Kept>,
    /// The piece at rest, and whether to put it back so this tick.
    rest: Option<Rest>,
    restoring: bool,
    scoring: Scoring,
}

impl Quake {
    /// A table `radius` across its corners, still, its drums in their
    /// middles, nothing on it yet. A loop draws with both its legs, each
    /// from the lug to a pulley; three along the edge hold the deck from
    /// swaying half again as stiffly as one.
    pub fn of_radius(radius: f32) -> Quake {
        let area = 1.5 * 3.0f32.sqrt() * radius * radius;
        let mass = DECK_DENSITY * area;
        let cord = 2.0 * Hardware::MEDIUM.cable_stiffness.0 / LOOP_LEG;
        let ties = mass * EARTH_GRAVITY.f32() / TIE_LENGTH;
        let loop_stiffness = ((mass * (TAU * DECK_SWAY).powi(2) - ties) / 1.5).clamp(ties, cord);
        Quake {
            deck: Deck { radius, ..Deck::default() },
            velocity: Vec3::ZERO,
            spin: 0.0,
            mass,
            weight: mass * EARTH_GRAVITY.f32(),
            loop_stiffness,
            motor: None,
            pull: Vec3::ZERO,
            pull_twist: 0.0,
            watched: Vec::new(),
            response: 0.0,
            standing: SecondaryMap::new(),
            task: None,
            next: None,
            search: Search::default(),
            played: 0,
            demo: None,
            found: None,
            rest: None,
            restoring: false,
            scoring: Scoring::QUAKE,
        }
    }

    /// The deck made `ROOM` larger than it need be for where the feet and
    /// guy anchors of the piece `fabric` stand on it, and no smaller than
    /// `SMALLEST`, unless its plan chose the table; and its weight and the
    /// piece's on the ties.
    pub fn fit(&mut self, fabric: &Fabric) {
        let feet = fabric.joints.values().filter(|joint| joint.location.y < FOOTING).map(|joint| self.deck.on_deck(joint.location));
        let fitting = match fabric.dimensions.table_scales {
            Some(_) => self.deck.radius,
            None => (ROOM * smallest_hexagon(feet)).max(SMALLEST),
        };
        let fitted = Quake::of_radius(fitting);
        (self.deck.radius, self.mass, self.loop_stiffness) = (fitting, fitted.mass, fitted.loop_stiffness);
        let piece = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum::<f32>() / 1000.0;
        self.weight = (self.mass + piece) * EARTH_GRAVITY.f32();
    }

    /// The piece `fabric` landed on the table: its weight on the ties, and
    /// every joint watched.
    pub fn mount(&mut self, fabric: &Fabric) {
        self.watched = fabric
            .joints
            .iter()
            .map(|(key, joint)| Watched {
                key,
                name: joint.label.map_or_else(|| "?".to_string(), |label| label.to_string()),
                stance: joint.location,
            })
            .collect();
    }

    /// Whether a piece has landed on it.
    pub fn mounted(&self) -> bool {
        !self.watched.is_empty()
    }

    pub fn deck(&self) -> Deck {
        self.deck
    }

    /// Score the cues it tries by `scoring` rather than as the Quake button
    /// does, before it starts evolving.
    pub fn score_by(&mut self, scoring: Scoring) {
        self.scoring = scoring;
    }

    /// How far the watched joints have swung against the deck lately.
    pub fn response(&self) -> f32 {
        self.response
    }

    /// What the evolution has come to: the cues kept, where each find was
    /// kept in turn, how many tried, and whether it is still evolving.
    fn shared<T>(&self, look: impl FnOnce(&Shared) -> T) -> Option<T> {
        self.search.0.as_ref().map(|shared| look(&shared.lock().unwrap()))
    }

    /// The cues kept so far, A first.
    pub fn kept(&self) -> Vec<Kept> {
        self.shared(|shared| shared.evolution.kept.clone()).unwrap_or_default()
    }

    /// How many cues it has tried.
    pub fn tries(&self) -> u32 {
        self.shared(|shared| shared.evolution.tries).unwrap_or(0)
    }

    /// Whether it is still evolving.
    pub fn evolving(&self) -> bool {
        self.shared(|shared| shared.evolving).unwrap_or(false)
    }

    /// The kept cue it plays, if it plays one.
    pub fn playing(&self) -> Option<usize> {
        match (&self.task, self.running()) {
            (Some(Task::Play(index, ..)), true) => Some(*index),
            _ => None,
        }
    }

    /// What the steppers are doing, for the subtitle, briefly: the kept cue
    /// they play, by its letter, and what they are told
    /// (`Cue::describe`), and the joint it moves most.
    pub fn describe(&self) -> String {
        let (Some(motor), Some(task)) = (&self.motor, &self.task) else {
            return "stopped".to_string();
        };
        let name = |task: &Task| match task {
            Task::Play(index, ..) => letter(*index).to_string(),
            _ => "?".to_string(),
        };
        if motor.stopping {
            return self.next.as_ref().map_or("stopping".to_string(), |next| format!("→ {}", name(next)));
        }
        match task {
            Task::Play(index, cue, _) => {
                let most = self.shared(|shared| shared.evolution.kept.get(*index).map(|kept| kept.most.clone())).flatten();
                format!("{} {} · {}", name(task), cue.describe(), most.unwrap_or_default())
            }
            Task::Try(cue) => cue.describe(),
            Task::Wait => "evolving".to_string(),
        }
    }

    /// How the evolution goes, for the side of the screen: how far it has
    /// got, the cues kept with how each did, and what plays now.
    pub fn report(&self) -> String {
        let Some((kept, tries, rounds, stale, evolving)) = self.shared(|shared| {
            (shared.evolution.kept.clone(), shared.evolution.tries, shared.rounds, shared.stale, shared.evolving)
        }) else {
            return String::new();
        };
        let mut lines = Vec::new();
        if self.demo.is_some() {
            lines.push(format!("Demo: the kept in turn, {:.0} s each,", PLAYING.0));
            lines.push(format!("then evolving goes on ({tries} tried)"));
        } else if evolving {
            lines.push(format!("Evolving, round {}: {tries} cues tried", rounds + 1));
            lines.push(format!("{ROUND} at a time on copies of the piece,"));
            lines.push(format!("{:.0} s each, out of sight", TRYING.0));
            lines.push(format!("Last find {stale} rounds ago"));
        } else {
            lines.push(format!("{tries} cues tried in {rounds} rounds"));
        }
        lines.push(String::new());
        lines.push("Kept, each a different swing:".to_string());
        if kept.is_empty() {
            lines.push("none yet".to_string());
        }
        let playing = self.playing();
        for (index, kept) in kept.iter().enumerate() {
            let marker = if playing == Some(index) { "→ " } else { "" };
            lines.push(format!("{marker}{} {:.2}  {:.0} mm  {}", letter(index), kept.score, kept.swing * 1000.0, kept.most));
        }
        lines.push(String::new());
        lines.push(match &self.task {
            Some(Task::Play(index, _, seconds)) => format!("Playing {} for {:.0} s, from rest", letter(*index), seconds.0),
            _ => "Waiting for a first find".to_string(),
        });
        lines.push(format!("Score: 1 for {:.0} mm of swing,", self.scoring.expressive * 1000.0));
        lines.push("less for the drums' work and quick sway".to_string());
        lines.join("\n")
    }

    /// The Quake button: it evolves cues on copies of `piece`, as it stands
    /// now, playing what it keeps, until pressed again. With a `memory`,
    /// the cues kept there before are tried first, and what it keeps is
    /// kept there.
    pub fn advance(&mut self, age: Age, piece: Option<(Fabric, Physics)>, memory: Option<PathBuf>) {
        if self.running() {
            self.then(age, None);
            self.demo = None;
            if let Some(shared) = &self.search.0 {
                let mut shared = shared.lock().unwrap();
                (shared.stop, shared.paused) = (true, false);
            }
            return;
        }
        if let Some((piece, Some(quake))) = piece.as_ref().map(|(piece, _)| (piece, piece.quake.as_ref())) {
            self.rest = Some(Rest {
                joints: piece.joints.iter().map(|(key, joint)| (key, joint.location, joint.velocity)).collect(),
                deck: quake.deck,
                velocity: quake.velocity,
                spin: quake.spin,
                standing: quake.standing.clone(),
                watched: quake.watched.clone(),
            });
        }
        let shared = self.search.0.get_or_insert_with(|| {
            let remembered = memory.as_deref().map(remembered).unwrap_or_default();
            Arc::new(Mutex::new(Shared {
                evolution: Evolution::new(remembered, self.scoring),
                finds: Vec::new(),
                evolving: false,
                paused: false,
                stop: false,
                trying: false,
                memory,
                rounds: 0,
                stale: 0,
            }))
        });
        let start = {
            let mut shared = shared.lock().unwrap();
            let start = !shared.evolving;
            (shared.evolving, shared.stop) = (true, false);
            start
        };
        if let (true, Some((piece, physics))) = (start, piece) {
            let shared = Arc::clone(shared);
            // The web has no threads to evolve on.
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || evolve(shared, piece, physics));
            #[cfg(target_arch = "wasm32")]
            drop((shared, piece, physics));
        }
        self.go_on(age);
    }

    /// Play `cue` round and round, from now on, as the gallery does.
    pub fn play(&mut self, age: Age, cue: Cue) {
        self.then(age, Some(Task::Play(0, cue, Seconds(f32::INFINITY))));
    }

    /// Bring the drums back to their middles and stop.
    pub fn stop(&mut self, age: Age) {
        self.then(age, None);
    }

    /// The Demo button: evolving waits while the cues kept play in turn,
    /// `PLAYING` each, then goes on.
    pub fn demo(&mut self, age: Age) {
        let kept = self.kept();
        let (true, Some(first)) = (self.running(), kept.first()) else {
            return;
        };
        if let Some(shared) = &self.search.0 {
            shared.lock().unwrap().paused = true;
        }
        self.demo = Some(0);
        self.then(age, Some(Task::Play(0, first.cue.clone(), PLAYING)));
    }

    /// Whether evolving waits for a demo, no round being tried.
    pub fn waiting(&self) -> bool {
        self.shared(|shared| shared.paused && !shared.trying).unwrap_or(false)
    }

    /// Whether a demo plays.
    pub fn demoing(&self) -> bool {
        self.demo.is_some()
    }

    /// On to what is next: in a demo the next kept cue, or at its end back
    /// to evolving; while evolving, the newest find, or else the next kept
    /// cue, a moment each, or with none yet, waiting for one.
    fn go_on(&mut self, age: Age) {
        let kept = self.kept();
        if let Some(index) = self.demo {
            if index + 1 < kept.len() {
                self.demo = Some(index + 1);
                self.then(age, Some(Task::Play(index + 1, kept[index + 1].cue.clone(), PLAYING)));
                return;
            }
            self.demo = None;
            if let Some(shared) = &self.search.0 {
                shared.lock().unwrap().paused = false;
            }
        }
        let finds = self.shared(|shared| shared.finds.clone()).unwrap_or_default();
        let next = if finds.len() > self.played {
            self.played = finds.len();
            let index = finds[finds.len() - 1];
            Task::Play(index, kept[index].cue.clone(), SHOWING)
        } else if !kept.is_empty() {
            let index = match self.task {
                Some(Task::Play(index, ..)) => (index + 1) % kept.len(),
                _ => 0,
            };
            Task::Play(index, kept[index].cue.clone(), SHOWING)
        } else {
            Task::Wait
        };
        self.then(age, Some(next));
    }

    /// On to `next` from rest, the piece put back as it stood when Quake
    /// was pressed; or with nothing next, bring the drums back to their
    /// middles.
    fn then(&mut self, age: Age, next: Option<Task>) {
        self.next = next;
        match &mut self.motor {
            Some(_) if self.next.is_some() && self.rest.is_some() => {
                self.restoring = true;
                self.motor = None;
                self.begin(age);
            }
            Some(motor) => motor.stopping = true,
            None => self.begin(age),
        }
    }

    fn begin(&mut self, age: Age) {
        self.task = self.next.take();
        if self.task.is_some() {
            let shape = vec![Vec3::ZERO; self.watched.len() * BINS];
            let counts = vec![0; self.watched.len() * BINS];
            let hearing = Hearing { shape, counts, ..Hearing::default() };
            self.motor = Some(Motor { since: age, stopping: false, drums: [0.0; 3], speeds: [0.0; 3], hearing });
        }
    }

    /// The cue it plays now.
    fn cue(&self) -> Option<&Cue> {
        match self.task.as_ref()? {
            Task::Try(cue) | Task::Play(_, cue, _) => Some(cue),
            Task::Wait => None,
        }
    }

    /// Done with the cue in hand: a cue tried is scored and the try ends,
    /// or it goes on to what is next.
    fn done(&mut self, age: Age) {
        let (Some(Task::Try(cue)), Some(motor)) = (self.task.clone(), &self.motor) else {
            self.go_on(age);
            return;
        };
        let hearing = &motor.hearing;
        let ticks = hearing.ticks.max(1) as f32;
        let swing = hearing.swing / ticks;
        let work = hearing.work / ticks / EFFORT_SPEED;
        let Scoring { expressive: target, effort, haste } = self.scoring;
        let expressive = swing / target;
        let expressive = if expressive <= 1.0 { expressive } else { (2.0 - expressive).max(0.0) };
        let quickness = (hearing.speed / hearing.moved.max(1e-12)).sqrt() / TAU;
        let score = expressive - effort * work - haste * quickness;
        let paths: Vec<Vec3> = hearing.shape.iter().zip(&hearing.counts).map(|(sum, &count)| *sum / count.max(1) as f32).collect();
        let reach = |joint: usize| paths[joint * BINS..(joint + 1) * BINS].iter().map(|v| v.length_squared()).sum::<f32>();
        let most = (0..self.watched.len()).max_by(|&a, &b| reach(a).total_cmp(&reach(b)));
        let most = most.map_or(String::new(), |joint| {
            format!("{} {}", self.watched[joint].name, way(&paths[joint * BINS..(joint + 1) * BINS]))
        });
        let shape = paths.iter().flat_map(|v| v.to_array()).collect();
        self.found = Some(Kept { cue, score, swing, most, quickness, shape });
        (self.task, self.motor) = (None, None);
    }

    /// How alike two swings are, whatever their size and wherever in the
    /// loop they start: one for the same, zero for nothing alike.
    pub fn alike(a: &[f32], b: &[f32]) -> f32 {
        let joints = a.len() / (3 * BINS);
        let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
        let (na, nb) = (norm(a), norm(b));
        (0..BINS)
            .map(|shift| {
                let mut dot = 0.0;
                for joint in 0..joints {
                    for bin in 0..BINS {
                        let (i, j) = (joint * BINS + bin, joint * BINS + (bin + shift) % BINS);
                        for axis in 0..3 {
                            dot += a[i * 3 + axis] * b[j * 3 + axis];
                        }
                    }
                }
                dot.abs() / (na * nb)
            })
            .fold(0.0, f32::max)
    }

    /// Whether the steppers are playing, not stopping.
    pub fn running(&self) -> bool {
        self.motor.as_ref().is_some_and(|motor| !motor.stopping)
    }

    /// Whether the steppers are playing, or bringing their drums back.
    pub fn busy(&self) -> bool {
        self.motor.is_some()
    }

    /// Whether the deck is still going, with the motor or after it.
    pub fn moving(&self) -> bool {
        self.motor.is_some() || self.velocity.length() > 1e-3 || self.spin.abs() > 1e-3
    }

    /// The steppers this tick: each drum after its cue's position, or back
    /// to its middle, as fast as it can go; and done with the cue once it
    /// has had its time.
    fn turn_drums(&mut self, age: Age, dt: f32) {
        let Some(motor) = &self.motor else {
            return;
        };
        let elapsed = age.elapsed_since(motor.since).0;
        let (targets, fastest) = match (motor.stopping, self.cue()) {
            (false, Some(cue)) => (cue.value(elapsed).map(|value| (value - ZERO as f32) * PER_VALUE), TOP_SPEED),
            _ => ([0.0; 3], RETURNING),
        };
        let motor = self.motor.as_mut().unwrap();
        for ((drum, speed), target) in motor.drums.iter_mut().zip(&mut motor.speeds).zip(targets) {
            let drawn = (target - *drum).clamp(-fastest * dt, fastest * dt);
            *drum += drawn;
            *speed = drawn / dt;
        }
        if motor.stopping {
            if motor.drums.iter().all(|drum| drum.abs() < 1e-6) {
                self.motor = None;
                self.begin(age);
            }
            return;
        }
        let lasts = match self.task {
            Some(Task::Try(..)) => TRYING.0,
            Some(Task::Play(_, _, seconds)) => seconds.0,
            _ => WAITING.0,
        };
        if elapsed >= lasts {
            self.done(age);
        }
    }

    /// One tick of the deck: the steppers, the ties pulling it back, the
    /// cable loops, damping, and what the piece's feet pulled with.
    fn step(&mut self, age: Age, dt: f32) {
        self.turn_drums(age, dt);
        let stiffness = self.weight / TIE_LENGTH;
        let tie_radius = TIE_RADIUS * self.deck.radius;
        let twist_stiffness = stiffness * tie_radius * tie_radius;
        let inertia = self.mass * 5.0 / 12.0 * self.deck.radius * self.deck.radius;
        let held = stiffness + 1.5 * self.loop_stiffness;
        let sway_damping = 2.0 * DAMPING * (held * self.mass).sqrt();
        let twist_damping = 2.0 * DAMPING * (twist_stiffness * inertia).sqrt();

        let mut force = self.pull - self.deck.offset * stiffness - self.velocity * sway_damping;
        let mut torque = self.pull_twist - self.deck.turn * twist_stiffness - self.spin * twist_damping;
        let drums = self.motor.as_ref().map_or([0.0; 3], |motor| motor.drums);
        for (drum, lug) in lugs(self.deck.radius).into_iter().enumerate() {
            let along = drum_along(drum);
            let at = self.deck.carry(lug);
            let pull = along * self.loop_stiffness * (drums[drum] - (at - lug).dot(along));
            let arm = at - self.deck.offset;
            force += pull;
            torque += arm.z * pull.x - arm.x * pull.z;
        }
        self.velocity += force / self.mass * dt;
        self.velocity.y = 0.0;
        self.spin += torque / inertia * dt;
        self.deck.offset += self.velocity * dt;
        self.deck.turn += self.spin * dt;
    }

    /// How far the watched joints swing about where they have stood on the
    /// deck lately, the farthest, remembered as it dies away; and while a
    /// cue plays, once settled in, what it hears.
    fn watch(&mut self, fabric_joints: impl Fn(JointKey) -> Option<(Vec3, Vec3)>, age: Age, dt: f32) {
        let follow = dt / STANCE_MEMORY.0;
        let mut swing = 0.0f32;
        let (mut moved, mut speed) = (0.0, 0.0);
        let mut aways = Vec::with_capacity(self.watched.len());
        for watched in &mut self.watched {
            let (away, relative) = fabric_joints(watched.key).map_or((Vec3::ZERO, Vec3::ZERO), |(at, velocity)| {
                let arm = at - self.deck.offset;
                let carried = self.velocity + Vec3::new(self.spin * arm.z, 0.0, -self.spin * arm.x);
                (self.deck.on_deck(at) - watched.stance, velocity - carried)
            });
            swing = swing.max(away.length());
            (moved, speed) = (moved + away.length_squared(), speed + relative.length_squared());
            watched.stance += away * follow;
            aways.push(away);
        }
        self.response = swing.max(self.response * (-dt / RESPONSE_MEMORY.0).exp());
        let loop_seconds = self.cue().map(Cue::seconds);
        let response = self.response;
        let Some(motor) = &mut self.motor else {
            return;
        };
        let elapsed = age.elapsed_since(motor.since).0;
        let (Some(loop_seconds), false, true) = (loop_seconds, motor.stopping, elapsed >= SETTLING_IN.0) else {
            return;
        };
        let hearing = &mut motor.hearing;
        hearing.ticks += 1;
        hearing.swing += response;
        hearing.work += motor.speeds.iter().map(|speed| speed.abs()).sum::<f32>() / 3.0;
        (hearing.moved, hearing.speed) = (hearing.moved + moved, hearing.speed + speed);
        let bin = ((elapsed / loop_seconds).fract() * BINS as f32) as usize % BINS;
        for (joint, away) in aways.into_iter().enumerate() {
            hearing.shape[joint * BINS + bin] += away;
            hearing.counts[joint * BINS + bin] += 1;
        }
    }
}

impl Fabric {
    /// The deck under a Medium piece, where it is now.
    pub fn deck(&self) -> Option<Deck> {
        self.quake.as_ref().map(Quake::deck)
    }

    /// Move every joint standing on the deck, within `contact` of it, as
    /// the deck moves in this tick. Each is put where it stands on the deck,
    /// remembered while nothing else moves it, so no rounding builds up.
    pub(crate) fn apply_quake(&mut self, contact: f32) {
        let Some(quake) = &mut self.quake else {
            return;
        };
        if std::mem::take(&mut quake.restoring) {
            if let Some(rest) = &quake.rest {
                for &(key, location, velocity) in &rest.joints {
                    if let Some(joint) = self.joints.get_mut(key) {
                        (joint.location, joint.velocity) = (location, velocity);
                    }
                }
                (quake.deck, quake.velocity, quake.spin) = (rest.deck, rest.velocity, rest.spin);
                (quake.standing, quake.watched) = (rest.standing.clone(), rest.watched.clone());
                quake.response = 0.0;
            }
        }
        let before = quake.deck;
        quake.step(self.age, Age::iteration_duration().f32());
        let after = quake.deck;
        for (key, joint) in self.joints.iter_mut() {
            if joint.location.y > contact {
                quake.standing.remove(key);
                continue;
            }
            let place = match quake.standing.get(key) {
                Some(&(place, left)) if left.x == joint.location.x && left.z == joint.location.z => place,
                _ => before.on_deck(joint.location),
            };
            let moved = after.carry(place);
            (joint.location.x, joint.location.z) = (moved.x, moved.z);
            quake.standing.insert(key, (place, joint.location));
        }
    }

    /// What the joints standing on the deck pull on it with, as the
    /// intervals pull on them, for the deck's next tick.
    pub(crate) fn feel_quake(&mut self, contact: f32) {
        let Some(quake) = &mut self.quake else {
            return;
        };
        let middle = quake.deck.offset;
        let (mut pull, mut twist) = (Vec3::ZERO, 0.0);
        for joint in self.joints.values().filter(|joint| joint.location.y <= contact) {
            let force = Vec3::new(joint.force.x, 0.0, joint.force.z) / 1000.0;
            let arm = joint.location - middle;
            pull += force;
            twist += arm.z * force.x - arm.x * force.z;
        }
        (quake.pull, quake.pull_twist) = (pull, twist);
        let joints = &self.joints;
        quake.watch(|key| joints.get(key).map(|joint| (joint.location, joint.velocity)), self.age, Age::iteration_duration().f32());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::dsl::fabric_library::{self, FabricName};
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::fabric::dimensions::Size;
    use crate::fabric::physics::presets::VIEWING;
    use crate::fabric::physics::Physics;
    use crate::fabric::cue::{FORE, SIDE};

    fn medium(name: FabricName) -> (Fabric, Physics) {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(name).at_size(Size::Medium));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..VIEWING };
        (executor.fabric, physics)
    }

    /// How big each Medium piece's table is, and how far its feet and guy
    /// anchors reach as it stands, in metres and in its `scale`s, for
    /// choosing its table (`FabricDimensions::with_table`).
    #[test]
    #[ignore]
    fn measure_tables() {
        use strum::IntoEnumIterator;
        for name in FabricName::iter() {
            let (fabric, _) = medium(name);
            let deck = fabric.deck().unwrap();
            let standing = fabric.joints.values().filter(|joint| joint.location.y < FOOTING);
            let feet = smallest_hexagon(standing.map(|joint| joint.location));
            let scale = fabric.dimensions.scale.f32();
            eprintln!(
                "{name}: {:.2} m tall, table {:.2} m ({:.1} scales), feet and anchors within {:.2} m ({:.1} scales)",
                fabric.height().f32(),
                deck.radius,
                deck.radius / scale,
                feet,
                feet / scale
            );
        }
    }

    /// Kept cues remembered in a file come back as the cues tried first.
    #[test]
    fn kept_cues_are_remembered() {
        let path = std::env::temp_dir().join("tensegrity-lab-cues").join("remembered.txt");
        let kept: Vec<Kept> = [Cue::rocking(SIDE, 2.0), Cue::swaying(SIDE, FORE, 1.3)]
            .into_iter()
            .map(|cue| Kept { cue, score: 0.8, swing: 0.04, most: "L12 side".to_string(), quickness: 1.0, shape: Vec::new() })
            .collect();
        remember(&path, &kept);
        let cues = remembered(&path);
        assert_eq!(cues, kept.iter().map(|kept| kept.cue.clone()).collect::<Vec<_>>());
        assert_eq!(Evolution::new(cues, Scoring::QUAKE).next(), kept[0].cue);
    }

    /// Quake pressed on `fabric`, as the button does.
    fn press(fabric: &mut Fabric, physics: &Physics) {
        let age = fabric.age;
        let piece = fabric.quake.as_ref().is_some_and(|quake| !quake.running()).then(|| (fabric.clone(), physics.clone()));
        fabric.quake.as_mut().unwrap().advance(age, piece, None);
    }

    /// A Medium Headless Hug on its wobble table, Quake pressed once: it
    /// evolves cues on copies of itself, keeps several unlike one another
    /// that swing the piece, and plays them. Pressed again the drums go
    /// back to their middles, and the piece stands where it stood.
    #[test]
    fn medium_headless_hug_rides_the_wobble_table() {
        let (mut fabric, physics) = medium(FabricName::HeadlessHug);
        assert_eq!(fabric.quake.as_ref().map(|quake| quake.watched.len()), Some(fabric.joints.len()), "it watches every joint");
        fabric.take_motion();
        let before: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
        let per_second = Age::iterations_per_second() as usize;
        press(&mut fabric, &physics);
        while fabric.quake.as_ref().unwrap().tries() < 160 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let age = fabric.age;
        fabric.quake.as_mut().unwrap().demo(age);
        assert!(fabric.quake.as_ref().unwrap().demoing(), "the demo plays");
        let mut ticks = 0;
        while fabric.quake.as_ref().unwrap().playing().is_none() {
            fabric.iterate(&physics);
            ticks += 1;
            assert!(ticks < 60 * per_second, "it played what it kept within a minute");
        }
        while !fabric.quake.as_ref().unwrap().waiting() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let tries = fabric.quake.as_ref().unwrap().tries();
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert_eq!(fabric.quake.as_ref().unwrap().tries(), tries, "evolving waits while the demo plays");
        let quake = fabric.quake.as_ref().unwrap();
        for kept in quake.kept() {
            eprintln!("{} scored {:.2}, {:.0} mm, most {}", kept.cue.describe(), kept.score, kept.swing * 1000.0, kept.most);
        }
        let kept = quake.kept();
        assert!(kept.len() > 1, "it kept {} cues", kept.len());
        for (index, a) in kept.iter().enumerate() {
            for b in &kept[index + 1..] {
                assert!(Quake::alike(&a.shape, &b.shape) < DISTINCT, "two kept cues swing alike");
            }
        }
        for _ in 0..20 * per_second {
            fabric.iterate(&physics);
        }
        press(&mut fabric, &physics);
        assert!(!fabric.quake.as_ref().unwrap().running(), "pressed again, it stops");
        for _ in 0..30 * per_second {
            fabric.iterate(&physics);
        }
        assert!(fabric.joints.values().all(|joint| joint.location.is_finite()), "the figure diverged");
        let after = fabric.joints.values().zip(&before).map(|(joint, b)| joint.location.distance(*b));
        let settled = after.fold(0.0, f32::max);
        let motion = fabric.take_motion().expect("motion was measured");
        eprintln!(
            "{settled:.3} m off after, cable max {:.0} N, strut max {:.0} N, slack {}",
            motion.max_cable_force.0, motion.max_strut_force.0, motion.slack_cable_count
        );
        assert!(settled < 0.03, "it ended {:.0} mm from where it stood", settled * 1000.0);
    }

    /// Evolve cues for a Medium piece (`PROBE` names the fabric, Headless
    /// Hug by default; `TRIES` how many, 200 by default), and print those
    /// kept as DMX loops, for the electronics.
    #[test]
    #[ignore]
    fn evolve_wobble_cues() {
        let name: FabricName = std::env::var("PROBE").ok().and_then(|name| name.parse().ok()).unwrap_or(FabricName::HeadlessHug);
        let tries: u32 = std::env::var("TRIES").ok().and_then(|tries| tries.parse().ok()).unwrap_or(200);
        let (mut fabric, physics) = medium(name);
        let started = std::time::Instant::now();
        press(&mut fabric, &physics);
        let quake = fabric.quake.as_ref().unwrap();
        let mut reported = 0;
        while quake.tries() < tries {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if quake.tries() != reported {
                reported = quake.tries();
                let best: Vec<String> = quake.kept().iter().map(|kept| format!("{:.2}", kept.score)).collect();
                eprintln!("{:.1} s, {reported} tried: kept {}", started.elapsed().as_secs_f32(), best.join(" "));
            }
        }
        for (index, kept) in quake.kept().iter().enumerate() {
            eprintln!(
                "// {} {}: scored {:.2}, {:.0} mm, {:.1}/s, most {}\n{}",
                letter(index),
                kept.cue.describe(),
                kept.score,
                kept.swing * 1000.0,
                kept.quickness,
                kept.most,
                kept.cue.written()
            );
        }
    }
}
