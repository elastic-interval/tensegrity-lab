//! The gallery (`--gallery`): Medium pieces on their wobble tables, each
//! playing the cues evolved for it at its size in turn (`cues/gallery/`,
//! by the ignored `evolve_gallery_cues`), in a room walked through with
//! the arrow keys. Nothing on the screen but the room and what stands in
//! it; Esc leaves.

use crate::build::dsl::fabric_library::{self, FabricName};
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::camera::OPENGL_TO_WGPU_MATRIX;
use crate::fabric::cue::{Cue, SIDE};
use crate::fabric::dimensions::MEDIUM_HEIGHT;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::Physics;
use crate::fabric::quake::{self, DECK_HEIGHT};
use crate::fabric::Fabric;
use crate::units::Percent;
use glam::{Mat4, Quat, Vec3, Vec4};
use std::sync::mpsc::{channel, Receiver};

/// A piece in the gallery: where its table stands on the floor, x across
/// (to the left seen from the start) and z deep; how far it is turned,
/// degrees; how much taller than Medium it is made; and how much of its
/// Medium pretension it keeps, less to sway more slowly and further.
struct Station {
    name: FabricName,
    at: [f32; 2],
    facing: f32,
    taller: f32,
    pretension: f32,
}

const STATIONS: [Station; 3] = [
    Station { name: FabricName::OpenClaw, at: [4.5, 4.0], facing: 0.0, taller: 1.25, pretension: 0.1 },
    Station { name: FabricName::HeadlessHug, at: [0.0, 6.0], facing: 180.0, taller: 1.25, pretension: 1.0 },
    Station { name: FabricName::Quadruped, at: [-4.5, 4.0], facing: 0.0, taller: 1.0, pretension: 1.0 },
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
/// How far down the visitor looks, and how wide they see, radians.
const LOOK_DOWN: f32 = 0.15;
const VIEW: f32 = 1.0;
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
    pub views: [Mat4; 3],
    pub places: [Vec4; 3],
    pub aims: [Vec4; 3],
}

/// A piece on its table, where it stands in the room.
pub struct Piece {
    pub fabric: Fabric,
    physics: Physics,
    pub place: Mat4,
    /// Its cues, the one playing, and how long it has played.
    cues: Vec<Cue>,
    playing: usize,
    played: f32,
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

pub struct Walker {
    /// Where the eyes are, the floor at zero, and which way they look,
    /// zero along the room's depth.
    at: Vec3,
    heading: f32,
    pub arrows: Arrows,
}

impl Walker {
    fn new() -> Walker {
        Walker { at: Vec3::new(START[0], EYE, START[1]), heading: 0.0, arrows: Arrows::default() }
    }

    fn ahead(&self) -> Vec3 {
        Vec3::new(self.heading.sin(), 0.0, self.heading.cos())
    }

    /// Walk and turn for `seconds` as the arrows say, kept off the walls
    /// and the tables.
    fn walk(&mut self, seconds: f32, tables: impl Iterator<Item = (Vec3, f32)>) {
        let arrow = |held: bool| if held { 1.0 } else { 0.0 };
        self.heading += (arrow(self.arrows.left) - arrow(self.arrows.right)) * TURN * seconds;
        let step = (arrow(self.arrows.forward) - arrow(self.arrows.back)) * WALK * seconds;
        let mut at = self.at + self.ahead() * step;
        for (middle, radius) in tables {
            let away = Vec3::new(at.x - middle.x, 0.0, at.z - middle.z);
            let clear = radius + KEEP_OFF;
            if away.length() < clear {
                at += away.try_normalize().unwrap_or(Vec3::X) * (clear - away.length());
            }
        }
        let (x, z) = (WIDTH / 2.0 - KEEP_OFF, DEPTH / 2.0 - KEEP_OFF);
        (at.x, at.z) = (at.x.clamp(-x, x), at.z.clamp(-z, z));
        self.at = at;
    }

