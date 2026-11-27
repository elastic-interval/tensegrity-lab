#![allow(clippy::result_large_err)]

use crate::build::dsl::animate_phase::{Actuator, AnimatePhase, Waveform};
use crate::build::dsl::build_phase::BuildPhase;
use crate::build::dsl::fabric_library::FabricName;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::joint_names::Link;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::settle_phase::{GuyLine, SettlePhase};
use crate::build::dsl::shape_phase::ShapePhase;
use crate::build::dsl::fabric_plan_executor::{ExecutorStage, FabricPlanExecutor};
use crate::fabric::dimensions::{Size, MEDIUM_HEIGHT, MEDIUM_PRETENSION};
use crate::fabric::quake::{smallest_hexagon, ROOM, SMALLEST};
use crate::units::{Meters, Unit};
use glam::{Quat, Vec2, Vec3};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Mutex, OnceLock};
use crate::fabric::FabricDimensions;

use crate::units::{Hertz, Percent, Seconds};

static BUILT: OnceLock<Mutex<HashMap<FabricName, Built>>> = OnceLock::new();

/// A plan's fabric as built Large, up to its pretensing: how tall, how far
/// it reaches from its middle, and how far across the corners a wobble
/// table it stands on would be, past its spread and its guy lines' anchors.
#[derive(Clone, Copy, Debug)]
struct Built {
    height: f32,
    reach: f32,
    table: f32,
}

#[derive(Debug, Clone)]
pub struct FabricPlan {
    pub name: FabricName,
    pub build_phase: BuildPhase,
    pub shape_phase: ShapePhase,
    pub pretense_phase: PretensePhase,
    pub fall_phase: FallPhase,
    pub settle_phase: Option<SettlePhase>,
    pub animate_phase: Option<AnimatePhase>,
    pub dimensions: FabricDimensions,
}

impl FabricPlan {
    pub fn fall(mut self, seconds: Seconds) -> Self {
        self.fall_phase = FallPhase { seconds };
        self
    }

    pub fn settle(mut self, seconds: Seconds) -> Self {
        self.settle_phase = Some(SettlePhase { seconds, guy_lines: Vec::new(), cables: Vec::new() });
        self
    }

    /// Guy lines are created on landing, so they need a settle phase.
    pub fn guy_lines<const N: usize>(mut self, guy_lines: [GuyLine; N]) -> Self {
        self.settle_phase
            .as_mut()
            .expect("guy_lines requires settle()")
            .guy_lines = guy_lines.to_vec();
        self
    }

    /// Cables added halfway through the settle, along each link with its
    /// images, each cut to `length` of its span then, so they need a settle
    /// phase.
    pub fn cables<const N: usize>(mut self, length: Percent, links: [Link; N]) -> Self {
        self.settle_phase
            .as_mut()
            .expect("cables requires settle()")
            .cables
            .extend(links.map(|link| (link, length)));
        self
    }

    /// This plan made at `size`, knowing how far it reaches once built, for
    /// the camera to keep back from the start. Medium is built small enough to stand about
    /// `MEDIUM_HEIGHT` tall, its lengths in metres shrunk with it, and made
    /// in its own materials from pretensing on (`FabricDimensions::medium`);
    /// it has no node plates, so no bridles, and its hinges are made by
    /// hand. Small is shown from the Large fabric.
    pub fn at_size(self, size: Size) -> Self {
        self.at(size, MEDIUM_HEIGHT)
    }

    /// This plan made Medium, but `height` tall rather than `MEDIUM_HEIGHT`.
    pub fn at_medium_height(self, height: Meters) -> Self {
        self.at(Size::Medium, height)
    }

    fn at(mut self, size: Size, medium_height: Meters) -> Self {
        let Built { height, reach, table } = self.built();
        self.dimensions.reach = Some(Meters(reach));
        if size == Size::Medium {
            let shrink = medium_height.f32() / height;
            self.dimensions.reach = Some(Meters(reach * shrink));
            self.dimensions = self.dimensions.drawn_medium();
            self.dimensions.scale = self.dimensions.scale * shrink;
            let table = match self.dimensions.table_scales {
                Some(scales) => self.dimensions.scale.f32() * scales,
                None => (table * shrink).max(SMALLEST),
            };
            self.dimensions.table = Some(Meters(table));
            self.dimensions.altitude = self.dimensions.altitude * shrink;
            // Things fall faster the smaller they are, in time going as the
            // square root of their size: in the time a Large one falls, a
            // Medium one would topple before its guy lines are on.
            self.fall_phase.seconds = Seconds(self.fall_phase.seconds.0 * shrink.sqrt());
            if let Some(settle) = &mut self.settle_phase {
                for guy_line in &mut settle.guy_lines {
                    let (outward, along) = guy_line.ground_offset;
                    guy_line.ground_offset = (outward * shrink, along * shrink);
                }
            }
            self.pretense_phase.bridles.clear();
            self.pretense_phase.pretenst = Percent(self.pretense_phase.pretenst.0 * MEDIUM_PRETENSION);
        }
        self.dimensions.size = size;
        self
    }

