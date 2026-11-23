//! The wobble table a Medium piece stands on: a hexagonal deck hung on three
//! short ties, at alternate corners, from arms rising off a fixed hexagonal
//! base. Hung so, the deck can't tilt; it sways on its ties like a pendulum
//! and twists about its middle. A stepper motor in the middle of the base
//! turns a crank, and a cable from the crank's pin to each of the deck's
//! hooks draws the deck after the pin. The piece's feet stand on the deck
//! and go with it, and pull on it in turn.
//!
//! The stepper plays cues (`Cue`), DMX loops of the crank's position, bred
//! out of sight: each tried on a copy of the piece as it stood when Quake
//! was pressed, `ROUND` at a time side by side. A try watches the piece's
//! hands (or, with none, its highest joints) swing against the deck,
//! scoring the cue for the hands swinging expressively and for how little
//! the crank works, and the best of each kind of swing is kept, unlike one
//! another. The piece itself plays what is kept: each new find a moment,
//! and once breeding has stopped finding, each in turn.

use crate::fabric::cue::Cue;
use crate::fabric::dimensions::Hardware;
use crate::fabric::joint::JointLabel;
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
/// The pin's height above the base.
pub const CAM_HEIGHT: f32 = 0.05;
/// The crank on the stepper's shaft: its pin this far from the axis.
pub const CRANK: f32 = 0.008;
/// The crank's zero: pointing forward, the way the piece faces, along its
/// mirror plane.
pub const CRANK_ZERO: f32 = TAU / 4.0;
/// The stepper's top speed, radians a second.
const TOP_SPEED: f32 = 4.0 * TAU;
/// Deck and frame, per square metre.
const DECK_DENSITY: f32 = 15.0;
/// The deck reaches this far past the farthest foot or guy anchor.
pub const DECK_MARGIN: f32 = 0.1;
/// How much of its swing the deck loses each cycle, as a damping ratio.
const DAMPING: f32 = 0.05;
/// Each cable's tension with the deck still and the crank at its zero.
/// Enough that none goes slack drawing the deck round.
const CABLE_PRETENSION: f32 = 300.0;
/// How far the hands should swing against the deck: expressive, no more.
const EXPRESSIVE: f32 = 0.04;
/// A cue is tried this long, listened to after it has had time to build.
const TRYING: Seconds = Seconds(12.0);
const SETTLING_IN: Seconds = Seconds(4.0);
/// A cue kept is played this long, and one just found this long.
const PLAYING: Seconds = Seconds(15.0);
const SHOWING: Seconds = Seconds(5.0);
/// With nothing kept yet, it looks again this often.
const WAITING: Seconds = Seconds(0.25);
/// Cues tried side by side, a round of them.
const ROUND: usize = 8;
/// Breeding stops after this many rounds finding nothing new or better, or
/// after this many tries.
const STALE: u32 = 10;
const MOST_TRIES: u32 = 400;
/// A kept cue bettered by this much is a find.
const NOTABLE: f32 = 0.05;
/// It keeps at most this many cues.
const KEPT: usize = 6;
/// How much the crank's work counts against a cue, against its hands'
/// swing: a turn every half second costs this much.
const EFFORT: f32 = 0.3;
/// Two swings less alike than this (`Quake::alike`) are of different kinds.
pub const DISTINCT: f32 = 0.6;
/// A swing's shape is followed in this many steps through its cue's loop.
const BINS: usize = 12;
/// How long the hands' swing is remembered, as it dies away, and how long
/// where they stand is averaged over.
const RESPONSE_MEMORY: Seconds = Seconds(1.0);
const STANCE_MEMORY: Seconds = Seconds(3.0);

/// Which way the hexagon's corner `index` lies, from the middle, measured
/// from +x toward +z, the way the piece faces: corners 0, 2 and 4 at 30°,
/// 150° and 270°.
pub fn corner_angle(index: usize) -> f32 {
    (index as f32 * 60.0 + 30.0).to_radians()
}

