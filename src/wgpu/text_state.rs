use crate::Age;
use crate::FabricStats;
use crate::{ControlState, StateChange, TestScenario};
use std::default::Default;
use wgpu_text::glyph_brush::{
    BuiltInLineBreaker, HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign,
};

#[derive(Clone, Debug, Copy)]
pub enum SectionName {
    Top = 0,
    TopSubtitle = 1,
    Bottom = 2,
    Left = 3,
    Right = 4,
    BottomLeft = 5,
    Center = 6,
}

impl SectionName {
    const fn count() -> usize {
        7
    }
}

#[derive(Clone, Debug)]
pub struct TextState {
    mobile_device: bool,
    width: f32,
    height: f32,
    fabric_name: Option<String>,
    experiment_title: String,
    control_state: ControlState,
    stage_label: String,
    fabric_stats: Option<FabricStats>,
    movement_analysis: Option<String>,
    sections: [Option<OwnedSection>; SectionName::count()],
    keyboard_legend: Option<String>,
    animating: bool,
    frames_per_second: f32,
    age: Age,
    time_scale: f32,
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
    pub fn new(mobile_device: bool, width: u32, height: u32) -> Self {
        let mut fresh = Self {
            mobile_device,
            width: width as f32,
            height: height as f32,
            fabric_name: None,
            animating: false,
            experiment_title: "".to_string(),
            control_state: ControlState::Waiting,
            stage_label: "Waiting".to_string(),
            fabric_stats: None,
            movement_analysis: None,
            keyboard_legend: None,
            sections: Default::default(),
            frames_per_second: 0.0,
            age: Age::default(),
            time_scale: 1.0,
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
            SetStageLabel(label) => {
                self.stage_label = label.clone();
            }
            SetFabricName(fabric_name) => {
                self.fabric_name = Some(fabric_name.to_string());
            }
            SetFabricStats(fabric_stats) => {
                self.fabric_stats = fabric_stats.clone();
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
            SetKeyboardLegend(legend) => {
                self.keyboard_legend = Some(legend.clone());
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
                    PhysicsTesting(scenario) => match scenario {
                        TestScenario::PhysicsTest => Large(format!(
                            "Physics test of {} {}",
                            fabric_name, self.experiment_title
                        )),
                        _ => unreachable!(),
                    },
                    _ => Large(fabric_name.clone()),
                },
            );
            
            // Add state subtitle below title (normal size, not small)
            self.update_section(
                SectionName::TopSubtitle,
                Normal(self.stage_label.clone()),
            );
        }

        // Show time scale if not 1.0
        let bottom_left_text = if (self.time_scale - 1.0).abs() > 0.01 {
            format!("{:.0}fps {} ({:.1}×)", self.frames_per_second, self.age, self.time_scale)
        } else {
            format!("{:.0}fps {}", self.frames_per_second, self.age)
        };

        self.update_section(
            SectionName::BottomLeft,
            Normal(bottom_left_text),
        );

        if !self.mobile_device {
            self.update_section(
                SectionName::Bottom,
                match &self.keyboard_legend {
                    Some(legend) => Normal(legend.clone()),
                    None => Nothing,
                },
            );
            self.update_section(
                SectionName::Right,
                match control_state {
                    Viewing { .. } => Large("Click to select".to_string()),
                    ShowingJoint(joint_details) => Large(joint_details.to_string()),
                    ShowingInterval(interval_details) => Large(interval_details.to_string()),
                    PhysicsTesting(_) => match &self.movement_analysis {
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
                    Animating => {
                        Normal("2025\nGerald de Jong\npretenst.com".to_string())
                    }
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
                        height,
                        push_count,
                        push_total,
                        push_range,
                        pull_count,
                        pull_range,
                        pull_total,
                        age,
                        scale,
                        ..
                    } = fabric_stats;

                    let scale_mm = scale.to_mm();
                    let text = format!(
                        "Stats at {age}:\n\
                         Height: {:.3}m\n\
                         Joints: {:?}\n\
                         Bars: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m\n\
                         Cables: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m",
                        height.0,
                        joint_count,
                        push_count,
                        push_range.0 * scale_mm,
                        push_range.1 * scale_mm,
                        push_total * scale_mm / 1000.0,
                        pull_count,
                        pull_range.0 * scale_mm,
                        pull_range.1 * scale_mm,
                        pull_total * scale_mm / 1000.0,
                    );
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
                    .with_color([0.8, 0.8, 0.8, 1.0])
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
                Bottom => VerticalAlign::Bottom,
                Left => VerticalAlign::Center,
                Right => VerticalAlign::Center,
                BottomLeft => VerticalAlign::Bottom,
                Center => VerticalAlign::Center,
            })
            .h_align(match section_name {
                Top | TopSubtitle => HorizontalAlign::Center,
                Bottom => HorizontalAlign::Center,
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
            Bottom => [self.width, self.width],
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
            TopSubtitle => [middle_h, margin + 70.0],  // Below the title
            Bottom => [middle_h, self.height - margin],
            Left => [margin, middle_v],
            Right => [self.width - margin, middle_v],
            BottomLeft => [margin, self.height - margin],
            Center => [middle_h, middle_v], // Dead center of screen
        }
    }
}
