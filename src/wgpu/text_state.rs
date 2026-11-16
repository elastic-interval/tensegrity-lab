use crate::fabric::{FabricMotion, FabricStats};
use crate::{Age, ControlState, StateChange};
use crate::caliper::brass_tube_mm;
use std::default::Default;
use wgpu_text::glyph_brush::{
    BuiltInLineBreaker, HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign,
};

#[derive(Clone, Debug, Copy)]
pub enum SectionName {
    Top = 0,
    TopSubtitle = 1,
    Left = 2,
    Right = 3,
    BottomLeft = 4,
    Center = 5,
}

impl SectionName {
    const fn count() -> usize {
        6
    }
}

#[derive(Clone, Debug)]
pub struct TextState {
    mobile_device: bool,
    model_scale: Option<f32>,
    width: f32,
    height: f32,
    fabric_name: Option<String>,
    experiment_title: String,
    control_state: ControlState,
    stage_label: String,
    quaking: bool,
    fabric_stats: Option<FabricStats>,
    movement_analysis: Option<String>,
    sections: [Option<OwnedSection>; SectionName::count()],
    animating: bool,
    frames_per_second: f32,
    age: Age,
    time_scale: f32,
    show_mode: bool,
    /// Dark text, on the white background.
    white: bool,
}

enum TextInstance {
    Nothing,
    Normal(String),
    Large(String),
}

impl TextInstance {
    pub fn scale_factor(&self) -> f32 {
        match self {
            TextInstance::Nothing => 10.0,
            TextInstance::Normal(_) => 30.0,
            TextInstance::Large(_) => 60.0,
        }
    }
}

impl TextState {
    pub fn new(mobile_device: bool, model_scale: Option<f32>, width: u32, height: u32) -> Self {
        let mut fresh = Self {
            mobile_device,
            model_scale,
            width: width as f32,
            height: height as f32,
            fabric_name: None,
            animating: false,
            experiment_title: "".to_string(),
            control_state: ControlState::Waiting,
            stage_label: "Waiting".to_string(),
            quaking: false,
            fabric_stats: None,
            movement_analysis: None,
            sections: Default::default(),
            frames_per_second: 0.0,
            age: Age::default(),
            time_scale: 1.0,
            show_mode: false,
            white: false,
        };
        fresh.update_sections();
        fresh
    }

    pub fn update_state(&mut self, app_change: &StateChange) {
        use StateChange::*;
        match app_change {
            SetControlState(control_state) => {
                self.control_state = control_state.clone();
            }
            SetQuaking(quaking) => {
                self.quaking = *quaking;
            }
            SetStageLabel(label) => {
                self.stage_label = label.clone();
            }
            SetFabricName(fabric_name) => {
                self.fabric_name = Some(fabric_name.to_string());
            }
            SetFabricStats(fabric_stats) => {
                self.fabric_stats = fabric_stats.clone();
            }
            SetFabricMotion(motion) => {
                if let Some(fabric_stats) = &mut self.fabric_stats {
                    fabric_stats.motion = motion.clone();
                }
            }
            SetAnimating(animating) => {
                self.animating = animating.clone();
            }
            SetExperimentTitle {
                title,
                fabric_stats,
            } => {
                self.experiment_title = title.clone();
                self.fabric_stats = Some(fabric_stats.clone());
            }
            Time {
                frames_per_second,
                age,
                time_scale,
            } => {
                self.frames_per_second = frames_per_second.clone();
                self.age = *age;
                self.time_scale = *time_scale;
            }
            ShowMovementAnalysis(text) => {
                self.movement_analysis = text.clone();
            }
            SetShowMode(on) => {
                self.show_mode = *on;
            }
            ToggleBackground => {
                self.white = !self.white;
            }
            _ => {}
        }
        self.update_sections()
    }

    pub fn sections(&self) -> Vec<&OwnedSection> {
        self.sections.iter().flatten().collect()
    }