/// The y of a hook's foot, where its tie and its cable are tied.
pub fn hook_foot() -> f32 {
    -DECK_THICKNESS - HEAD_CLEARANCE - TIE_LENGTH
}

/// The stepper's axis.
pub fn cam_axis() -> Vec3 {
    Vec3::new(0.0, -DECK_HEIGHT + BASE_THICKNESS + CAM_HEIGHT, 0.0)
}

/// How far the pin is from the stepper's axis, the crank at `angle` from
/// its zero.
pub fn crank_pin(angle: f32) -> Vec3 {
    let angle = CRANK_ZERO + angle;
    Vec3::new(angle.cos(), 0.0, angle.sin()) * CRANK
}

/// Where each hook's cable is tied, at its foot's inner corner, on the deck
/// at rest.
pub fn cable_ends(radius: f32) -> [Vec3; 3] {
    HANG_CORNERS.map(|corner| {
        let angle = corner_angle(corner);
        Vec3::new(angle.cos(), 0.0, angle.sin()) * (TIE_RADIUS * radius - HOOK_REACH) + Vec3::Y * hook_foot()
    })
}

/// `angle` brought round to between a half turn back and a half turn on.
fn wrapped(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

/// A cue kept: how well it did, how far the hands swung, and the shape of
/// their swing (`Quake::alike`).
#[derive(Clone, Debug)]
pub struct Kept {
    pub cue: Cue,
    pub score: f32,
    pub swing: f32,
    shape: Vec<f32>,
}

/// What the stepper does: try a cue, on a copy of the piece, play one kept
/// for so long, or wait at its zero for one to be found.
#[derive(Clone, Debug, PartialEq)]
enum Task {
    Try(Cue),
    Play(usize, Cue, Seconds),
    Wait,
}

/// The breeding: the seeds still to try, the cues kept, unlike one another,
/// each named by where it is kept, and how many have been tried.
#[derive(Clone, Debug)]
struct Breeder {
    seeds: VecDeque<Cue>,
    kept: Vec<Kept>,
    tries: u32,
    random: ChaCha8Rng,
}

impl Breeder {
    /// Breeding from `remembered` cues, tried first, and then the seeds.
    fn new(remembered: Vec<Cue>) -> Breeder {
        let seeds = remembered.into_iter().chain(Cue::seeds()).collect();
        Breeder { seeds, kept: Vec::new(), tries: 0, random: ChaCha8Rng::seed_from_u64(1) }
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

    /// A cue tried: kept if the hands swung and no kept cue's swing is like
    /// it, or in place of the one whose is (only one) but did less well, or
    /// of the worst when there are enough; and where, if it is a find: a new
    /// kind, or `NOTABLE` better.
    fn keep(&mut self, found: Kept) -> Option<usize> {
        self.tries += 1;
        if found.swing <= EXPRESSIVE / 2.0 {
            return None;
        }
        let alike: Vec<usize> =
            (0..self.kept.len()).filter(|&index| Quake::alike(&self.kept[index].shape, &found.shape) >= DISTINCT).collect();
        match alike[..] {
            [index] if found.score > self.kept[index].score => {
                let better = found.score - self.kept[index].score;
                self.kept[index] = found;
                (better >= NOTABLE).then_some(index)
            }
            [_, ..] => None,
            [] if self.kept.len() < KEPT => {
                self.kept.push(found);
                Some(self.kept.len() - 1)
            }
            [] => {
                let worst = (0..self.kept.len()).min_by(|&a, &b| self.kept[a].score.total_cmp(&self.kept[b].score))?;
                (found.score > self.kept[worst].score).then(|| {
                    self.kept[worst] = found;
                    worst
                })
            }
        }
    }
}

/// The breeding, shared with the rounds run out of sight: where each find
/// was kept, in turn, whether it is still breeding, whether it should
/// stop, and where what it keeps is remembered.
#[derive(Debug)]
struct Shared {
    breeder: Breeder,
    finds: Vec<usize>,
    breeding: bool,
    stop: bool,
    memory: Option<PathBuf>,
}

/// Where the cues kept for the piece `name` are remembered between runs.
pub fn cues_file(name: &str) -> PathBuf {
    PathBuf::from("cues").join(format!("{}.txt", name.to_lowercase().replace(' ', "-")))
}

/// The cues remembered in `path`: each line a kept cue, what the stepper
/// is told after `|`.
fn remembered(path: &Path) -> Vec<Cue> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| Cue::parse(line.split_once('|')?.1))
        .collect()
}