    /// The fabric as it is built, Large, up to its pretensing: unseen,
    /// once for each fabric.
    fn built(&self) -> Built {
        if let Some(found) = self.known() {
            return found;
        }
        let mut executor = self.unseen();
        while !self.learn(&mut executor) {
            let _ = executor.iterate();
        }
        self.known().unwrap()
    }

    fn known(&self) -> Option<Built> {
        BUILT.get_or_init(Default::default).lock().unwrap().get(&self.name).copied()
    }

    /// Whether it is already known how this plan builds Large; if not,
    /// `unseen` builds it, a little at a time if need be, until `learn`
    /// says it has learned.
    pub fn is_known(&self) -> bool {
        self.known().is_some()
    }

    pub fn unseen(&self) -> FabricPlanExecutor {
        FabricPlanExecutor::new(self.clone())
    }

    pub fn learn(&self, executor: &mut FabricPlanExecutor) -> bool {
        if matches!(executor.stage(), ExecutorStage::Building) {
            return false;
        }
        let fabric = &mut executor.fabric;
        fabric.update_bounding_radius();
        let middle = fabric.joints.values().fold(Vec3::ZERO, |sum, joint| sum + joint.location) / fabric.joints.len() as f32;
        let spread = fabric.joints.values().fold(Vec2::ZERO, |most, joint| {
            most.max(Vec2::new((joint.location.x - middle.x).abs(), (joint.location.z - middle.z).abs()))
        });
        // Its feet to be: its lowest joints, turned as it will stand on the
        // table. Where they land, the deck is fitted again (`Quake::mount`).
        let lowest = fabric.joints.values().map(|joint| joint.location.y).fold(f32::MAX, f32::min);
        let turn = Quat::from_rotation_y(-self.dimensions.table_turn.0.to_radians());
        let feet = fabric
            .joints
            .values()
            .filter(|joint| joint.location.y < lowest + 0.02 * fabric.height().f32())
            .map(|joint| turn * joint.location);
        let feet = smallest_hexagon(feet);
        let guys = self.settle_phase.iter().flat_map(|settle| &settle.guy_lines);
        let anchors = guys.fold(0.0f32, |most, guy| {
            let (outward, along) = guy.ground_offset;
            most.max((spread.x + outward.f32().abs()).hypot(along.f32().abs()))
        });
        let found = Built {
            height: fabric.height().f32(),
            reach: fabric.bounding_radius(),
            table: ROOM * feet.max(anchors / (PI / 6.0).cos()),
        };
        BUILT.get_or_init(Default::default).lock().unwrap().insert(self.name, found);
        true
    }

    pub fn animate(self) -> AnimateBuilder {
        AnimateBuilder {
            plan: self,
            phase: AnimatePhase::new(),
        }
    }
}

/// Builder for configuring animation with chained methods
pub struct AnimateBuilder {
    plan: FabricPlan,
    phase: AnimatePhase,
}

impl AnimateBuilder {
    pub fn actuator_frequency(mut self, frequency: Hertz) -> Self {
        self.phase.period = frequency.to_seconds();
        self
    }

    pub fn amplitude(mut self, amplitude: Percent) -> Self {
        self.phase.amplitude = amplitude;
        self
    }

    pub fn stiffness(mut self, stiffness: Percent) -> Self {
        self.phase.stiffness = stiffness;
        self
    }

    pub fn sine(mut self) -> Self {
        self.phase.waveform = Waveform::Sine;
        self
    }

    pub fn pulse(mut self, duty_cycle: Percent) -> Self {
        self.phase.waveform = Waveform::Pulse { duty_cycle };
        self
    }

    /// Terminal method: add actuators and return the completed FabricPlan
    pub fn actuators<const N: usize>(mut self, actuators: [Actuator; N]) -> FabricPlan {
        self.phase.actuators = actuators.to_vec();
        self.plan.animate_phase = Some(self.phase);
        self.plan
    }
}