    fn update_sections(&mut self) {
        use ControlState::*;
        use TextInstance::*;
        let control_state = &self.control_state.clone();
        if let Some(fabric_name) = &self.fabric_name {
            self.update_section(
                SectionName::Top,
                match control_state {
                    PhysicsTesting => Large(format!(
                        "Physics test of {} {}",
                        fabric_name, self.experiment_title
                    )),
                    _ => Large(fabric_name.clone()),
                },
            );

            // Add state subtitle below title (normal size, not small)
            let subtitle = if self.quaking { "Quaking".to_string() } else { self.stage_label.clone() };
            self.update_section(SectionName::TopSubtitle, Normal(subtitle));
        }

        // Show time scale if not 1.0
        let bottom_left_text = if (self.time_scale - 1.0).abs() > 0.01 {
            format!(
                "{:.0}fps {} ({:.1}×)",
                self.frames_per_second, self.age, self.time_scale
            )
        } else {
            format!("{:.0}fps {}", self.frames_per_second, self.age)
        };

        self.update_section(SectionName::BottomLeft, Normal(bottom_left_text));

        if self.show_mode {
            // Show mode (cycle): hide stats, controls, hints; keep only the
            // title, subtitle, and the bottom-left fps/age strip.
            self.update_section(SectionName::Left, Nothing);
            self.update_section(SectionName::Right, Nothing);
            self.update_section(SectionName::Center, Nothing);
            return;
        }

        if !self.mobile_device {
            // What the selection is shows in the scene, on its labels.
            self.update_section(
                SectionName::Right,
                match control_state {
                    Viewing { .. } => Large("Click: joint".to_string()),
                    ShowingJoint(_) => Large(
                        "Click: interval\nRight-click: other joint".to_string(),
                    ),
                    ShowingInterval(_) => Large(
                        "Click: next interval\nRight-click: far end".to_string(),
                    ),
                    PhysicsTesting => match &self.movement_analysis {
                        Some(text) => Normal(text.clone()),
                        None => Nothing,
                    },
                    _ => Nothing,
                },
            );
        } else {
            self.update_section(
                SectionName::Right,
                match control_state {
                    Animating => Normal("2025\nGerald de Jong\npretenst.com".to_string()),
                    _ => Nothing,
                },
            );
        }

        self.update_section(
            SectionName::Left,
            match &self.fabric_stats {
                None => Nothing,
                Some(fabric_stats) => {
                    let FabricStats {
                        joint_count,
                        push_count,
                        push_total,
                        push_range,
                        pull_count,
                        pull_range,
                        pull_total,
                        age,
                        mass,
                        motion,
                        ..
                    } = fabric_stats;
                    let FabricMotion {
                        height,
                        max_cable_force,
                        max_strut_force,
                        cable_strain_range,
                        strut_strain_range,
                        slack_cable_count,
                    } = motion;

                    let scale_label = self
                        .model_scale
                        .map(|s| {
                            let ratio = 1.0 / s;
                            if (ratio - ratio.round()).abs() < 0.01 {
                                format!(" ({}:1)", ratio as u32)
                            } else {
                                format!(" ({:.1}:1)", ratio)
                            }
                        })
                        .unwrap_or_default();

                    let text = match self.model_scale {
                        Some(scale) => {
                            let struts = *push_count as f32;
                            let tube_mm = brass_tube_mm(push_total.to_mm() * scale / struts) * struts;
                            format!(
                                "Built at {age}{scale_label}:\n\
                                 Joints: {:?}\n\
                                 Bars: {:?}\n\
                                 → {:.1}-{:.1}mm\n\
                                 → total {:.2}m\n\
                                 → brass tube {:.2}m\n\
                                 Cables: {:?}\n\
                                 → {:.1}-{:.1}mm\n\
                                 → total {:.2}m\n\
                                 Height: {:.0}mm",
                                joint_count,
                                push_count,
                                push_range.0.to_mm() * scale,
                                push_range.1.to_mm() * scale,
                                push_total.0 * scale,
                                tube_mm / 1000.0,
                                pull_count,
                                pull_range.0.to_mm() * scale,
                                pull_range.1.to_mm() * scale,
                                pull_total.0 * scale,
                                height.to_mm() * scale,
                            )
                        }
                        None => format!(
                            "Built at {age}:\n\
                             Joints: {:?}\n\
                             Bars: {:?}\n\
                             → {:.1}-{:.1}mm\n\
                             → total {:.1}m\n\
                             Cables: {:?}\n\
                             → {:.1}-{:.1}mm\n\
                             → total {:.1}m\n\
                             Mass: {:.0}kg\n\
                             \n\
                             Each second:\n\
                             Height: {:.3}m\n\
                             Max cable: {:.1}kN\n\
                             Max bar: {:.1}kN\n\
                             Cable strain: {:.2}% to {:.2}%\n\
                             Bar strain: {:.2}% to {:.2}%\n\
                             Slack cables: {}",
                            joint_count,
                            push_count,
                            push_range.0.to_mm(),
                            push_range.1.to_mm(),
                            push_total.0,
                            pull_count,
                            pull_range.0.to_mm(),
                            pull_range.1.to_mm(),
                            pull_total.0,
                            mass.to_kg(),
                            height.0,
                            max_cable_force.to_kilonewtons(),
                            max_strut_force.to_kilonewtons(),
                            cable_strain_range.0 * 100.0,
                            cable_strain_range.1 * 100.0,
                            strut_strain_range.0 * 100.0,
                            strut_strain_range.1 * 100.0,
                            slack_cable_count,
                        ),
                    };
                    Normal(text)
                }
            },
        );

        // Center section reserved for future use
        self.update_section(SectionName::Center, Nothing);
    }

