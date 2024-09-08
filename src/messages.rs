use std::time::SystemTime;

use crate::crucible::CrucibleAction;
use crate::fabric::interval::Role;
use crate::wgpu::Wgpu;

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
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub enum ControlState {
    #[default]
    Viewing,
    ShowingJoint(JointDetails),
    ShowingInterval(IntervalDetails),
}

#[derive(Debug, Clone)]
pub enum LabEvent {
    ContextCreated(Wgpu),
    Crucible(CrucibleAction),
    CalibrateStrain,
    UpdatedLibrary(SystemTime),
    LoadFabric(String),
    CapturePrototype(usize),
}
