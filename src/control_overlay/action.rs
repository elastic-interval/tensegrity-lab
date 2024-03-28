use crate::build::tenscript::fabric_library::FabricLibrary;
use crate::crucible::CrucibleAction;
use crate::scene::SceneAction;
use std::time::SystemTime;
use winit::keyboard::Key;

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
pub enum ControlMessage {
    Reset,
    Keyboard(KeyboardMessage),
    StrainThreshold(StrainThresholdMessage),
    Gravity(GravityMessage),
    Muscle(MuscleMessage),
    Action(Action),
    FreshLibrary(FabricLibrary),
}

#[derive(Clone, Debug)]
pub enum Action {
    Crucible(CrucibleAction),
    Scene(SceneAction),
    CalibrateStrain,
    UpdatedLibrary(SystemTime),
    LoadFabric(String),
}
