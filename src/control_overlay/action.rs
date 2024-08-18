use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::crucible::CrucibleAction;
use std::time::SystemTime;
use winit::keyboard::Key;
use crate::camera::Pick;
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
pub enum KeyboardMessage {
    KeyPressed(Key),
    SubmitAction(Action),
    FreshLibrary(FabricLibrary),
}

#[derive(Debug, Clone)]
pub enum Action {
    ContextCreated(Wgpu),
    Crucible(CrucibleAction),
    Scene(Pick),
    CalibrateStrain,
    UpdatedLibrary(SystemTime),
    LoadFabric(String),
}
