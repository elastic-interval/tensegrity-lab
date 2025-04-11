use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::FabricPlan;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::FabricStats;
use crate::wgpu::Wgpu;
use crate::Age;
use cgmath::Point3;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::time::SystemTime;
use winit::dpi::PhysicalPosition;

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
            StateChange::SetControlState(_) => "SetcontrolState()",
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
