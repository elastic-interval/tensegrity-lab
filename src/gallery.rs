//! The gallery (`--gallery`): Medium pieces on their wobble tables, each
//! playing the cues evolved for it at its size in turn (`cues/gallery/`,
//! by the ignored `evolve_gallery_cues`), in a room walked through with
//! the trackpad or the arrow keys: a two-finger swipe walks and turns, a
//! pinch rises and sinks, dragging looks round and up; smoothly, as on a
//! gimbal. Nothing on the screen but the room and
//! what stands in it; Esc leaves.

use crate::build::dsl::fabric_library::{self, FabricName};
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::camera::OPENGL_TO_WGPU_MATRIX;
use crate::fabric::cue::{Cue, SIDE};
use crate::fabric::dimensions::MEDIUM_HEIGHT;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::Physics;
use crate::fabric::quake::{self, DECK_HEIGHT};
use crate::fabric::Fabric;
use crate::units::Unit;
use crate::Age;
use crate::units::Percent;
use glam::{Mat4, Quat, Vec3, Vec4};
use std::sync::mpsc::{channel, Receiver};

mod sensor;
use sensor::Sensor;
#[cfg(not(target_arch = "wasm32"))]
mod sound;

/// A piece in the gallery: where its table stands on the floor, x across
/// (to the left seen from the start) and z deep; how far it is turned,
/// degrees; how much taller than Medium it is made; how much of its
/// Medium pretension it keeps, less to sway more slowly and further; and
/// the joints carrying its movement sensors, the music's, as the ignored
/// `place_sensors` places them.
struct Station {
    name: FabricName,
    at: [f32; 2],
    facing: f32,
    taller: f32,
    pretension: f32,
    sensors: [&'static str; 3],
}

/// How many pieces the gallery shows.
pub const PIECES: usize = 4;

const STATIONS: [Station; PIECES] = [
    Station {
        name: FabricName::OpenClaw,
        at: [4.5, 4.0],
        facing: 0.0,
        taller: 1.25,
        pretension: 0.02,
        sensors: ["C19", "A14", "B11"],
    },
    Station {
        name: FabricName::HeadlessHug,
        at: [0.0, 6.0],
        facing: 180.0,
        taller: 1.25,
        pretension: 0.25,
        sensors: ["R62", "L62", "R47"],
    },
    Station {
        name: FabricName::Quadruped,
        at: [-4.5, 4.0],
        facing: 0.0,
        taller: 1.25,
        pretension: 0.25,
        sensors: ["L2", "R2", "R43"],
    },
    Station {
        name: FabricName::HaloByCrane,
        at: [0.0, -7.0],
        facing: 0.0,
        taller: 1.25,
        pretension: 1.0,
        sensors: ["A12.2", "A08.4", "A07.6"],
    },
];
/// The room, across and deep, and its walls' height, metres.
pub const WIDTH: f32 = 18.0;
pub const DEPTH: f32 = 22.0;
pub const WALL_HEIGHT: f32 = 3.5;
/// Where the walk starts, looking into the room.
const START: [f32; 2] = [0.0, -2.0];
/// The visitor's eyes above the floor, how fast they walk and turn, and
/// how close they come to a wall or a table.
const EYE: f32 = 1.7;
const WALK: f32 = 1.2;
const TURN: f32 = 1.0;
const KEEP_OFF: f32 = 0.4;
/// How far down the visitor looks at first, how far up or down they can
/// look, and how wide they see, radians.
const LOOK_DOWN: f32 = 0.15;
const LOOK_FARTHEST: f32 = 1.3;
const VIEW: f32 = 1.0;
/// A two-finger swipe: metres walked and radians turned for each pixel;
/// a line of a mouse wheel is this many pixels. Dragging: radians looked
/// for each pixel.
const SWIPE_WALK: f32 = 0.01;
const SWIPE_TURN: f32 = 0.003;
const LINE: f32 = 30.0;
const DRAG_LOOK: f32 = 0.004;
/// A pinch: metres risen for spreading the fingers as far as a pinch
/// counts one; and the lowest the eyes go.
const PINCH_RISE: f32 = 1.5;
const LOWEST_EYE: f32 = 0.4;
/// How long the eyes take to follow, moving and looking, about.
const MOVING: f32 = 0.8;
const LOOKING: f32 = 0.35;
/// What the trackpad does, along the bottom of the screen.
pub const INSTRUCTIONS: &str = "Two fingers: swipe to walk and turn  ·  pinch to rise and sink  ·  click and drag to look  ·  Esc to leave";
/// Each table's light: how far below the deck's top, in the table, and
/// how bright.
const LIGHT_DEPTH: f32 = 0.1;
const LIGHT: f32 = 1.0;
/// Each table's spot, in the middle of its deck under the piece, this high
/// and shining straight up at the ceiling, its cone this wide either side,
/// degrees, and how bright.
const SPOT_HEIGHT: f32 = 0.03;
const SPOT_CONE: f32 = 40.0;
const SPOT: f32 = 2.5;
/// For a piece with no cues evolved: rocking side to side.
const ROCKING: f32 = 1.0;
/// How long each cue plays before the next.
const PLAYING: f32 = 20.0;

/// The spots on the tables: each one's view for its shadow map, where it
/// is and how bright, and which way it shines with its cone's cosine.
pub struct Spots {
    pub views: [Mat4; PIECES],
    pub places: [Vec4; PIECES],
    pub aims: [Vec4; PIECES],
}

/// A piece on its table, where it stands in the room.
pub struct Piece {
    station: usize,
    pub fabric: Fabric,
    physics: Physics,
    pub place: Mat4,
    /// Its cues, the one playing, and how long it has played.
    cues: Vec<Cue>,
    playing: usize,
    played: f32,
    /// Its movement sensors, the ticks since one last read, and the turns
    /// they felt since the music last heard: which sensor, when, seconds of
    /// the piece's own time, and how hard.
    sensors: Vec<Sensor>,
    unread: usize,
    felt: Vec<(usize, f32, f32)>,
}

impl Piece {
    /// Where the table stands, and how far out it reaches.
    fn footprint(&self) -> (Vec3, f32) {
        let middle = self.place.transform_point3(Vec3::ZERO);
        let radius = self.fabric.deck().map_or(0.0, |deck| deck.radius);
        (middle, radius)
    }
}

/// The arrow keys held down.
#[derive(Default)]
pub struct Arrows {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
}

/// The visitor, moved as a camera on a gimbal is: what they ask for moves
/// where they mean to be, and their eyes follow it smoothly, easing in and
/// out without overshooting.
pub struct Walker {
    /// Where the eyes are, the floor at zero, and which way they look:
    /// heading, zero along the room's depth, and pitch, up above zero.
    at: Vec3,
    look: Vec3,
    /// How fast each is changing, following where they mean to be.
    moving: Vec3,
    looking: Vec3,
    /// Where they mean to be, and to look.
    goal: Vec3,
    goal_look: Vec3,
    pub arrows: Arrows,
    /// Where the pointer is while dragging.
    dragging: Option<(f32, f32)>,
}

impl Walker {
    fn new() -> Walker {
        let (at, look) = (Vec3::new(START[0], EYE, START[1]), Vec3::new(0.0, -LOOK_DOWN, 0.0));
        Walker {
            at,
            look,
            moving: Vec3::ZERO,
            looking: Vec3::ZERO,
            goal: at,
            goal_look: look,
            arrows: Arrows::default(),
            dragging: None,
        }
    }