/// Remember `kept` in `path`, A first: its name, score, how far the hands
/// swung, what it is (`Cue::describe`), and after `|` its DMX frames.
fn remember(path: &Path, kept: &[Kept]) {
    let mut text = String::from("# Wobble table cues, kept by Quake and tried first next time.\n");
    text += "# name score hands description | DMX frames, 44 a second, 128 the crank's zero\n";
    for (index, kept) in kept.iter().enumerate() {
        let frames: Vec<String> = kept.cue.frames.iter().map(u8::to_string).collect();
        text += &format!(
            "{} {:.2} {:.0}mm {} | {}\n",
            letter(index),
            kept.score,
            kept.swing * 1000.0,
            kept.cue.describe(),
            frames.join(" ")
        );
    }
    let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(path, text));
    if let Err(error) = written {
        eprintln!("Couldn't remember the cues in {}: {error}", path.display());
    }
}

/// The piece's link to its breeding. A copy of the piece has none, and when
/// the piece goes, its breeding stops.
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
fn trials(piece: &Fabric, physics: &Physics, cues: Vec<Cue>) -> Vec<Option<Kept>> {
    #[cfg(not(target_arch = "wasm32"))]
    return std::thread::scope(|scope| {
        let tries: Vec<_> = cues.into_iter().map(|cue| scope.spawn(move || trial(piece, physics, cue))).collect();
        tries.into_iter().map(|try_| try_.join().unwrap()).collect()
    });
    #[cfg(target_arch = "wasm32")]
    cues.into_iter().map(|cue| trial(piece, physics, cue)).collect()
}

/// `cue` tried on a copy of `piece`, for as long as a try lasts.
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

/// Breed in rounds, each cue tried on its own copy of `piece`, until it
/// stops finding or is told to stop.
fn breed(shared: Arc<Mutex<Shared>>, piece: Fabric, physics: Physics) {
    let mut stale = 0;
    loop {
        let cues: Vec<Cue> = {
            let mut shared = shared.lock().unwrap();
            if shared.stop {
                break;
            }
            (0..ROUND).map(|_| shared.breeder.next()).collect()
        };
        let found = trials(&piece, &physics, cues);
        let mut shared = shared.lock().unwrap();
        let before = shared.finds.len();
        for found in found.into_iter().flatten() {
            if let Some(index) = shared.breeder.keep(found) {
                shared.finds.push(index);
            }
        }
        stale = if shared.finds.len() > before { 0 } else { stale + 1 };
        if let (Some(memory), true) = (&shared.memory, shared.finds.len() > before) {
            remember(memory, &shared.breeder.kept);
        }
        if stale >= STALE || shared.breeder.tries >= MOST_TRIES {
            break;
        }
    }
    shared.lock().unwrap().breeding = false;
}

/// A kept cue's name: A to F, by where it is kept.
fn letter(index: usize) -> char {
    (b'A' + index as u8) as char
}

/// A joint the motor watches, and where it has stood on the deck lately.
#[derive(Clone, Debug)]
struct Watched {
    key: JointKey,
    stance: Vec3,
}

/// What it hears while a cue plays, once it has settled in: the hands'
/// swing, the crank's work, and the hands' way from where they stand
/// through the loop, `BINS` steps of it for each.
#[derive(Clone, Debug, Default)]
struct Hearing {
    ticks: u32,
    swing: f32,
    work: f32,
    shape: Vec<Vec3>,
    counts: Vec<u32>,
}

