use std::time::SystemTime;

use crate::camera::Pick;
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

#[derive(Debug, Clone)]
pub enum SceneAction {
    ForcePick(Pick),
    EscapeHappens,
}

#[derive(Debug, Clone)]
pub enum LabEvent {
    ContextCreated(Wgpu),
    Crucible(CrucibleAction),
    Scene(SceneAction),
    CalibrateStrain,
    UpdatedLibrary(SystemTime),
    LoadFabric(String),
}

#[derive(Clone, Debug, Copy)]
pub struct IntervalDetails {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub length: f32,
    pub role: Role,
}

#[derive(Clone, Debug, Default)]
pub enum ControlState {
    #[default]
    Choosing,
    Viewing,
    ShowingJoint(usize),
    ShowingInterval(IntervalDetails),
    SettingLength(IntervalDetails),
}
