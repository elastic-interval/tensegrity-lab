use crate::build::tenscript::brick::Prototype;
use crate::build::tenscript::FabricPlan;
use crate::fabric::interval::Role;
use crate::fabric::FabricStats;
use crate::wgpu::Wgpu;
use crate::Age;
use cgmath::Point3;
use std::collections::HashMap;
use std::time::SystemTime;
use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsFeature {
    Mass,
    Pretense,
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

impl ControlState {
    pub fn send(self, radio: &Radio) {
        LabEvent::UpdateState(StateChange::SetControlState(self)).send(radio);
    }
}

#[derive(Debug, Clone)]
pub enum FailureTesterAction {
    PrevExperiment,
    NextExperiment,
}

#[derive(Debug, Clone)]
pub enum PhysicsTesterAction {
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
    FailureTesterDo(FailureTesterAction),
    ToPhysicsTesting(TestScenario),
    PhysicsTesterDo(PhysicsTesterAction),
    ToEvolving(u64),
}

impl CrucibleAction {
    pub fn send(self, radio: &Radio) {
        LabEvent::Crucible(self).send(&radio);
    }
}

#[derive(Clone, Debug)]
pub enum StateChange {
    SetFabricName(String),
    SetFabricStats(Option<FabricStats>),
    SetControlState(ControlState),
    SetIntervalColor {
        key: (usize, usize),
        color: [f32; 4],
    },
    ResetView,
    SetAnimating(bool),
    SetExperimentTitle {
        title: String,
        fabric_stats: FabricStats,
    },
    SetKeyboardLegend(String),
    SetPhysicsParameter(PhysicsParameter),
    Time { frames_per_second:f32, age: Age },
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
        radio.send_event(self).unwrap()
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
