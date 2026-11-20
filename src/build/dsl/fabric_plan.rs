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
use crate::fabric::quake::DECK_MARGIN;
use crate::units::{Meters, Unit};
use glam::{Vec2, Vec3};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use crate::fabric::FabricDimensions;

use crate::units::{Hertz, Percent, Seconds};

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

    /// This plan made at `size`. Medium is built small enough to stand about
    /// `MEDIUM_HEIGHT` tall, its lengths in metres shrunk with it, and made
    /// in its own materials from pretensing on (`FabricDimensions::medium`);
    /// it has no node plates, so no bridles, and its hinges are made by
    /// hand. Small is shown from the Large fabric.
    pub fn at_size(mut self, size: Size) -> Self {
        if size == Size::Medium {
            let (shrink, table) = self.medium_shrink();
            self.dimensions.table = Some(Meters(table));
            self.dimensions.scale = self.dimensions.scale * shrink;
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

    /// How much smaller than Large to build it, for it to stand about
    /// `MEDIUM_HEIGHT` tall, and how far across its table's corners: from
    /// building it Large, unseen, up to its pretensing, once for each
    /// fabric. The table reaches past the piece's spread and its guy lines'
    /// anchors.
    fn medium_shrink(&self) -> (f32, f32) {
        static SHRINKS: OnceLock<Mutex<HashMap<FabricName, (f32, f32)>>> = OnceLock::new();
        let shrinks = SHRINKS.get_or_init(Default::default);
        if let Some(&found) = shrinks.lock().unwrap().get(&self.name) {
            return found;
        }
        let mut executor = FabricPlanExecutor::new(self.clone());
        while matches!(executor.stage(), ExecutorStage::Building) {
            let _ = executor.iterate();
        }
        let fabric = &executor.fabric;
        let shrink = MEDIUM_HEIGHT.f32() / fabric.height().f32();
        let middle = fabric.joints.values().fold(Vec3::ZERO, |sum, joint| sum + joint.location) / fabric.joints.len() as f32;
        let spread = fabric.joints.values().fold(Vec2::ZERO, |most, joint| {
            most.max(Vec2::new((joint.location.x - middle.x).abs(), (joint.location.z - middle.z).abs()))
        });
        let guys = self.settle_phase.iter().flat_map(|settle| &settle.guy_lines);
        let anchors = guys.fold(0.0f32, |most, guy| {
            let (outward, along) = guy.ground_offset;
            most.max((spread.x + outward.f32().abs()).hypot(along.f32().abs()))
        });
        let table = (spread.length().max(anchors) * shrink + DECK_MARGIN) * 1.05;
        shrinks.lock().unwrap().insert(self.name, (shrink, table));
        (shrink, table)
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

