use crate::application::AppStateChange;
use crate::crucible::CrucibleAction;
use crate::fabric::interval::Role;
use crate::fabric::FabricStats;
use crate::wgpu::Wgpu;
use cgmath::Point3;
use std::time::SystemTime;
use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Copy)]
pub enum Scenario {
    Viewing,
    TensionTest,
    CompressionTest,
}

#[derive(Debug, Clone)]
pub enum RunStyle {
    Unknown,
    Fabric {
        fabric_name: String,
        scenario: Scenario,
    },
    Prototype(usize),
    Seeded(u64),
}

#[derive(Debug, Clone)]
pub enum GravityMessage {
    NuanceChanged(f32),
    Reset,
}

#[derive(Debug, Clone)]
pub enum MuscleMessage {
    NuanceChanged(f32),
    Reset,
}

#[derive(Debug, Clone)]
pub enum StrainThresholdMessage {
    SetStrainLimits((f32, f32)),
    NuanceChanged(f32),
    Calibrate,
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
    Testing(Scenario),
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
