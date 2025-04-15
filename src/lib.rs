use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::FabricPlan;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::FabricStats;
use crate::wgpu::Wgpu;
use cgmath::Point3;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::time::SystemTime;
use winit::dpi::PhysicalPosition;

pub mod application;
pub mod build;
pub mod camera;
#[cfg(not(target_arch = "wasm32"))]
pub mod cord_machine;
pub mod crucible;
pub mod fabric;
pub mod keyboard;
pub mod scene;
pub mod test;
pub mod wgpu;

#[derive(Debug, Clone, Copy)]
pub struct Age(f64);

impl Display for Age {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}s", self.0/1_000_000.0)
    }
}

impl Default for Age {
    fn default() -> Self {
        Self(0.0)
    }
}

const TICK_MICROSECONDS: f64 = 200.0;

impl Age {
    pub fn tick(&mut self) -> f32 {
        self.0 += TICK_MICROSECONDS;
        TICK_MICROSECONDS as f32
    }

    pub fn advanced(&self, ticks: usize) -> Self {
        Self(self.0 + TICK_MICROSECONDS * (ticks as f64))
    }

    pub fn brick_baked(&self) -> bool {
        self.0 > 20000.0 * TICK_MICROSECONDS
    }

    pub fn within(&self, limit: &Self) -> bool {
        self.0 < limit.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsFeature {
    Mass,
    Pretenst,
    Stiffness,
    IterationsPerFrame,
    CycleTicks,
    Viscosity,
    Drag,
    StrainLimit,
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicsParameter {
    pub feature: PhysicsFeature,
    pub value: f32,
}

impl PhysicsFeature {
    pub fn parameter(self, value: f32) -> PhysicsParameter {
        PhysicsParameter {
            feature: self,
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TestScenario {
    TensionTest,
    CompressionTest,
    PhysicsTest,
    MachineTest(String),
}

#[derive(Debug, Clone)]
pub enum RunStyle {
    Unknown,
    Fabric {
        fabric_name: String,
        scenario: Option<TestScenario>,
    },
    Prototype(usize),
    Seeded(u64),
}

#[derive(Clone)]
pub enum RenderStyle {
    Normal,
    WithAppearanceFunction(AppearanceFunction),
    WithPullMap(HashMap<(usize, usize), [f32; 4]>),
    WithPushMap(HashMap<(usize, usize), [f32; 4]>),
}

#[derive(Clone, Debug, Copy)]
pub struct IntervalDetails {
    pub near_joint: usize,
    pub far_joint: usize,
    pub length: f32,
    pub strain: f32,
    pub role: Role,
}

#[derive(Clone, Debug, Copy)]
pub struct JointDetails {
    pub index: usize,
    pub location: Point3<f32>,
}

#[derive(Clone, Debug)]
pub enum ControlState {
    Waiting,
    UnderConstruction,
    Baking,
    Viewing,
    Animating,
    ShowingJoint(JointDetails),
    ShowingInterval(IntervalDetails),
    FailureTesting(TestScenario),
    PhysicsTesting(TestScenario),
}

impl ControlState {
    pub fn send(self, radio: &Radio) {
        LabEvent::UpdateState(StateChange::SetControlState(self)).send(radio);
    }
}

#[derive(Debug, Clone)]
pub enum TesterAction {
    PrevExperiment,
    NextExperiment,
    SetPhysicalParameter(PhysicsParameter),
    DumpPhysics,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(FabricPlan),
    ToViewing,
    ToAnimating,
    ToFailureTesting(TestScenario),
    TesterDo(TesterAction),
    ToPhysicsTesting(TestScenario),
    ToEvolving(u64),
}

impl CrucibleAction {
    pub fn send(self, radio: &Radio) {
        LabEvent::Crucible(self).send(&radio);
    }
}

#[derive(Debug, Clone)]
pub struct Appearance {
    pub color: [f32; 4],
    pub radius: f32,
}

impl Appearance {
    pub fn with_color(&self, color: [f32; 4]) -> Self {
        Self {
            color,
            radius: self.radius + 1.0,
        }
    }

    pub fn active(&self) -> Self {
        Self {
            color: [0.9, 0.1, 0.1, 1.0],
            radius: self.radius + 1.0,
        }
    }

    pub fn highlighted(&self) -> Self {
        Self {
            color: [0.0, 1.0, 0.0, 1.0],
            radius: self.radius + 1.0,
        }
    }

    pub fn faded(&self) -> Self {
        Self {
            color: [0.1, 0.1, 0.1, 1.0],
            radius: self.radius,
        }
    }
}

type AppearanceFunction = Rc<dyn Fn(&Interval) -> Option<Appearance>>;

#[derive(Clone)]
pub enum StateChange {
    SetFabricName(String),
    SetFabricStats(Option<FabricStats>),
    SetControlState(ControlState),
    ResetView,
    SetAppearanceFunction(AppearanceFunction),
    SetIntervalColor {
        key: (usize, usize),
        color: [f32; 4],
    },
    SetAnimating(bool),
    SetExperimentTitle {
        title: String,
        fabric_stats: FabricStats,
    },
    SetKeyboardLegend(String),
    SetPhysicsParameter(PhysicsParameter),
    Time {
        frames_per_second: f32,
        age: Age,
    },
}

impl Debug for StateChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            StateChange::SetFabricName(_) => "SetFabricName()",
            StateChange::SetFabricStats(_) => "SetFabricStats()",
            StateChange::SetControlState(_) => "SetControlState()",
            StateChange::SetAppearanceFunction(_) => "SetColorFunction()",
            StateChange::SetIntervalColor { .. } => "SetIntervalColor()",
            StateChange::ResetView => "ResetView()",
            StateChange::SetAnimating(_) => "SetAnimating()",
            StateChange::SetExperimentTitle { .. } => "SetExperimentTitle()",
            StateChange::SetKeyboardLegend(_) => "SetKeyboardLegend()",
            StateChange::SetPhysicsParameter(_) => "SetPhysicsParameter()",
            StateChange::Time { .. } => "Time()",
        };
        write!(f, "StateChange::{name}")
    }
}

impl StateChange {
    pub fn send(self, radio: &Radio) {
        LabEvent::UpdateState(self).send(&radio);
    }
}

#[derive(Debug, Clone)]
pub enum LabEvent {
    Run(RunStyle),
    ContextCreated { wgpu: Wgpu, mobile_device: bool },
    FabricBuilt(FabricStats),
    Crucible(CrucibleAction),
    UpdateState(StateChange),
    UpdatedLibrary(SystemTime),
    PrintCord(f32),
    DumpCSV,
}

pub type Radio = winit::event_loop::EventLoopProxy<LabEvent>;

impl LabEvent {
    pub fn send(self, radio: &Radio) {
        radio.send_event(self).expect("Radio working")
    }
}

#[derive(Debug, Clone)]
pub enum Shot {
    NoPick,
    Joint,
    Interval,
}

#[derive(Debug, Clone)]
pub enum PointerChange {
    NoChange,
    Moved(PhysicalPosition<f64>),
    Zoomed(f32),
    Pressed,
    Released(Shot),
}
