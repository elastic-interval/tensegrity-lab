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
use crate::fabric::variant::{Table, Variant};
use crate::fabric::quake::{smallest_hexagon, ROOM, SMALLEST};
use crate::units::{Meters, Unit};
use glam::{Quat, Vec2, Vec3};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::{Mutex, OnceLock};
use crate::fabric::{Fabric, FabricDimensions};

use crate::units::{Hertz, Percent, Seconds};

static BUILT: OnceLock<Mutex<HashMap<FabricName, Built>>> = OnceLock::new();

/// Each plan's `Built`, stored so that no plan need be built unseen before
/// it is made smaller: a line each, its name, then its height, reach and
/// table as the bits of their floats, exactly. Written by the ignored
/// `store_builds`; `stored_builds_are_current` says when it is stale.
const BUILDS: &str = include_str!("builds.txt");

fn stored(name: FabricName) -> Option<Built> {
    BUILDS.lines().find_map(|line| {
        let (named, bits) = line.split_once('|')?;
        (named.trim() == name.to_string()).then_some(())?;
        let bits: Vec<f32> = bits.split_whitespace().take(3).map(|word| u32::from_str_radix(word, 16).ok().map(f32::from_bits)).collect::<Option<_>>()?;
        Some(Built { height: bits[0], reach: bits[1], table: bits[2] })
    })
}

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

    /// This plan made as `variant` says, knowing how far it reaches once
    /// built, for the camera to keep back from the start. One made smaller
    /// is built smaller from the start, its lengths in metres shrunk with
    /// it, and made of its own materials from pretensing on (the executor
    /// sees to that).
    pub fn at(mut self, variant: &Variant) -> Self {
        let Built { height, reach, table } = self.built();
        self.dimensions.reach = Some(Meters(reach));
        if let Some(made_height) = variant.height {
            let shrink = made_height.f32() / height;
            self.dimensions.reach = Some(Meters(reach * shrink));
            if !variant.connectors {
                self.dimensions = self.dimensions.without_connectors();
            }
            if variant.materials.is_some() {
                self.dimensions = self.dimensions.drawn_hand_built();
            }
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
        }
        if !variant.connectors {
            self.pretense_phase.bridles.clear();
        }
        if variant.pretension != 1.0 {
            self.pretense_phase.pretenst = Percent(self.pretense_phase.pretenst.0 * variant.pretension);
        }
        match variant.table {
            Table::None => self.dimensions.table = None,
            Table::Planned => {}
            Table::Given(radius) => {
                self.dimensions.table = Some(radius);
                self.dimensions.table_scales = Some(radius.f32() / self.dimensions.scale.f32());
            }
        }
        if let Some(altitude) = variant.seed_altitude {
            self.dimensions.altitude = altitude;
            self.build_phase.seed_altitude = altitude.f32() / self.dimensions.scale.f32();
        }
        self.build_phase.seed_shift = variant.seed_shift;
        self.dimensions.variant = *variant;
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

    /// Known already, or stored (`BUILDS`), so most plans needn't be
    /// built unseen at all.
    fn known(&self) -> Option<Built> {
        let mut built = BUILT.get_or_init(Default::default).lock().unwrap();
        if let Some(found) = built.get(&self.name) {
            return Some(*found);
        }
        let stored = stored(self.name)?;
        built.insert(self.name, stored);
        Some(stored)
    }

    /// Built unseen now, whatever is known or stored.
    fn measured(&self) -> Built {
        let mut executor = self.unseen();
        while matches!(executor.stage(), ExecutorStage::Building) {
            let _ = executor.iterate();
        }
        self.measure(&mut executor.fabric)
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
        let found = self.measure(&mut executor.fabric);
        BUILT.get_or_init(Default::default).lock().unwrap().insert(self.name, found);
        true
    }

    fn measure(&self, fabric: &mut Fabric) -> Built {
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
        Built {
            height: fabric.height().f32(),
            reach: fabric.bounding_radius(),
            table: ROOM * feet.max(anchors / (PI / 6.0).cos()),
        }
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


#[cfg(test)]
mod builds {
    use super::*;
    use crate::build::dsl::fabric_library;
    use strum::IntoEnumIterator;

    fn line(name: FabricName, built: Built) -> String {
        let Built { height, reach, table } = built;
        format!(
            "{name} | {:08x} {:08x} {:08x}  height {height:.3} m, reach {reach:.3} m, table {table:.3} m",
            height.to_bits(),
            reach.to_bits(),
            table.to_bits()
        )
    }

    /// Every plan's stored `Built` is what building it unseen gives now.
    #[test]
    fn stored_builds_are_current() {
        for name in FabricName::iter() {
            let measured = fabric_library::get_fabric_plan(name).measured();
            assert_eq!(
                stored(name).map(|stored| line(name, stored)),
                Some(line(name, measured)),
                "{name}'s stored build is stale: run store_builds"
            );
        }
    }

    /// Store every plan's `Built` in `builds.txt`.
    #[test]
    #[ignore]
    fn store_builds() {
        let lines: Vec<String> = FabricName::iter().map(|name| line(name, fabric_library::get_fabric_plan(name).measured())).collect();
        std::fs::write("src/build/dsl/builds.txt", lines.join("\n") + "\n").unwrap();
    }
}
