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
    Bottom = 1,
    Left = 2,
    Right = 3,
    BottomLeft = 4,
}

impl SectionName {
    const fn count() -> usize {
        5
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
    fabric_stats: Option<FabricStats>,
    sections: [Option<OwnedSection>; SectionName::count()],
    keyboard_legend: Option<String>,
    animating: bool,
    frames_per_second: f32,
    age: Age,
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
            fabric_stats: None,
            keyboard_legend: None,
            sections: Default::default(),
            frames_per_second: 0.0,
            age: Age::default(),
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
            } => {
                self.frames_per_second = frames_per_second.clone();
                self.age = *age;
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
                    FailureTesting(scenario) => match scenario {
                        TestScenario::TensionTest => Large(format!(
                            "Tension test of {} {}",
                            fabric_name, self.experiment_title
                        )),
                        TestScenario::CompressionTest => Large(format!(
                            "Compression test of {} {}",
                            fabric_name, self.experiment_title
                        )),
                        _ => unreachable!(),
                    },
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
        }

        self.update_section(
            SectionName::BottomLeft,
            Normal(format!("{:.0}fps {}", self.frames_per_second, self.age)),
        );

        if !self.mobile_device {
            self.update_section(
                SectionName::Bottom,
                match &self.keyboard_legend {
                    None => Nothing,
                    Some(legend) => Normal(legend.clone()),
                },
            );
            self.update_section(
                SectionName::Right,
                match control_state {
                    Viewing => Large("Click to select".to_string()),
                    ShowingJoint(joint_details) => Large(joint_details.to_string()),
                    ShowingInterval(interval_details) => Large(interval_details.to_string()),
                    _ => Nothing,
                },
            );
        } else {
            self.update_section(
                SectionName::Right,
                match control_state {
                    Animating => {
                        Normal("2025\nGerald de Jong\nAte Snijder\npretenst.com".to_string())
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
                        scale,
                        joint_count,
                        max_height,
                        push_count,
                        push_total,
                        push_range,
                        pull_count,
                        pull_range,
                        pull_total,
                        age,
                        ..
                    } = fabric_stats;
                    Normal(format!(
                        "Stats at {age}:\n\
                         Height: {:.1}m\n\
                         Joints: {:?}\n\
                         Bars: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m\n\
                         Cables: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m\n\
                         ",
                        max_height * scale / 1000.0,
                        joint_count,
                        push_count,
                        push_range.0 * scale,
                        push_range.1 * scale,
                        push_total * scale / 1000.0,
                        pull_count,
                        pull_range.0 * scale,
                        pull_range.1 * scale,
                        pull_total * scale / 1000.0,
                    ))
                }
            },
        );
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
        })
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
                Top => VerticalAlign::Top,
                Bottom => VerticalAlign::Bottom,
                Left => VerticalAlign::Center,
                Right => VerticalAlign::Center,
                BottomLeft => VerticalAlign::Bottom,
            })
            .h_align(match section_name {
                Top => HorizontalAlign::Center,
                Bottom => HorizontalAlign::Center,
                Left => HorizontalAlign::Left,
                Right => HorizontalAlign::Right,
                BottomLeft => HorizontalAlign::Left,
            })
    }

    fn create_bounds(&self, section_name: SectionName) -> [f32; 2] {
        use SectionName::*;
        let middle = self.width / 2.0;
        match section_name {
            Top => [self.width, self.width],
            Bottom => [self.width, self.width],
            Left => [middle, self.width],
            Right => [middle, self.width],
            BottomLeft => [middle, middle],
        }
    }

    fn create_position(&self, section_name: SectionName) -> [f32; 2] {
        use SectionName::*;
        let middle_h = self.width / 2.0;
        let middle_v = self.height / 2.0;
        let margin = 50.0;
        match section_name {
            Top => [middle_h, margin],
            Bottom => [middle_h, self.height - margin],
            Left => [margin, middle_v],
            Right => [self.width - margin, middle_v],
            BottomLeft => [margin, self.height - margin],
        }
    }
}
