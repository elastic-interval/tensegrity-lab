#[allow(dead_code)]
use crate::build::dsl::brick::Prototype;
use crate::build::dsl::FabricPlan;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::{FabricStats, UniqueId};
use crate::wgpu::Wgpu;
use cgmath::{Point3, Vector3};
use instant::Instant;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Mul;
use std::rc::Rc;
use winit::dpi::PhysicalPosition;

pub mod application;
pub mod build;
pub mod camera;
#[cfg(not(target_arch = "wasm32"))]
pub mod cord_machine;
pub mod crucible;
pub mod crucible_context;
pub mod fabric;
pub mod keyboard;
pub mod pointer;
pub mod scene;
pub mod units;

pub mod wgpu;

use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Age(Duration);

impl Display for Age {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let total_secs = self.0.as_secs_f64();

        if total_secs < 60.0 {
            // Less than a minute: show as whole seconds
            write!(f, "{}s", total_secs as u64)
        } else {
            // 60 seconds or more: show as minutes:seconds
            let minutes = (total_secs / 60.0).floor() as u64;
            let seconds = (total_secs % 60.0) as u64;
            write!(f, "{}:{:02}", minutes, seconds)
        }
    }
}

/// Time values for each physics iteration, with pre-computed derivatives
#[derive(Debug, Clone, Copy)]
pub struct IterationDuration {
    pub duration: Duration,
    pub secs: f32,
}

impl IterationDuration {
    const fn new(microseconds: u64) -> Self {
        let duration = Duration::from_micros(microseconds);
        let secs = microseconds as f32 / 1_000_000.0;

        Self { duration, secs }
    }
}

/// Duration of each physics iteration tick
pub const ITERATION_DURATION: IterationDuration = IterationDuration::new(50);

impl Default for Age {
    fn default() -> Self {
        Self(Duration::ZERO)
    }
}

impl Age {
    pub fn tick(&mut self) -> Duration {
        self.0 += ITERATION_DURATION.duration;
        ITERATION_DURATION.duration
    }

    pub fn tick_scaled(&mut self, dt_scale: f32) -> Duration {
        let scaled_duration = ITERATION_DURATION.duration.mul_f32(dt_scale);
        self.0 += scaled_duration;
        scaled_duration
    }

    pub fn advanced(&self, ticks: usize) -> Self {
        Self(self.0 + ITERATION_DURATION.duration * ticks as u32)
    }

    pub fn brick_baked(&self) -> bool {
        self.0 > ITERATION_DURATION.duration * 20000
    }

    pub fn within(&self, limit: &Self) -> bool {
        self.0 < limit.0
    }

    pub fn as_duration(&self) -> Duration {
        self.0
    }
}

// Actual physics parameters that affect simulation behavior
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsFeature {
    Pretenst,
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

// Tweak parameters that scale/modify the physics (user-controlled view on physics)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TweakFeature {
    MassScale,
    RigidityScale,
    TimeScale,
}

#[derive(Debug, Clone, Copy)]
pub struct TweakParameter {
    pub feature: TweakFeature,
    pub value: f32,
}

