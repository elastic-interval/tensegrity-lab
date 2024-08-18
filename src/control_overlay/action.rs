use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::crucible::CrucibleAction;
use std::time::SystemTime;
use winit::keyboard::Key;
use crate::camera::Pick;
use crate::wgpu_context::WgpuContext;

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

pub enum KeyboardMessage {
    KeyPressed(Key),
    SubmitAction(Action),
    FreshLibrary(FabricLibrary),
}

pub enum ControlMessage {
    Reset,
    Keyboard(KeyboardMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    Muscle(MuscleMessage),
    Action(Action),
    FreshLibrary(FabricLibrary),
}

pub enum Action {
    ContextCreated(WgpuContext),
    Crucible(CrucibleAction),
    Scene(Pick),
    CalibrateStrain,
    UpdatedLibrary(SystemTime),
    LoadFabric(String),
}