    fn update_section(&mut self, section_name: SectionName, text_instance: TextInstance) {
        use TextInstance::*;
        let section = self.create_section(section_name);
        let scale_factor = text_instance.scale_factor();
        self.sections[section_name as usize] = Some(match text_instance {
            Nothing => section,
            Normal(text) | Large(text) => section.add_text(
                OwnedText::new(text)
                    .with_color(if self.white { [0.0, 0.0, 0.0, 1.0] } else { [0.8, 0.8, 0.8, 1.0] })
                    .with_scale(scale_factor),
            ),
        });
    }

    fn create_section(&self, section_name: SectionName) -> OwnedSection {
        OwnedSection::default()
            .with_layout(Self::create_layout(section_name))
            .with_bounds(self.create_bounds(section_name))
            .with_screen_position(self.create_position(section_name))
    }

    fn create_layout(section_name: SectionName) -> Layout<BuiltInLineBreaker> {
        use SectionName::*;
        Layout::default()
            .v_align(match section_name {
                Top | TopSubtitle => VerticalAlign::Top,
                Left => VerticalAlign::Center,
                Right => VerticalAlign::Center,
                BottomLeft => VerticalAlign::Bottom,
                Center => VerticalAlign::Center,
            })
            .h_align(match section_name {
                Top | TopSubtitle => HorizontalAlign::Center,
                Left => HorizontalAlign::Left,
                Right => HorizontalAlign::Right,
                BottomLeft => HorizontalAlign::Left,
                Center => HorizontalAlign::Center,
            })
    }

    fn create_bounds(&self, section_name: SectionName) -> [f32; 2] {
        use SectionName::*;
        let middle = self.width / 2.0;
        match section_name {
            Top | TopSubtitle => [self.width, self.width],
            Left => [middle, self.width],
            Right => [middle, self.width],
            BottomLeft => [middle, middle],
            Center => [self.width * 0.8, self.height * 0.8], // Large center area
        }
    }

    fn create_position(&self, section_name: SectionName) -> [f32; 2] {
        use SectionName::*;
        let middle_h = self.width / 2.0;
        let middle_v = self.height / 2.0;
        let margin = 50.0;
        match section_name {
            Top => [middle_h, margin],
            TopSubtitle => [middle_h, margin + 70.0], // Below the title
            Left => [margin, middle_v],
            Right => [self.width - margin, middle_v],
            BottomLeft => [margin, self.height - margin],
            Center => [middle_h, middle_v], // Dead center of screen
        }
    }
}