impl TweakFeature {
    pub fn parameter(self, value: f32) -> TweakParameter {
        TweakParameter {
            feature: self,
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TestScenario {
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

#[derive(Clone)]
pub enum RenderStyle {
    Normal {
        show_attachment_points: bool,
    },
    ColorByRole {
        show_attachment_points: bool,
    },
    WithAppearanceFunction {
        function: AppearanceFunction,
        show_attachment_points: bool,
    },
    WithPullMap {
        map: HashMap<(usize, usize), [f32; 4]>,
        show_attachment_points: bool,
    },
    WithPushMap {
        map: HashMap<(usize, usize), [f32; 4]>,
        show_attachment_points: bool,
    },
}

impl RenderStyle {
    pub fn show_attachment_points(&self) -> bool {
        match self {
            RenderStyle::Normal {
                show_attachment_points,
            }
            | RenderStyle::ColorByRole {
                show_attachment_points,
            }
            | RenderStyle::WithAppearanceFunction {
                show_attachment_points,
                ..
            }
            | RenderStyle::WithPullMap {
                show_attachment_points,
                ..
            }
            | RenderStyle::WithPushMap {
                show_attachment_points,
                ..
            } => *show_attachment_points,
        }
    }

    pub fn toggle_attachment_points(&mut self) {
        match self {
            RenderStyle::Normal {
                show_attachment_points,
            }
            | RenderStyle::ColorByRole {
                show_attachment_points,
            }
            | RenderStyle::WithAppearanceFunction {
                show_attachment_points,
                ..
            }
            | RenderStyle::WithPullMap {
                show_attachment_points,
                ..
            }
            | RenderStyle::WithPushMap {
                show_attachment_points,
                ..
            } => *show_attachment_points = !*show_attachment_points,
        }

        // Update the thread-local state for joint text formatting
        SHOW_ATTACHMENT_POINTS.with(|cell| {
            *cell.borrow_mut() = self.show_attachment_points();
        });
    }
}

#[derive(Clone, Debug, Copy)]
pub struct IntervalDetails {
    pub id: UniqueId,
    pub near_joint: usize,
    pub near_slot: Option<usize>,
    pub far_slot: Option<usize>,
    pub far_joint: usize,
    pub length: f32,
    pub strain: f32,
    pub distance: f32,
    pub role: Role,
    pub scale: f32,
    pub selected_push: Option<UniqueId>,
}

impl Display for IntervalDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let role_text = match self.role {
            Role::Pushing => "Strut",
            Role::Pulling => "Cable",
            Role::Springy => "Spring",
            Role::Circumference => "Circumference",
            Role::BowTie => "BowTie",
            Role::FaceRadial => "FaceRadial",
            Role::Support => "Support",
            Role::GuyLine => "GuyLine",
            Role::PrismPull => "PrismPull",
        };

        // Get the current attachment point visibility from thread-local storage
        let show_attachment_points = SHOW_ATTACHMENT_POINTS.with(|cell| *cell.borrow());

        write!(
            f,
            "{} {}-{}\nLength: {:.1} mm\nStrain: {:.6}%\nDistance: {:.1} mm\nRight-click to jump",
            role_text,
            self.near_joint_text(show_attachment_points),
            self.far_joint_text(show_attachment_points),
            self.length_mm(),
            self.strain_percent(),
            self.distance_mm()
        )
    }
}

impl IntervalDetails {
    pub fn length_mm(&self) -> f32 {
        self.length * self.scale
    }

    pub fn strain_percent(&self) -> f32 {
        self.strain * 100.0
    }

    pub fn distance_mm(&self) -> f32 {
        self.distance * self.scale
    }

    /// Format a joint index as a string, optionally with a slot number
    /// If show_attachment_points is false, the slot number will be hidden
    pub fn format_joint(&self, is_near: bool, show_attachment_points: bool) -> String {
        let (joint_index, slot) = if is_near {
            (self.near_joint, self.near_slot)
        } else {
            (self.far_joint, self.far_slot)
        };

        // Only show slot numbers if attachment points are visible
        if show_attachment_points {
            match slot {
                Some(slot_idx) => format!("J{}:{}", joint_index, slot_idx),
                None => format!("J{}", joint_index),
            }
        } else {
            // Always use the simple format when attachment points are hidden
            format!("J{}", joint_index)
        }
    }

    /// Format the near joint as a string
    pub fn near_joint_text(&self, show_attachment_points: bool) -> String {
        self.format_joint(true, show_attachment_points)
    }

    /// Format the far joint as a string
    pub fn far_joint_text(&self, show_attachment_points: bool) -> String {
        self.format_joint(false, show_attachment_points)
    }
}

#[derive(Clone, Debug, Copy)]
pub struct JointDetails {
    pub index: usize,
    pub location: Point3<f32>,
    pub scale: f32,
    pub selected_push: Option<UniqueId>,
}

impl Display for JointDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let location_mm = self.location_mm();
        let height_m = location_mm.y / 1000.0; // Convert mm to meters

        let surface_location = match self.surface_location_mm() {
            None => "".into(),
            Some((x, z)) => format!(" at ({x:.1} mm, {z:.1} mm)"),
        };

        write!(
            f,
            "{} at {:.2} m{}\nClick interval for details",
            self.joint_text(),
            height_m,
            surface_location
        )
    }
}

impl JointDetails {
    pub fn location_mm(&self) -> Point3<f32> {
        self.location.mul(self.scale)
    }

    pub fn surface_location_mm(&self) -> Option<(f32, f32)> {
        let Point3 { x, y, z } = self.location;
        (y <= 0.0).then(|| (x * self.scale, z * self.scale))
    }

    /// Format this joint as a string (e.g., "J1" or "J1:2" with attachment points)
    pub fn joint_text(&self) -> String {
        // Always use the simple format "J{index}" for joint labels
        // The attachment point numbers were causing confusion and showing invalid values
        format!("J{}", self.index)
    }
}

#[derive(Debug, Clone)]
pub enum ControlState {
    Waiting,
    Building,
    Shaping,
    Pretensing,
    Converging,
    Viewing,
    Animating,
    Converged,
    ShowingJoint(JointDetails),
    ShowingInterval(IntervalDetails),
    PhysicsTesting(TestScenario),
    Baking,
    UnderConstruction, // Deprecated - use Building instead
}

impl ControlState {
    pub fn send(self, radio: &Radio) {
        LabEvent::UpdateState(StateChange::SetControlState(self)).send(radio);
    }
}

#[derive(Debug, Clone)]
pub enum TesterAction {
    SetPhysicalParameter(PhysicsParameter),
    SetTweakParameter(TweakParameter),
    DumpPhysics,
    ToggleMovementSampler,
    DropFromHeight,
}

#[derive(Debug, Clone)]
pub enum CrucibleAction {
    BakeBrick(Prototype),
    BuildFabric(FabricPlan),
    CentralizeFabric(Option<units::Millimeters>),
    ToViewing,
    ToAnimating,
    ToPhysicsTesting(TestScenario),
    ToEvolving(u64),
    TesterDo(TesterAction),
}