    /// A two-finger swipe, or a mouse wheel's lines: forward and back
    /// walks, as it zooms in elsewhere; across turns.
    pub fn swipe(&mut self, across: f32, forward: f32, lines: bool) {
        let pixels = if lines { LINE } else { 1.0 };
        self.goal += ahead(self.goal_look.x) * forward * pixels * SWIPE_WALK;
        self.goal_look.x += across * pixels * SWIPE_TURN;
    }

    /// A pinch: spreading the fingers rises, closing them sinks.
    pub fn pinch(&mut self, spread: f32) {
        self.goal.y += spread * PINCH_RISE;
    }

    /// Pressed or let go: dragging from where the pointer is then.
    pub fn press(&mut self, pressed: bool, pointer: Option<(f32, f32)>) {
        self.dragging = if pressed { pointer } else { None };
    }

    /// The pointer moved to `x`, `y`: dragging, the room is turned with it,
    /// and tilted.
    pub fn pointer_moved(&mut self, x: f32, y: f32) {
        if let Some((from_x, from_y)) = self.dragging.as_mut().map(|at| std::mem::replace(at, (x, y))) {
            self.goal_look.x += (x - from_x) * DRAG_LOOK;
            self.goal_look.y += (y - from_y) * DRAG_LOOK;
        }
    }

