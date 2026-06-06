//! Events and commands that flow through the `Radio` (the winit event loop
//! proxy). Anything the simulation, UI, or user sends to be reacted to lives
//! here. UI state types that *react* to events live in `control.rs`.

use crate::control::{
    AppearanceFunction, ControlState, PointerChange, TweakParameter,
};
use crate::build::dsl::FabricPlan;
use crate::fabric::{self, FabricStats, JointKey};
use crate::units::Meters;
use crate::wgpu::Wgpu;
use crate::{Age, RunStyle};
use std::fmt::{Debug, Formatter};

#[derive(Debug, Clone)]
pub enum TesterAction {
    SetTweakParameter(TweakParameter),
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
    ClearSelection,
    AdjustAnimationFrequency(f32),
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
    SetControlState(ControlState),
    SetStageLabel(String),
    ResetView,
    RestartApproach,
    JumpToFabric,
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
    SetKeyboardLegend(String),
    SetTweakParameter(TweakParameter),
    Time {
        frames_per_second: f32,
        age: Age,
        time_scale: f32,
    },
    /// Toggle between perspective and orthogonal projection
    ToggleProjection,
    /// Toggle visibility of attachment points
    ToggleAttachmentPoints,
    /// Show movement analysis overlay (None to hide)
    ShowMovementAnalysis(Option<String>),
    /// Hide left/right text overlays and the bottom keyboard legend (for
    /// `--cycle` Show mode).
    SetShowMode(bool),
}

impl Debug for StateChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            StateChange::SetFabricName(_) => "SetFabricName()",
            StateChange::SetFabricStats(_) => "SetFabricStats()",
            StateChange::SetControlState(_) => "SetControlState()",
            StateChange::SetStageLabel(_) => "SetStageLabel()",
            StateChange::SetAppearanceFunction(_) => "SetColorFunction()",
            StateChange::SetIntervalColor { .. } => "SetIntervalColor()",
            StateChange::ResetView => "ResetView()",
            StateChange::RestartApproach => "RestartApproach()",
            StateChange::JumpToFabric => "JumpToFabric()",
            StateChange::SetAnimating(_) => "SetAnimating()",
            StateChange::SetExperimentTitle { .. } => "SetExperimentTitle()",
            StateChange::SetKeyboardLegend(_) => "SetKeyboardLegend()",
            StateChange::SetTweakParameter(_) => "SetTweakParameter()",
            StateChange::Time { .. } => "Time()",
            StateChange::ToggleProjection => "ToggleProjection",
            StateChange::ToggleAttachmentPoints => "ToggleAttachmentPoints",
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
    AdjustTimeScale(f32),
    SetTimeScale(f32),
    #[cfg(not(target_arch = "wasm32"))]
    ToGpuPhysics,
    #[cfg(not(target_arch = "wasm32"))]
    ToggleAnimationExport,
    #[cfg(not(target_arch = "wasm32"))]
    ExportSnapshot,
}

pub type Radio = winit::event_loop::EventLoopProxy<LabEvent>;

impl LabEvent {
    pub fn send(self, radio: &Radio) {
        radio.send_event(self).expect("Radio working")
    }
}
