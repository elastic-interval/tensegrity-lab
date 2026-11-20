//! Events and commands that flow through the `Radio` (the winit event loop
//! proxy). Anything the simulation, UI, or user sends to be reacted to lives
//! here. UI state types that *react* to events live in `control.rs`.

use crate::control::{AppearanceFunction, ControlState, PointerChange, TweakFeature};
use crate::build::dsl::FabricPlan;
use crate::fabric::{self, FabricMotion, FabricStats, JointKey};
use crate::units::Meters;
use crate::wgpu::Wgpu;
use crate::{Age, RunStyle};
use std::fmt::{Debug, Formatter};

#[derive(Debug, Clone)]
pub enum TesterAction {
    /// Multiply the mass or rigidity by a factor.
    ScaleTweak(TweakFeature, f32),
    DumpPhysics,
    ToggleMovementSampler,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    StartBaking,
    CycleBrick,
    BuildFabric(FabricPlan),
    /// Load a pre-built algorithmic fabric directly (e.g., tensegrity ball)
    LoadAlgoFabric(fabric::Fabric),
    CentralizeFabric(Option<Meters>),
    /// Step the wind: from the front, the side, behind, then off.
    CycleWind,
    /// Start the ground shaking, or stop it.
    Quake,
    /// Take the standing figure apart strut by strut, or put it together again.
    Assemble,
    ClearSelection,
    ToViewing,
    ToAnimating,
    ToPhysicsTesting,
    ToEvolving(u64),
    ToArticulating(u64),
    TesterDo(TesterAction),
}

impl CrucibleAction {
    pub fn send(self, radio: &Radio) {
        LabEvent::Crucible(self).send(&radio);
    }
}


#[derive(Clone)]
pub enum StateChange {
    SetFabricName(String),
    SetFabricStats(Option<FabricStats>),
    /// The latest reading of how the fabric is moving.
    SetFabricMotion(FabricMotion),
    SetControlState(ControlState),
    SetStageLabel(String),
    /// What the wobble table's motor is doing, shown over the stage label,
    /// or `None` when it stands still.
    SetQuaking(Option<String>),
    ResetView,
    RestartApproach,
    JumpToFabric,
    /// Step a selected interval back to the joint it continues from.
    BackToJoint,
    ToggleColorByRole,
    SetAppearanceFunction(AppearanceFunction),
    SetIntervalColor {
        key: (JointKey, JointKey),
        color: [f32; 4],
    },
    SetAnimating(bool),
    SetExperimentTitle {
        title: String,
        fabric_stats: FabricStats,
    },
    Time {
        frames_per_second: f32,
        age: Age,
        time_scale: f32,
    },
    /// Toggle between perspective and orthogonal projection
    ToggleProjection,
    /// Toggle visibility of attachment points
    ToggleAttachmentPoints,
    /// Toggle the parts view (`wgpu::parts_view`)
    TogglePartsView,
    /// Toggle between the starry sky and a white background
    ToggleBackground,
    /// Show movement analysis overlay (None to hide)
    ShowMovementAnalysis(Option<String>),
    /// Hide left/right text overlays (for `--cycle` Show mode).
    SetShowMode(bool),
}

impl Debug for StateChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            StateChange::SetFabricName(_) => "SetFabricName()",
            StateChange::SetFabricStats(_) => "SetFabricStats()",
            StateChange::SetFabricMotion(_) => "SetFabricMotion()",
            StateChange::SetControlState(_) => "SetControlState()",
            StateChange::SetStageLabel(_) => "SetStageLabel()",
            StateChange::SetQuaking(_) => "SetQuaking()",
            StateChange::SetAppearanceFunction(_) => "SetColorFunction()",
            StateChange::SetIntervalColor { .. } => "SetIntervalColor()",
            StateChange::ResetView => "ResetView()",
            StateChange::RestartApproach => "RestartApproach()",
            StateChange::JumpToFabric => "JumpToFabric()",
            StateChange::BackToJoint => "BackToJoint()",
            StateChange::SetAnimating(_) => "SetAnimating()",
            StateChange::SetExperimentTitle { .. } => "SetExperimentTitle()",
            StateChange::Time { .. } => "Time()",
            StateChange::ToggleProjection => "ToggleProjection",
            StateChange::ToggleAttachmentPoints => "ToggleAttachmentPoints",
            StateChange::TogglePartsView => "TogglePartsView",
            StateChange::ToggleBackground => "ToggleBackground",
            StateChange::ToggleColorByRole => "ToggleColorByRole",
            StateChange::ShowMovementAnalysis(_) => "ShowMovementAnalysis()",
            StateChange::SetShowMode(_) => "SetShowMode()",
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
    ContextCreated {
        wgpu: Wgpu,
        mobile_device: bool,
    },
    FabricBuilt(FabricStats),
    Crucible(CrucibleAction),
    UpdateState(StateChange),
    RebuildFabric,
    NextBrick,
    RequestRedraw,
    PointerChanged(PointerChange),
    SetTimeScale(f32),
    #[cfg(not(target_arch = "wasm32"))]
    ToGpuPhysics,
}

pub type Radio = winit::event_loop::EventLoopProxy<LabEvent>;

impl LabEvent {
    pub fn send(self, radio: &Radio) {
        radio.send_event(self).expect("Radio working")
    }
}