impl CrucibleAction {
    pub fn send(self, radio: &Radio) {
        LabEvent::Crucible(self).send(&radio);
    }
}

#[derive(Debug, Clone)]
pub enum AppearanceMode {
    Faded,
    HighlightedPush,
    HighlightedPull,
    SelectedPush,
    SelectedPull,
}

#[derive(Debug, Clone)]
pub struct Appearance {
    pub color: [f32; 4],
    pub radius: f32,
}

impl Appearance {
    pub fn apply_mode(&self, mode: AppearanceMode) -> Self {
        match mode {
            // For Faded mode, we want to preserve the gray colors from the role's appearance
            // but make them slightly darker to indicate they're not selected
            AppearanceMode::Faded => {
                // Get the original color and darken it slightly
                let original_color = self.color;
                Self {
                    // Darken the color by multiplying each component by 0.7
                    color: [
                        original_color[0] * 0.7,
                        original_color[1] * 0.7,
                        original_color[2] * 0.7,
                        original_color[3],
                    ],
                    radius: self.radius,
                }
            }
            AppearanceMode::HighlightedPush => Self {
                color: [0.4, 0.4, 0.9, 1.0], // Bluish color for highlighted elements
                radius: self.radius,         // Keep radius unchanged
            },
            AppearanceMode::HighlightedPull => Self {
                color: [0.4, 0.4, 0.9, 1.0], // Bluish color for highlighted elements
                radius: self.radius,         // Keep radius unchanged
            },
            AppearanceMode::SelectedPush => Self {
                color: [0.0, 1.0, 0.0, 1.0], // Green color for selected elements
                radius: self.radius,         // Keep radius unchanged
            },
            AppearanceMode::SelectedPull => Self {
                color: [0.0, 1.0, 0.0, 1.0], // Green color for selected elements
                radius: self.radius,         // Keep radius unchanged
            },
        }
    }

    // Keep these methods for backward compatibility
    pub fn with_color(&self, color: [f32; 4]) -> Self {
        Self {
            color,
            radius: self.radius * 2.0,
        }
    }

    pub fn highlighted_for_role(&self, role: Role) -> Self {
        match role {
            Role::Pushing => self.apply_mode(AppearanceMode::HighlightedPush),
            _ => self.apply_mode(AppearanceMode::HighlightedPull),
        }
    }

    pub fn selected_for_role(&self, role: Role) -> Self {
        match role {
            Role::Pushing => self.apply_mode(AppearanceMode::SelectedPush),
            _ => self.apply_mode(AppearanceMode::SelectedPull),
        }
    }
}

type AppearanceFunction = Rc<dyn Fn(&Interval) -> Option<Appearance>>;

#[derive(Clone)]
pub enum StateChange {
    SetFabricName(String),
    SetFabricStats(Option<FabricStats>),
    SetControlState(ControlState),
    SetStageLabel(String), // Simple string label for current stage
    ResetView,
    ToggleColorByRole,
    SetAppearanceFunction(AppearanceFunction),
    SetIntervalColor {
        key: (usize, usize),
        color: [f32; 4],
    },
    SetAnimating(bool),
    SetExperimentTitle {
        title: String,
        fabric_stats: FabricStats,
    },
    SetKeyboardLegend(String),
    SetPhysicsParameter(PhysicsParameter),
    SetTweakParameter(TweakParameter),
    Time {
        frames_per_second: f32,
        age: Age,
        target_time_scale: f32,
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
            StateChange::SetAnimating(_) => "SetAnimating()",
            StateChange::SetExperimentTitle { .. } => "SetExperimentTitle()",
            StateChange::SetKeyboardLegend(_) => "SetKeyboardLegend()",
            StateChange::SetPhysicsParameter(_) => "SetPhysicsParameter()",
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
    ContextCreated { wgpu: Wgpu, mobile_device: bool },
    FabricBuilt(FabricStats),
    FabricCentralized(Vector3<f32>),
    Crucible(CrucibleAction),
    UpdateState(StateChange),
    UpdatedLibrary(Instant),
    RefreshLibrary,
    RebuildFabric,
    PrintCord(f32),
    DumpCSV,
    RequestRedraw,
    PointerChanged(PointerChange),
}

pub type Radio = winit::event_loop::EventLoopProxy<LabEvent>;

impl LabEvent {
    pub fn send(self, radio: &Radio) {
        radio.send_event(self).expect("Radio working")
    }
}

/// Represents the user's intent when clicking in the scene
#[derive(Debug, Clone)]
pub enum PickIntent {
    Reset,
    Select,
    Traverse,
}

#[derive(Debug, Clone)]
pub enum PointerChange {
    NoChange,
    Moved(PhysicalPosition<f64>),
    Pressed,
    Released(PickIntent),
    TouchPressed(PhysicalPosition<f64>),
    TouchReleased(PickIntent),
    Zoomed(f32),
}

thread_local! {
    static SHOW_ATTACHMENT_POINTS: RefCell<bool> = RefCell::new(false);
}