    /// `seconds` on: the arrows move where they mean to be, kept off the
    /// walls and the tables, within reach of floor and ceiling; and the
    /// eyes follow.
    fn walk(&mut self, seconds: f32, tables: impl Iterator<Item = (Vec3, f32)>) {
        let arrow = |held: bool| if held { 1.0 } else { 0.0 };
        self.goal_look.x += (arrow(self.arrows.left) - arrow(self.arrows.right)) * TURN * seconds;
        let step = (arrow(self.arrows.forward) - arrow(self.arrows.back)) * WALK * seconds;
        let mut goal = self.goal + ahead(self.goal_look.x) * step;
        for (middle, radius) in tables {
            let away = Vec3::new(goal.x - middle.x, 0.0, goal.z - middle.z);
            let clear = radius + KEEP_OFF;
            if away.length() < clear {
                goal += away.try_normalize().unwrap_or(Vec3::X) * (clear - away.length());
            }
        }
        let (x, z) = (WIDTH / 2.0 - KEEP_OFF, DEPTH / 2.0 - KEEP_OFF);
        (goal.x, goal.z) = (goal.x.clamp(-x, x), goal.z.clamp(-z, z));
        goal.y = goal.y.clamp(LOWEST_EYE, WALL_HEIGHT - KEEP_OFF);
        self.goal = goal;
        self.goal_look.y = self.goal_look.y.clamp(-LOOK_FARTHEST, LOOK_FARTHEST);
        self.at = follow(self.at, self.goal, &mut self.moving, MOVING, seconds);
        self.look = follow(self.look, self.goal_look, &mut self.looking, LOOKING, seconds);
    }

    /// Where the eyes are in the pieces' frame, whose decks are at zero.
    pub fn eye(&self) -> Vec3 {
        self.at - Vec3::Y * DECK_HEIGHT
    }

    pub fn mvp(&self, width: f32, height: f32) -> Mat4 {
        let look = ahead(self.look.x) * self.look.y.cos() + Vec3::Y * self.look.y.sin();
        let view = Mat4::look_at_rh(self.eye(), self.eye() + look, Vec3::Y);
        let projection = Mat4::perspective_rh(VIEW, width / height, 0.05, 100.0);
        OPENGL_TO_WGPU_MATRIX * projection * view
    }
}

/// Along the floor, `heading` from the room's depth.
fn ahead(heading: f32) -> Vec3 {
    Vec3::new(heading.sin(), 0.0, heading.cos())
}

/// `now` following `goal` for `seconds` on a critically damped spring that
/// takes about `smoothing` to get there, `speed` how fast it goes.
fn follow(now: Vec3, goal: Vec3, speed: &mut Vec3, smoothing: f32, seconds: f32) -> Vec3 {
    let stiffness = 2.0 / smoothing;
    let x = stiffness * seconds;
    let decay = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let off = now - goal;
    let pull = (*speed + off * stiffness) * seconds;
    *speed = (*speed - pull * stiffness) * decay;
    goal + (off + pull) * decay
}

pub struct Gallery {
    built: Receiver<(usize, Fabric, Physics)>,
    pub pieces: Vec<Piece>,
    pub walker: Walker,
    /// The music, where there is an output to play it on.
    #[cfg(not(target_arch = "wasm32"))]
    sound: Option<sound::Sound>,
}

impl Gallery {
    /// The pieces start building out of sight, side by side, each joining
    /// the room once it stands.
    pub fn new() -> Gallery {
        let (send, built) = channel();
        for (index, station) in STATIONS.iter().enumerate() {
            let send = send.clone();
            let build = move || {
                let (fabric, physics) = medium(station);
                let _ = send.send((index, fabric, physics));
            };
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(build);
            #[cfg(target_arch = "wasm32")]
            build();
        }
        Gallery {
            built,
            pieces: Vec::new(),
            walker: Walker::new(),
            #[cfg(not(target_arch = "wasm32"))]
            sound: sound::Sound::new(),
        }
    }