#[derive(Clone, Debug)]
struct Motor {
    since: Age,
    stopping: bool,
    /// The crank's angle from its zero, and how fast it turned last tick.
    angle: f32,
    speed: f32,
    hearing: Hearing,
}

/// Where the deck is: moved this far from where it hangs still and turned
/// this far about its middle, and how far the pin is from the stepper's
/// axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Deck {
    pub offset: Vec3,
    pub turn: f32,
    pub radius: f32,
    pub pin: Vec3,
}

impl Deck {
    /// Where a point resting on the deck at rest is now.
    fn carry(&self, point: Vec3) -> Vec3 {
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
    /// Each cable's rest length, and its stiffness, newtons a metre.
    cables: [f32; 3],
    cable_stiffness: f32,
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
    /// What the stepper plays, and what comes once it is back at its zero.
    task: Option<Task>,
    next: Option<Task>,
    /// The breeding, and how many of its finds have been played.
    search: Search,
    played: usize,
    /// A cue tried on a copy of the piece, once its try is done.
    found: Option<Kept>,
}

impl Quake {
    /// A table `radius` across its corners, still, its crank at its zero,
    /// nothing on it yet.
    pub fn of_radius(radius: f32) -> Quake {
        let area = 1.5 * 3.0f32.sqrt() * radius * radius;
        let mass = DECK_DENSITY * area;
        let home = cam_axis() + crank_pin(0.0);
        let spans = cable_ends(radius).map(|end| end.distance(home));
        let ea = Hardware::MEDIUM.cable_stiffness.0;
        let cable_stiffness = ea / (spans.iter().sum::<f32>() / 3.0);
        Quake {
            deck: Deck { radius, pin: crank_pin(0.0), ..Deck::default() },
            velocity: Vec3::ZERO,
            spin: 0.0,
            mass,
            weight: mass * EARTH_GRAVITY.f32(),
            cables: spans.map(|span| span - CABLE_PRETENSION / cable_stiffness),
            cable_stiffness,
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
            found: None,
        }
    }

    /// The piece `fabric` landed on the table: its weight on the ties, and
    /// `hands` watched, or with none the highest joint off its symmetry
    /// axis or plane, with its images, or with no symmetry its three
    /// highest joints.
    pub fn mount(&mut self, fabric: &Fabric, hands: &[JointKey]) {
        let piece = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum::<f32>() / 1000.0;
        self.weight = (self.mass + piece) * EARTH_GRAVITY.f32();
        let mut watched: Vec<JointKey> = hands.iter().copied().filter(|&key| fabric.joints.contains_key(key)).collect();
        if watched.is_empty() {
            let twin = |key: JointKey| match fabric.joints[key].label {
                Some(JointLabel::Twin { number, .. }) => Some(number),
                _ => None,
            };
            let highest = fabric.joints.keys().filter(|&key| twin(key).is_some()).max_by(|&a, &b| {
                fabric.joints[a].location.y.total_cmp(&fabric.joints[b].location.y)
            });
            if let Some(number) = highest.and_then(twin) {
                watched = fabric.joints.keys().filter(|&key| twin(key) == Some(number)).collect();
            }
        }
        if watched.is_empty() {
            let mut highest: Vec<JointKey> = fabric.joints.keys().collect();
            highest.sort_by(|&a, &b| fabric.joints[b].location.y.total_cmp(&fabric.joints[a].location.y));
            watched = highest.into_iter().take(3).collect();
        }
        self.watched = watched.into_iter().map(|key| Watched { key, stance: fabric.joints[key].location }).collect();
    }

    /// Whether a piece has landed on it.
    pub fn mounted(&self) -> bool {
        !self.watched.is_empty()
    }

    pub fn deck(&self) -> Deck {
        self.deck
    }

    /// How far the watched joints have swung against the deck lately.
    pub fn response(&self) -> f32 {
        self.response
    }

    /// What the breeding has come to: the cues kept, where each find was
    /// kept in turn, how many tried, and whether it is still breeding.
    fn shared<T>(&self, look: impl FnOnce(&Shared) -> T) -> Option<T> {
        self.search.0.as_ref().map(|shared| look(&shared.lock().unwrap()))
    }