    /// Where the eyes are in the pieces' frame, whose decks are at zero.
    pub fn eye(&self) -> Vec3 {
        self.at - Vec3::Y * DECK_HEIGHT
    }

    pub fn mvp(&self, width: f32, height: f32) -> Mat4 {
        let look = self.ahead() - Vec3::Y * LOOK_DOWN;
        let view = Mat4::look_at_rh(self.eye(), self.eye() + look, Vec3::Y);
        let projection = Mat4::perspective_rh(VIEW, width / height, 0.05, 100.0);
        OPENGL_TO_WGPU_MATRIX * projection * view
    }
}

pub struct Gallery {
    built: Receiver<(usize, Fabric, Physics)>,
    pub pieces: Vec<Piece>,
    pub walker: Walker,
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
        Gallery { built, pieces: Vec::new(), walker: Walker::new() }
    }

    /// The light in each table, where it is and how bright. Before every
    /// piece has joined the room the missing ones are as good as dark, so
    /// the room is dark from the start.
    pub fn lights(&self) -> [Vec4; 3] {
        let mut lights = [Vec4::new(0.0, 0.0, 0.0, f32::MIN_POSITIVE); 3];
        for (light, piece) in lights.iter_mut().zip(&self.pieces) {
            let middle = piece.fabric.deck().map_or(Vec3::ZERO, |deck| deck.offset);
            *light = piece.place.transform_point3(middle - Vec3::Y * LIGHT_DEPTH).extend(LIGHT);
        }
        lights
    }

    /// The spots, going with their decks; none dark until its piece is in
    /// the room.
    pub fn spots(&self) -> Spots {
        let mut spots = Spots { views: [Mat4::IDENTITY; 3], places: [Vec4::ZERO; 3], aims: [Vec4::Z; 3] };
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
            let Station { name, at: [x, z], facing, .. } = STATIONS[index];
            let place = Mat4::from_rotation_translation(Quat::from_rotation_y(facing.to_radians()), Vec3::new(x, 0.0, z));
            let mut cues = cues(name);
            if cues.is_empty() {
                cues.push(Cue::rocking(SIDE, ROCKING));
            }
            let mut piece = Piece { fabric, physics, place, cues, playing: 0, played: PLAYING };
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
            }
        }
    }
}

/// The piece at `station` built at Medium, as tall and as tight as it
/// says, on its table.
fn medium(station: &Station) -> (Fabric, Physics) {
    let mut plan = fabric_library::get_fabric_plan(station.name).at_medium_height(MEDIUM_HEIGHT * station.taller);
    plan.pretense_phase.pretenst = Percent(plan.pretense_phase.pretenst.0 * station.pretension);
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
        _ => "",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evolve cues for each gallery piece at its size (`PROBE` names one,
    /// all by default; `TRIES` how many each, 300 by default), carrying on
    /// from those in `cues/gallery/`, and keep them there. Built in, they
    /// are what the gallery plays.
    #[test]
    #[ignore]
    fn evolve_gallery_cues() {
        let probe: Option<FabricName> = std::env::var("PROBE").ok().and_then(|name| name.parse().ok());
        let tries: u32 = std::env::var("TRIES").ok().and_then(|tries| tries.parse().ok()).unwrap_or(300);
        for station in STATIONS.iter().filter(|station| probe.is_none_or(|probe| probe == station.name)) {
            let (name, (mut fabric, physics)) = (station.name, medium(station));
            let memory = std::path::PathBuf::from("cues/gallery").join(quake::cues_file(&name.to_string()).file_name().unwrap());
            let (age, piece) = (fabric.age, (fabric.clone(), physics.clone()));
            fabric.quake.as_mut().unwrap().advance(age, Some(piece), Some(memory));
            let quake = fabric.quake.as_ref().unwrap();
            while quake.tries() < tries {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            for kept in quake.kept() {
                eprintln!("{name}: {} scored {:.2}, swings {:.0} mm, most {}", kept.cue.describe(), kept.score, kept.swing * 1000.0, kept.most);
            }
        }
    }
}
