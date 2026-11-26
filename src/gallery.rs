//! The gallery (`--gallery`): Medium pieces on their wobble tables, each
//! playing its first kept cue round and round, in a room walked through
//! with the arrow keys. Nothing on the screen but the room and what stands
//! in it; Esc leaves.

use crate::build::dsl::fabric_library::{self, FabricName};
use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
use crate::camera::OPENGL_TO_WGPU_MATRIX;
use crate::fabric::cue::{Cue, SIDE};
use crate::fabric::dimensions::MEDIUM_HEIGHT;
use crate::fabric::physics::presets::VIEWING;
use crate::fabric::physics::Physics;
use crate::fabric::quake::{self, DECK_HEIGHT};
use crate::fabric::Fabric;
use glam::{Mat4, Quat, Vec3, Vec4};
use std::sync::mpsc::{channel, Receiver};

/// Each piece, where its table stands on the floor, x across (to the left
/// seen from the start) and z deep, how far it is turned, degrees, and how
/// much taller than Medium it is made.
const STATIONS: [(FabricName, [f32; 2], f32, f32); 3] = [
    (FabricName::HeadlessHug, [3.5, 4.0], 0.0, 1.25),
    (FabricName::OpenClaw, [0.0, 5.5], 0.0, 1.0),
    (FabricName::Quadruped, [-3.0, 4.0], 0.0, 1.0),
];
/// The room, across and deep, and its walls' height, metres.
pub const WIDTH: f32 = 12.0;
pub const DEPTH: f32 = 16.0;
pub const WALL_HEIGHT: f32 = 3.5;
/// Where the walk starts, looking into the room.
const START: [f32; 2] = [0.0, -1.0];
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
/// For a piece with no kept cues: rocking side to side.
const ROCKING: f32 = 1.0;

/// A piece on its table, where it stands in the room.
pub struct Piece {
    pub fabric: Fabric,
    physics: Physics,
    pub place: Mat4,
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
        for (index, (name, _, _, taller)) in STATIONS.into_iter().enumerate() {
            let send = send.clone();
            let build = move || {
                let (fabric, physics) = playing(name, taller);
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

    /// `seconds` on: pieces built since join the room, the visitor walks,
    /// and every piece moves on `iterations`.
    pub fn advance(&mut self, seconds: f32, iterations: usize) {
        for (index, fabric, physics) in self.built.try_iter() {
            let ([x, z], facing) = (STATIONS[index].1, STATIONS[index].2);
            let place = Mat4::from_rotation_translation(Quat::from_rotation_y(facing.to_radians()), Vec3::new(x, 0.0, z));
            self.pieces.push(Piece { fabric, physics, place });
        }
        self.walker.walk(seconds, self.pieces.iter().map(Piece::footprint));
        for piece in &mut self.pieces {
            for _ in 0..iterations {
                piece.fabric.iterate(&piece.physics);
            }
        }
    }
}

/// The piece `name` built at Medium, `taller` times as tall, on its table,
/// playing its first kept cue, or rocking with none kept.
fn playing(name: FabricName, taller: f32) -> (Fabric, Physics) {
    let plan = fabric_library::get_fabric_plan(name).at_medium_height(MEDIUM_HEIGHT * taller);
    let mut executor = FabricPlanExecutor::new(plan);
    while !executor.is_complete() {
        let _ = executor.iterate();
    }
    let physics = Physics { surface: executor.physics.surface, ..VIEWING };
    let mut fabric = executor.fabric;
    let cue = quake::remembered(&quake::cues_file(&fabric.name)).into_iter().next();
    let age = fabric.age;
    if let Some(table) = &mut fabric.quake {
        table.play(age, cue.unwrap_or_else(|| Cue::rocking(SIDE, ROCKING)));
    }
    (fabric, physics)
}