    /// The cues kept so far, A first.
    pub fn kept(&self) -> Vec<Kept> {
        self.shared(|shared| shared.breeder.kept.clone()).unwrap_or_default()
    }

    /// How many cues it has tried.
    pub fn tries(&self) -> u32 {
        self.shared(|shared| shared.breeder.tries).unwrap_or(0)
    }

    /// Whether it is still breeding.
    pub fn breeding(&self) -> bool {
        self.shared(|shared| shared.breeding).unwrap_or(false)
    }

    /// The kept cue it plays, if it plays one.
    pub fn playing(&self) -> Option<usize> {
        match (&self.task, self.running()) {
            (Some(Task::Play(index, ..)), true) => Some(*index),
            _ => None,
        }
    }

    /// What the stepper is doing, for the subtitle, briefly: the kept cue
    /// it plays, by its letter, and what the stepper is told
    /// (`Cue::describe`), and while it breeds, how many tried.
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
        let (tries, breeding) = self.shared(|shared| (shared.breeder.tries, shared.breeding)).unwrap_or_default();
        let playing = match task {
            Task::Play(_, cue, _) => format!("{} {}", name(task), cue.describe()),
            Task::Try(cue) => cue.describe(),
            Task::Wait => "breeding".to_string(),
        };
        if breeding { format!("{playing} · {tries} tried") } else { playing }
    }