    /// The light in each table, where it is and how bright. Before every
    /// piece has joined the room the missing ones are as good as dark, so
    /// the room is dark from the start.
    pub fn lights(&self) -> [Vec4; PIECES] {
        let mut lights = [Vec4::new(0.0, 0.0, 0.0, f32::MIN_POSITIVE); PIECES];
        for (light, piece) in lights.iter_mut().zip(&self.pieces) {
            let middle = piece.fabric.deck().map_or(Vec3::ZERO, |deck| deck.offset);
            *light = piece.place.transform_point3(middle - Vec3::Y * LIGHT_DEPTH).extend(LIGHT);
        }
        lights
    }

    /// The spots, going with their decks; none dark until its piece is in
    /// the room.
    pub fn spots(&self) -> Spots {
        let mut spots = Spots { views: [Mat4::IDENTITY; PIECES], places: [Vec4::ZERO; PIECES], aims: [Vec4::Z; PIECES] };
        for (index, piece) in self.pieces.iter().enumerate() {
            let Some(deck) = piece.fabric.deck() else {
                continue;
            };
            let at = |local: Vec3| piece.place.transform_point3(deck.carry(local));
            let from = at(Vec3::Y * SPOT_HEIGHT);
            let aim = Vec3::Y;
            let cone = SPOT_CONE.to_radians();
            let projection = Mat4::perspective_rh(2.0 * cone, 1.0, 0.05, 30.0);
            spots.views[index] = projection * Mat4::look_at_rh(from, from + aim, Vec3::Z);
            spots.places[index] = from.extend(SPOT);
            spots.aims[index] = aim.extend(cone.cos());
        }
        spots
    }

