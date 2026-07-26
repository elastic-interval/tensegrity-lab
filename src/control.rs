//! UI state, render style, picking, and inspection-detail types.
//!
//! Anything the user sees or directly interacts with — render mode,
//! interval/joint detail overlays, pointer input — lives here.
//! Events that flow through the `Radio` live in `events.rs`.

use crate::caliper::caliper_reading;
use crate::events::{LabEvent, Radio, StateChange};
use crate::fabric::interval::{Interval, Role};
use crate::fabric::{IntervalKey, JointKey};
use crate::units::{Degrees, Meters};
use glam::Vec3;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::rc::Rc;
use winit::dpi::PhysicalPosition;

thread_local! {
    pub static SHOW_ATTACHMENT_POINTS: RefCell<bool> = RefCell::new(false);
}

// Tweak parameters that scale/modify the physics (user-controlled view on physics)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TweakFeature {
    MassScale,
    RigidityScale,
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

#[derive(Clone)]
pub enum RenderStyle {
    Normal,
    ColorByRole,
    WithAppearanceFunction {
        function: AppearanceFunction,
    },
    WithPullMap {
        map: HashMap<(JointKey, JointKey), [f32; 4]>,
    },
    WithPushMap {
        map: HashMap<(JointKey, JointKey), [f32; 4]>,
    },
}

#[derive(Clone, Debug)]
pub struct IntervalDetails {
    pub key: IntervalKey,
    pub near_joint: JointKey,
    pub near_joint_label: String,
    pub near_slot: Option<usize>,
    pub far_slot: Option<usize>,
    pub far_joint: JointKey,
    pub far_joint_label: String,
    pub alpha_pivot_angle: Option<Degrees>,
    pub omega_pivot_angle: Option<Degrees>,
    pub length: Meters,
    pub strain: f32,
    pub distance: Meters,
    pub role: Role,
    pub selected_push: Option<IntervalKey>,
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

        // Build pivot angle info if attachments are visible and we have angles
        let angle_info = if show_attachment_points {
            let alpha_angle = self
                .alpha_pivot_angle
                .map(|a| format!("α: {}", a))
                .unwrap_or_default();
            let omega_angle = self
                .omega_pivot_angle
                .map(|a| format!("ω: {}", a))
                .unwrap_or_default();
            if !alpha_angle.is_empty() || !omega_angle.is_empty() {
                let separator = if !alpha_angle.is_empty() && !omega_angle.is_empty() {
                    ", "
                } else {
                    ""
                };
                format!("\nPivot: {}{}{}", alpha_angle, separator, omega_angle)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        write!(
            f,
            "{} {}-{}\nDistance: {:.1} mm\nStrain: {:.6}%{}\nRight-click to jump",
            role_text,
            self.near_joint_text(show_attachment_points),
            self.far_joint_text(show_attachment_points),
            self.distance_mm(),
            self.strain_percent(),
            angle_info
        )
    }
}

impl IntervalDetails {
    pub fn length_mm(&self) -> f32 {
        self.length.to_mm()
    }

    pub fn strain_percent(&self) -> f32 {
        self.strain * 100.0
    }

    pub fn distance_mm(&self) -> f32 {
        self.distance.to_mm()
    }

    /// Format a joint label as a string, optionally with a slot number
    /// If show_attachment_points is false, the slot number will be hidden
    pub fn format_joint(&self, is_near: bool, show_attachment_points: bool) -> String {
        let (joint_label, slot) = if is_near {
            (&self.near_joint_label, self.near_slot)
        } else {
            (&self.far_joint_label, self.far_slot)
        };

        // Only show slot numbers if attachment points are visible
        if show_attachment_points {
            match slot {
                Some(slot_idx) => format!("{}:{}", joint_label, slot_idx),
                None => joint_label.clone(),
            }
        } else {
            // Always use the simple format when attachment points are hidden
            joint_label.clone()
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

    pub fn format_with_scale(&self, scale: f32) -> String {
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

        let show_attachment_points = SHOW_ATTACHMENT_POINTS.with(|cell| *cell.borrow());

        let angle_info = if show_attachment_points {
            let alpha_angle = self
                .alpha_pivot_angle
                .map(|a| format!("α: {}", a))
                .unwrap_or_default();
            let omega_angle = self
                .omega_pivot_angle
                .map(|a| format!("ω: {}", a))
                .unwrap_or_default();
            if !alpha_angle.is_empty() || !omega_angle.is_empty() {
                let separator = if !alpha_angle.is_empty() && !omega_angle.is_empty() {
                    ", "
                } else {
                    ""
                };
                format!("\nPivot: {}{}{}", alpha_angle, separator, omega_angle)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let scaled_distance_mm = self.distance_mm() * scale;

        // Show caliper reading for Pull intervals when in model-scale mode.
        let caliper_info = if self.role.is_pull_like() && (scale - 1.0).abs() > 0.001 {
            let reading = caliper_reading(Meters(self.distance.0 * scale));
            format!("\nCaliper: {} mm", reading)
        } else {
            String::new()
        };

        format!(
            "{} {}-{}\nDistance: {:.1} mm\nStrain: {:.6}%{}{}\nRight-click to jump",
            role_text,
            self.near_joint_text(show_attachment_points),
            self.far_joint_text(show_attachment_points),
            scaled_distance_mm,
            self.strain_percent(),
            angle_info,
            caliper_info
        )
    }
}

#[derive(Clone, Debug)]
pub struct JointDetails {
    pub key: JointKey,
    pub path: String,
    pub location: Vec3,
    pub selected_push: Option<IntervalKey>,
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
    pub fn location_mm(&self) -> Vec3 {
        // Coordinates are in meters, convert to mm
        self.location * 1000.0
    }

    pub fn surface_location_mm(&self) -> Option<(f32, f32)> {
        let Vec3 { x, y, z } = self.location;
        // Coordinates are in meters, convert to mm
        (y <= 0.0).then(|| (x * 1000.0, z * 1000.0))
    }

    /// Format this joint as a string using the path
    pub fn joint_text(&self) -> String {
        self.path.clone()
    }

    pub fn format_with_scale(&self, scale: f32) -> String {
        let height_m = self.location.y * scale;

        let surface_location = match self.surface_location_mm() {
            None => "".into(),
            Some((x, z)) => format!(" at ({:.1} mm, {:.1} mm)", x * scale, z * scale),
        };

        format!(
            "{} at {:.2} m{}\nClick interval for details",
            self.joint_text(),
            height_m,
            surface_location
        )
    }
}

#[derive(Debug, Clone)]
pub enum ControlState {
    Waiting,
    Building,
    Viewing { animation_available: bool },
    Animating,
    ShowingJoint(JointDetails),
    ShowingInterval(IntervalDetails),
    PhysicsTesting,
    Baking,
}

impl ControlState {
    pub fn send(self, radio: &Radio) {
        LabEvent::UpdateState(StateChange::SetControlState(self)).send(radio);
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

pub type AppearanceFunction = Rc<dyn Fn(&Interval) -> Option<Appearance>>;

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