    /// The Quake button: it breeds cues on copies of `piece`, as it stands
    /// now, playing what it keeps, until pressed again. With a `memory`,
    /// the cues kept there before are tried first, and what it keeps is
    /// kept there.
    pub fn advance(&mut self, age: Age, piece: Option<(Fabric, Physics)>, memory: Option<PathBuf>) {
        if self.running() {
            self.then(age, None);
            if let Some(shared) = &self.search.0 {
                shared.lock().unwrap().stop = true;
            }
            return;
        }
        let shared = self.search.0.get_or_insert_with(|| {
            let remembered = memory.as_deref().map(remembered).unwrap_or_default();
            Arc::new(Mutex::new(Shared { breeder: Breeder::new(remembered), finds: Vec::new(), breeding: false, stop: false, memory }))
        });
        let start = {
            let mut shared = shared.lock().unwrap();
            let start = !shared.breeding;
            (shared.breeding, shared.stop) = (true, false);
            start
        };
        if let (true, Some((piece, physics))) = (start, piece) {
            let shared = Arc::clone(shared);
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || breed(shared, piece, physics));
            #[cfg(target_arch = "wasm32")]
            breed(shared, piece, physics);
        }
        self.go_on(age);
    }

    /// Bring the crank back to its zero and stop.
    pub fn stop(&mut self, age: Age) {
        self.then(age, None);
    }

    /// On to what is next: the newest find, a moment, or else the next
    /// kept cue, a moment while it breeds, or with none yet, waiting for
    /// one.
    fn go_on(&mut self, age: Age) {
        let (finds, kept, breeding) =
            self.shared(|shared| (shared.finds.clone(), shared.breeder.kept.clone(), shared.breeding)).unwrap_or_default();
        let next = if finds.len() > self.played {
            self.played = finds.len();
            let index = finds[finds.len() - 1];
            Task::Play(index, kept[index].cue.clone(), SHOWING)
        } else if !kept.is_empty() {
            let index = match self.task {
                Some(Task::Play(index, ..)) => (index + 1) % kept.len(),
                _ => 0,
            };
            Task::Play(index, kept[index].cue.clone(), if breeding { SHOWING } else { PLAYING })
        } else {
            Task::Wait
        };
        self.then(age, Some(next));
    }

    /// Bring the crank back to its zero, then do `next`.
    fn then(&mut self, age: Age, next: Option<Task>) {
        self.next = next;
        match &mut self.motor {
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
            self.motor = Some(Motor { since: age, stopping: false, angle: 0.0, speed: 0.0, hearing });
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
        let work = hearing.work / ticks / (2.0 * TAU);
        let expressive = swing / EXPRESSIVE;
        let expressive = if expressive <= 1.0 { expressive } else { (2.0 - expressive).max(0.0) };
        let score = expressive - EFFORT * work;
        let shape: Vec<f32> = hearing
            .shape
            .iter()
            .zip(&hearing.counts)
            .flat_map(|(sum, &count)| (*sum / count.max(1) as f32).to_array())
            .collect();
        self.found = Some(Kept { cue, score, swing, shape });
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

    /// Whether the stepper is playing, not stopping.
    pub fn running(&self) -> bool {
        self.motor.as_ref().is_some_and(|motor| !motor.stopping)
    }

    /// Whether the stepper is playing, or bringing its crank back to zero.
    pub fn busy(&self) -> bool {
        self.motor.is_some()
    }

    /// Whether the deck is still going, with the motor or after it.
    pub fn moving(&self) -> bool {
        self.motor.is_some() || self.velocity.length() > 1e-3 || self.spin.abs() > 1e-3
    }

    /// The stepper this tick: after its cue's position, or back to its zero,
    /// as fast as it can go; and done with the cue once it has had its time.
    fn turn_crank(&mut self, age: Age, dt: f32) {
        let Some(motor) = &self.motor else {
            return;
        };
        let elapsed = age.elapsed_since(motor.since).0;
        let target = match (motor.stopping, self.cue()) {
            (false, Some(cue)) => Cue::angle(cue.value(elapsed)),
            _ => 0.0,
        };
        let motor = self.motor.as_mut().unwrap();
        let toward = wrapped(target - motor.angle);
        let turned = toward.clamp(-TOP_SPEED * dt, TOP_SPEED * dt);
        motor.angle += turned;
        motor.speed = turned / dt;
        self.deck.pin = crank_pin(motor.angle);
        if motor.stopping {
            if wrapped(motor.angle).abs() < 1e-4 {
                self.motor = None;
                self.deck.pin = crank_pin(0.0);
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

    /// One tick of the deck: the stepper, the ties pulling it back, the
    /// cables, damping, and what the piece's feet pulled with.
    fn step(&mut self, age: Age, dt: f32) {
        self.turn_crank(age, dt);
        let stiffness = self.weight / TIE_LENGTH;
        let tie_radius = TIE_RADIUS * self.deck.radius;
        let twist_stiffness = stiffness * tie_radius * tie_radius;
        let inertia = self.mass * 5.0 / 12.0 * self.deck.radius * self.deck.radius;
        let held = stiffness + 1.5 * self.cable_stiffness;
        let sway_damping = 2.0 * DAMPING * (held * self.mass).sqrt();
        let twist_damping = 2.0 * DAMPING * (twist_stiffness * inertia).sqrt();

        let mut force = self.pull - self.deck.offset * stiffness - self.velocity * sway_damping;
        let mut torque = self.pull_twist - self.deck.turn * twist_stiffness - self.spin * twist_damping;
        let pin = cam_axis() + self.deck.pin;
        for (end, rest) in cable_ends(self.deck.radius).into_iter().zip(self.cables) {
            let at = self.deck.carry(end);
            let toward = pin - at;
            let tension = (self.cable_stiffness * (toward.length() - rest)).max(0.0);
            let pull = Vec3::new(toward.x, 0.0, toward.z) / toward.length() * tension;
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
    fn watch(&mut self, fabric_joints: impl Fn(JointKey) -> Option<Vec3>, age: Age, dt: f32) {
        let follow = dt / STANCE_MEMORY.0;
        let mut swing = 0.0f32;
        let mut aways = Vec::with_capacity(self.watched.len());
        for watched in &mut self.watched {
            let away = fabric_joints(watched.key).map_or(Vec3::ZERO, |at| self.deck.on_deck(at) - watched.stance);
            swing = swing.max(away.length());
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
        hearing.work += motor.speed.abs();
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
        quake.watch(|key| joints.get(key).map(|joint| joint.location), self.age, Age::iteration_duration().f32());
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

    fn medium(name: FabricName) -> (Fabric, Physics) {
        let mut executor = FabricPlanExecutor::new(fabric_library::get_fabric_plan(name).at_size(Size::Medium));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..VIEWING };
        (executor.fabric, physics)
    }

    /// Kept cues remembered in a file come back as the cues tried first.
    #[test]
    fn kept_cues_are_remembered() {
        let path = std::env::temp_dir().join("tensegrity-lab-cues").join("remembered.txt");
        let kept: Vec<Kept> = [Cue::rocking(2.0, 30.0), Cue::turning(1.3)]
            .into_iter()
            .map(|cue| Kept { cue, score: 0.8, swing: 0.04, shape: Vec::new() })
            .collect();
        remember(&path, &kept);
        let cues = remembered(&path);
        assert_eq!(cues, kept.iter().map(|kept| kept.cue.clone()).collect::<Vec<_>>());
        assert_eq!(Breeder::new(cues).next(), kept[0].cue);
    }

    /// Quake pressed on `fabric`, as the button does.
    fn press(fabric: &mut Fabric, physics: &Physics) {
        let age = fabric.age;
        let piece = fabric.quake.as_ref().is_some_and(|quake| !quake.running()).then(|| (fabric.clone(), physics.clone()));
        fabric.quake.as_mut().unwrap().advance(age, piece, None);
    }

    /// A Medium Headless Hug on its wobble table, Quake pressed once: it
    /// breeds cues on copies of itself, keeps several unlike one another
    /// that swing the hands, and plays them. Pressed again the crank goes
    /// back to its zero, and the piece stands where it stood.
    #[test]
    fn medium_headless_hug_rides_the_wobble_table() {
        let (mut fabric, physics) = medium(FabricName::HeadlessHug);
        assert_eq!(fabric.quake.as_ref().map(|quake| quake.watched.len()), Some(6), "it watches both hands");
        fabric.take_motion();
        let before: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
        let per_second = Age::iterations_per_second() as usize;
        press(&mut fabric, &physics);
        while fabric.quake.as_ref().unwrap().breeding() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let mut ticks = 0;
        while fabric.quake.as_ref().unwrap().playing().is_none() {
            fabric.iterate(&physics);
            ticks += 1;
            assert!(ticks < 60 * per_second, "it played what it kept within a minute");
        }
        let quake = fabric.quake.as_ref().unwrap();
        for kept in quake.kept() {
            eprintln!("{} scored {:.2}, hands {:.0} mm", kept.cue.describe(), kept.score, kept.swing * 1000.0);
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

    /// Breed cues for a Medium piece (`PROBE` names the fabric, Headless
    /// Hug by default) until it stops finding, and print those kept as DMX
    /// loops, for the electronics.
    #[test]
    #[ignore]
    fn breed_wobble_cues() {
        let name: FabricName = std::env::var("PROBE").ok().and_then(|name| name.parse().ok()).unwrap_or(FabricName::HeadlessHug);
        let (mut fabric, physics) = medium(name);
        let started = std::time::Instant::now();
        press(&mut fabric, &physics);
        let quake = fabric.quake.as_ref().unwrap();
        let mut reported = 0;
        while quake.breeding() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if quake.tries() != reported {
                reported = quake.tries();
                let best: Vec<String> = quake.kept().iter().map(|kept| format!("{:.2}", kept.score)).collect();
                eprintln!("{:.1} s, {reported} tried: kept {}", started.elapsed().as_secs_f32(), best.join(" "));
            }
        }
        for (index, kept) in quake.kept().iter().enumerate() {
            eprintln!(
                "// {} {}: scored {:.2}, hands {:.0} mm\n{:?}",
                letter(index),
                kept.cue.describe(),
                kept.score,
                kept.swing * 1000.0,
                kept.cue.frames
            );
        }
    }
}
