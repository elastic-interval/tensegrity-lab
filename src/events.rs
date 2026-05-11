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
    TesterDo(TesterAction),
}

impl CrucibleAction {
    pub fn send(self, radio: &Radio) {
        LabEvent::Crucible(self).send(&radio);
    }
}

/// When to take a CSV snapshot during fabric construction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotMoment {
    /// After slackening, before pretensing begins
    Slack,
    /// After pretensing completes
    Pretenst,
    /// After settling on surface
    Settled,
    /// After gravitational pretensing completes
    GravPretenst,
    /// Export at all moments
    All,
}

impl SnapshotMoment {
    /// Get the suffix for this snapshot moment (e.g., "slack", "pretenst", "settled")
    pub fn suffix(&self) -> &'static str {
        match self {
            SnapshotMoment::Slack => "slack",
            SnapshotMoment::Pretenst => "pretenst",
            SnapshotMoment::Settled => "settled",
            SnapshotMoment::GravPretenst => "grav_pretenst",
            SnapshotMoment::All => unreachable!("All should be expanded before calling suffix"),
        }
    }

    /// Check if this moment matches the given moment (handles All)
    pub fn matches(&self, moment: SnapshotMoment) -> bool {
        *self == SnapshotMoment::All || *self == moment
    }

    /// Send this snapshot moment as a LabEvent
    pub fn send(self, radio: &Radio) {
        LabEvent::SnapshotReached(self).send(radio);
    }
}

impl std::str::FromStr for SnapshotMoment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "slack" | "slackened" => Ok(SnapshotMoment::Slack),
            "pretenst" | "pretensed" => Ok(SnapshotMoment::Pretenst),
            "settled" | "settle" => Ok(SnapshotMoment::Settled),
            "grav_pretenst" | "gravpretenst" | "grav" => Ok(SnapshotMoment::GravPretenst),
            "all" => Ok(SnapshotMoment::All),
            _ => Err(format!(
                "Unknown snapshot moment: '{}'. Use: slack, pretenst, settled, grav_pretenst, or all",
                s
            )),
        }
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
    DumpCSV,
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
    /// A snapshot moment has been reached during fabric construction
    SnapshotReached(SnapshotMoment),
}

pub type Radio = winit::event_loop::EventLoopProxy<LabEvent>;

impl LabEvent {
    pub fn send(self, radio: &Radio) {
        radio.send_event(self).expect("Radio working")
    }
}