    /// `seconds` on: pieces built since join the room, the visitor walks,
    /// and every piece moves on `iterations`.
    pub fn advance(&mut self, seconds: f32, iterations: usize) {
        for (index, fabric, physics) in self.built.try_iter() {
            let Station { name, at: [x, z], facing, sensors, .. } = STATIONS[index];
            let sensors = sensors.iter().filter_map(|name| Sensor::on(&fabric, name)).collect();
            let place = Mat4::from_rotation_translation(Quat::from_rotation_y(facing.to_radians()), Vec3::new(x, 0.0, z));
            let mut cues = cues(name);
            if cues.is_empty() {
                cues.push(Cue::rocking(SIDE, ROCKING));
            }
            let mut piece =
                Piece { station: index, fabric, physics, place, cues, playing: 0, played: PLAYING, sensors, unread: 0, felt: Vec::new() };
            piece.playing = piece.cues.len() - 1;
            self.pieces.push(piece);
        }
        self.walker.walk(seconds, self.pieces.iter().map(Piece::footprint));
        for piece in &mut self.pieces {
            piece.played += seconds;
            if piece.played >= PLAYING {
                piece.played = 0.0;
                piece.playing = (piece.playing + 1) % piece.cues.len();
                let (age, cue) = (piece.fabric.age, piece.cues[piece.playing].clone());
                if let Some(table) = &mut piece.fabric.quake {
                    table.play(age, cue);
                }
            }
            for _ in 0..iterations {
                piece.fabric.iterate(&piece.physics);
                piece.unread += 1;
                if piece.unread == sensor::SENSING {
                    piece.unread = 0;
                    let now = piece.fabric.age.as_duration().as_secs_f32();
                    let dt = sensor::SENSING as f32 * Age::iteration_duration().f32();
                    for (index, sensor) in piece.sensors.iter_mut().enumerate() {
                        if let Some(hard) = sensor.read(&piece.fabric, now, dt) {
                            piece.felt.push((index, now, hard));
                        }
                    }
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.listen();
    }

    /// Tell the music what each piece's sensors felt, and where it is from
    /// the visitor.
    #[cfg(not(target_arch = "wasm32"))]
    fn listen(&mut self) {
        let Some(sound) = &self.sound else {
            return;
        };
        let right = Vec3::new(-self.walker.look.x.cos(), 0.0, self.walker.look.x.sin());
        for piece in &mut self.pieces {
            let (middle, _) = piece.footprint();
            let toward = Vec3::new(middle.x - self.walker.at.x, 0.0, middle.z - self.walker.at.z);
            let side = toward.try_normalize().map_or(0.0, |toward| toward.dot(right));
            let now = piece.fabric.age.as_duration().as_secs_f32();
            sound.tell(piece.station, now, &piece.felt, toward.length(), side);
            piece.felt.clear();
        }
    }
}

/// The piece at `station` built at Medium, as tall and as tight as it
/// says, on its table.
fn medium(station: &Station) -> (Fabric, Physics) {
    medium_made(station, station.taller, station.pretension)
}

/// The piece at `station` built at Medium, `taller` than Medium, keeping
/// `pretension` of its Medium pretension.
fn medium_made(station: &Station, taller: f32, pretension: f32) -> (Fabric, Physics) {
    let mut plan = fabric_library::get_fabric_plan(station.name).at_medium_height(MEDIUM_HEIGHT * taller);
    plan.pretense_phase.pretenst = Percent(plan.pretense_phase.pretenst.0 * pretension);
    let mut executor = FabricPlanExecutor::new(plan);
    while !executor.is_complete() {
        let _ = executor.iterate();
    }
    let physics = Physics { surface: executor.physics.surface, ..VIEWING };
    (executor.fabric, physics)
}

/// The cues evolved for the piece `name` at its gallery size, built in.
fn cues(name: FabricName) -> Vec<Cue> {
    quake::kept_cues(match name {
        FabricName::OpenClaw => include_str!("../cues/gallery/open-claw.txt"),
        FabricName::HeadlessHug => include_str!("../cues/gallery/headless-hug.txt"),
        FabricName::Quadruped => include_str!("../cues/gallery/quadruped.txt"),
        FabricName::HaloByCrane => include_str!("../cues/gallery/halo-by-crane.txt"),
        _ => "",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::quake::Scoring;

    /// How the gallery's cues are evolved: a swing seen from across the
    /// room, half again the Quake button's, still slow.
    const SCORING: Scoring = Scoring { expressive: 0.06, ..Scoring::QUAKE };

    /// Where to place each gallery piece's movement sensors: every joint's
    /// movement measured, as an accelerometer on it would feel it, while
    /// each of its cues plays from rest, each cue counting alike; the three
    /// moving most, each at least `APART` of the piece's height from the
    /// others, so they feel different parts of it.
    #[test]
    #[ignore]
    fn place_sensors() {
        const SETTLING: f32 = 4.0;
        const LISTENING: f32 = 12.0;
        const APART: f32 = 0.3;
        for station in &STATIONS {
            let (fabric, physics) = medium(station);
            let per_second = crate::Age::iterations_per_second() as usize;
            let moved: Vec<Vec<f32>> = std::thread::scope(|scope| {
                let tries: Vec<_> = cues(station.name)
                    .into_iter()
                    .map(|cue| {
                        let (mut fabric, physics) = (fabric.clone(), &physics);
                        scope.spawn(move || {
                            let age = fabric.age;
                            fabric.quake.as_mut().unwrap().play(age, cue);
                            let mut moved = vec![0.0f32; fabric.joints.len()];
                            for tick in 0..((SETTLING + LISTENING) * per_second as f32) as usize {
                                fabric.iterate(physics);
                                if tick >= SETTLING as usize * per_second && tick % sensor::SENSING == 0 {
                                    for (sum, joint) in moved.iter_mut().zip(fabric.joints.values()) {
                                        *sum += joint.velocity.length_squared();
                                    }
                                }
                            }
                            let most = moved.iter().copied().fold(f32::MIN_POSITIVE, f32::max);
                            moved.iter().map(|sum| sum / most).collect()
                        })
                    })
                    .collect();
                tries.into_iter().map(|try_| try_.join().unwrap()).collect()
            });
            let joints: Vec<_> = fabric.joints.values().collect();
            let mut ranked: Vec<usize> = (0..joints.len()).collect();
            let score = |index: usize| moved.iter().map(|cue| cue[index]).sum::<f32>();
            ranked.sort_by(|&a, &b| score(b).total_cmp(&score(a)));
            let apart = APART * fabric.height().0;
            let mut placed: Vec<usize> = Vec::new();
            for index in ranked {
                if placed.len() < 3 && placed.iter().all(|&other| joints[other].location.distance(joints[index].location) >= apart) {
                    placed.push(index);
                }
            }
            let names: Vec<String> = placed
                .iter()
                .map(|&index| {
                    let label = joints[index].label.map_or("?".to_string(), |label| label.to_string());
                    format!("{label} ({:.2}, {:.2} m up)", score(index) / moved.len() as f32, joints[index].location.y)
                })
                .collect();
            eprintln!("{}: {}", station.name, names.join(", "));
        }
    }

    /// How each gallery piece moves at a spread of pretensions (`PRETENSIONS`,
    /// fractions of its Medium pretension; `PROBE` names one piece, or a
    /// fabric not yet in the gallery, a quarter taller than Medium; `TALLER`
    /// makes it so much taller than Medium instead of as its station says;
    /// `CUES` a folder of kept cues to play instead of the seeds, `SPEEDS`
    /// the seeds' speeds, cycles a second): how
    /// tall it stands and how many cables are slack at rest, and for each
    /// seed cue, played from rest, the joint swinging farthest and how far
    /// either way about its own middle, how far that middle leans from where
    /// it stood, how quickly the piece sways (cycles a second, as
    /// `Kept::quickness`), how many more cables go slack than at rest, at
    /// most and on average, how often a guy line hangs slack, and how low
    /// it sank. At rest it tells each guy line's strain, per mille.
    #[test]
    #[ignore]
    fn measure_gallery_motion() {
        use crate::fabric::cue::{FORE, TWIST};
        const SETTLING: f32 = 4.0;
        const LISTENING: f32 = 12.0;
        const READING: usize = 50;
        let probe: Option<FabricName> = std::env::var("PROBE").ok().and_then(|name| name.parse().ok());
        let pretensions: Vec<f32> = std::env::var("PRETENSIONS")
            .ok()
            .map(|list| list.split(',').filter_map(|value| value.parse().ok()).collect())
            .unwrap_or(vec![1.0, 0.5, 0.25, 0.1]);
        let speeds: Vec<f32> = std::env::var("SPEEDS")
            .ok()
            .map(|list| list.split(',').filter_map(|value| value.parse().ok()).collect())
            .unwrap_or(vec![0.4, 0.6, 0.8, 1.0, 1.3, 1.6, 2.0]);
        let seeds: Vec<(String, Cue)> = [(SIDE, "side"), (FORE, "fore"), (TWIST, "twist")]
            .into_iter()
            .flat_map(|(way, name)| speeds.iter().map(move |&hertz| (format!("{name} {hertz}Hz"), Cue::rocking(way, hertz))))
            .collect();
        let taller: Option<f32> = std::env::var("TALLER").ok().and_then(|taller| taller.parse().ok());
        let folder = std::env::var("CUES").ok();
        let guys = |fabric: &Fabric| -> Vec<f32> {
            let guys = fabric.intervals.values().filter(|interval| interval.has_role(crate::fabric::interval::Role::GuyLine));
            guys.map(|interval| interval.strain).collect()
        };
        let slack = |fabric: &Fabric| fabric.intervals.values().filter(|interval| interval.role.is_pull_like() && interval.strain <= 0.0).count();
        let unplaced = probe.filter(|probe| STATIONS.iter().all(|station| station.name != *probe));
        let unplaced = unplaced.map(|name| Station { name, at: [0.0, 0.0], facing: 0.0, taller: 1.25, pretension: 1.0, sensors: ["", "", ""] });
        let stations = STATIONS.iter().filter(|station| probe.is_none_or(|probe| probe == station.name)).chain(&unplaced);
        for station in stations {
            for &pretension in &pretensions {
                let (fabric, physics) = medium_made(station, taller.unwrap_or(station.taller), pretension);
                let (standing, at_rest) = (fabric.height().0, slack(&fabric));
                let cables = fabric.intervals.values().filter(|interval| interval.role.is_pull_like()).count();
                let strains: Vec<String> = guys(&fabric).iter().map(|strain| format!("{:.2}", strain * 1000.0)).collect();
                let pushing = |key| fabric.intervals.values().any(|interval| interval.has_role(crate::fabric::interval::Role::Pushing) && interval.touches(key));
                let feet: Vec<Vec3> = fabric.joints.iter().filter(|(key, joint)| joint.location.y < 0.02 && pushing(*key)).map(|(_, joint)| joint.location).collect();
                let between = feet.iter().sum::<Vec3>() / feet.len().max(1) as f32;
                let mass: f32 = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum();
                let gravity = fabric.joints.values().map(|joint| joint.location * joint.accumulated_mass.f32()).sum::<Vec3>() / mass;
                eprintln!(
                    "{} at {pretension}: {standing:.2} m tall, {} feet {:.2} m across, {at_rest} of {cables} cables slack at rest, centre of gravity {:+.0} mm fore of the feet, guy lines {}",
                    station.name,
                    feet.len(),
                    feet.iter().flat_map(|a| feet.iter().map(move |b| a.distance(*b))).fold(0.0, f32::max),
                    (gravity.z - between.z) * 1000.0,
                    strains.join(" ")
                );
                let per_second = crate::Age::iterations_per_second() as usize;
                let seeds = match &folder {
                    Some(folder) => {
                        let file = std::path::PathBuf::from(folder).join(quake::cues_file(&station.name.to_string()).file_name().unwrap());
                        let cues = quake::remembered(&file).into_iter().enumerate();
                        cues.map(|(index, cue)| (format!("{} {}", (b'A' + index as u8) as char, cue.describe()), cue)).collect()
                    }
                    None => seeds.clone(),
                };
                let rows: Vec<String> = std::thread::scope(|scope| {
                    let tries: Vec<_> = seeds
                        .iter()
                        .map(|(name, cue)| {
                            let (mut fabric, physics) = (fabric.clone(), &physics);
                            scope.spawn(move || {
                                let rest: Vec<Vec3> = fabric.joints.values().map(|joint| joint.location).collect();
                                let age = fabric.age;
                                fabric.quake.as_mut().unwrap().play(age, cue.clone());
                                let mut bounds = vec![(Vec3::MAX, Vec3::MIN, Vec3::ZERO); rest.len()];
                                let mut previous: Vec<Option<Vec3>> = vec![None; rest.len()];
                                let (mut moved, mut speed, mut lowest) = (0.0f32, 0.0f32, f32::MAX);
                                let (mut readings, mut slack_sum, mut most_slack, mut guys_slack) = (0, 0, 0, 0);
                                let dt = READING as f32 * crate::Age::iteration_duration().f32();
                                for tick in 0..((SETTLING + LISTENING) * per_second as f32) as usize {
                                    fabric.iterate(physics);
                                    if tick < SETTLING as usize * per_second || tick % READING != 0 {
                                        continue;
                                    }
                                    let deck = fabric.deck().unwrap();
                                    let back = Quat::from_rotation_y(-deck.turn);
                                    for (((bound, last), joint), rest) in bounds.iter_mut().zip(&mut previous).zip(fabric.joints.values()).zip(&rest) {
                                        let away = back * (joint.location - deck.offset) - *rest;
                                        *bound = (bound.0.min(away), bound.1.max(away), bound.2 + away);
                                        moved += away.length_squared();
                                        if let Some(last) = last.replace(away) {
                                            speed += ((away - last) / dt).length_squared();
                                        }
                                    }
                                    let slack = slack(&fabric).saturating_sub(at_rest);
                                    readings += 1;
                                    slack_sum += slack;
                                    guys_slack += guys(&fabric).iter().any(|&strain| strain <= 0.0) as usize;
                                    most_slack = most_slack.max(slack);
                                    lowest = lowest.min(fabric.height().0);
                                }
                                let swing = |&(low, high, _): &(Vec3, Vec3, Vec3)| (high - low).length() / 2.0;
                                let joints: Vec<_> = fabric.joints.values().collect();
                                let most = (0..joints.len()).max_by(|&a, &b| swing(&bounds[a]).total_cmp(&swing(&bounds[b]))).unwrap();
                                let label = joints[most].label.map_or("?".to_string(), |label| label.to_string());
                                format!(
                                    "  {name:28} {label:>5} swing {:4.0} mm  lean {:3.0} mm  {:.2}/s  slack +{most_slack} (mean {:.1})  sank {:.0} mm  guys slack {:.0}%",
                                    swing(&bounds[most]) * 1000.0,
                                    (bounds[most].2 / readings as f32).length() * 1000.0,
                                    (speed / moved.max(1e-12)).sqrt() / std::f32::consts::TAU,
                                    slack_sum as f32 / readings as f32,
                                    (standing - lowest) * 1000.0,
                                    100.0 * guys_slack as f32 / readings as f32
                                )
                            })
                        })
                        .collect();
                    tries.into_iter().map(|try_| try_.join().unwrap()).collect()
                });
                for row in rows {
                    eprintln!("{row}");
                }
            }
        }
    }

    /// Evolve cues for each gallery piece at its size (`PROBE` names one,
    /// all by default; `TRIES` how many each, 300 by default), carrying on
    /// from those in `cues/gallery/`, and keep them there. Built in, they
    /// are what the gallery plays.
    #[test]
    #[ignore]
    fn evolve_gallery_cues() {
        let probe: Option<FabricName> = std::env::var("PROBE").ok().and_then(|name| name.parse().ok());
        let tries: u32 = std::env::var("TRIES").ok().and_then(|tries| tries.parse().ok()).unwrap_or(300);
        let given = |name: &str| std::env::var(name).ok().and_then(|value| value.parse().ok());
        let scoring = Scoring {
            expressive: given("EXPRESSIVE").unwrap_or(SCORING.expressive),
            effort: given("EFFORT").unwrap_or(SCORING.effort),
            haste: given("HASTE").unwrap_or(SCORING.haste),
        };
        let folder = std::env::var("CUES").unwrap_or("cues/gallery".to_string());
        for station in STATIONS.iter().filter(|station| probe.is_none_or(|probe| probe == station.name)) {
            let (name, (mut fabric, physics)) = (station.name, medium(station));
            fabric.quake.as_mut().unwrap().score_by(scoring);
            let memory = std::path::PathBuf::from(&folder).join(quake::cues_file(&name.to_string()).file_name().unwrap());
            let (age, piece) = (fabric.age, (fabric.clone(), physics.clone()));
            fabric.quake.as_mut().unwrap().advance(age, Some(piece), Some(memory));
            let quake = fabric.quake.as_ref().unwrap();
            while quake.tries() < tries {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            for kept in quake.kept() {
                eprintln!(
                    "{name}: {} scored {:.2}, swings {:.0} mm, {:.2}/s, most {}",
                    kept.cue.describe(),
                    kept.score,
                    kept.swing * 1000.0,
                    kept.quickness,
                    kept.most
                );
            }
        }
    }
}

