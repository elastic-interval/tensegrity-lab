use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::FabricPlan;
use crate::fabric::interval::Role;
use crate::fabric::FabricStats;
use crate::wgpu::Wgpu;
use cgmath::Point3;
use std::collections::HashMap;
use std::time::SystemTime;
use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy)]
pub enum TestScenario {
    TensionTest,
    CompressionTest,
    PhysicsTest,
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

#[derive(Clone, Debug)]
pub enum IntervalFilter {
    ShowAll,
    ShowPush,
    ShowPull,
}

#[derive(Clone, Debug)]
pub enum RenderStyle {
    Normal,
    WithColoring {
        color_map: HashMap<(usize, usize), [f32; 4]>,
        filter: IntervalFilter,
    },
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

#[derive(Debug, Clone)]
pub enum FailureTesterAction {
    PrevExperiment,
    NextExperiment,
}

#[derive(Debug, Clone)]
pub enum PhysicsTesterAction {
    PrevExperiment,
    NextExperiment,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(FabricPlan),
    ToFailureTesting(TestScenario),
    ToPhysicsTesting(TestScenario),
    FailureTesterDo(FailureTesterAction),
    PhysicsTesterDo(PhysicsTesterAction),
    StartEvolving(u64),
    AdjustSpeed(f32),
    ViewingToAnimating,
    ToViewing,
}

#[derive(Clone, Debug)]
pub enum AppStateChange {
    SetIntervalColor {
        key: (usize, usize),
        color: [f32; 4],
    },
    SetControlState(ControlState),
    SetFabricName(String),
    SetFabricStats(Option<FabricStats>),
    SetAnimating(bool),
    SetExperimentTitle {
        title: String,
        fabric_stats: FabricStats,
    },
    SetKeyboardLegend(String),
    SetIterationsPerFrame(usize),
}

#[derive(Debug, Clone)]
pub enum LabEvent {
    ContextCreated { wgpu: Wgpu, mobile_device: bool },
    Run(RunStyle),
    Crucible(CrucibleAction),
    FabricBuilt(FabricStats),
    AppStateChanged(AppStateChange),
    DumpCSV,
    UpdatedLibrary(SystemTime),
}

pub type Broadcast = winit::event_loop::EventLoopProxy<LabEvent>;

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
