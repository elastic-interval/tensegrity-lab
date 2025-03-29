use crate::application::AppStateChange;
use crate::fabric::interval::Role;
use crate::fabric::FabricStats;
use crate::messages::{ControlState, Scenario};
use crate::ITERATIONS_PER_FRAME;
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
}

impl SectionName {
    const fn count() -> usize {
        4
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
    iterations_per_frame: usize,
    animating: bool,
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
            iterations_per_frame: ITERATIONS_PER_FRAME,
            sections: Default::default(),
        };
        fresh.update_sections();
        fresh
    }

    pub fn change_happened(&mut self, app_change: &AppStateChange) {
        match app_change {
            AppStateChange::SetControlState(control_state) => {
                self.control_state = control_state.clone();
            }
            AppStateChange::SetFabricName(fabric_name) => {
                self.fabric_name = Some(fabric_name.to_string());
            }
            AppStateChange::SetFabricStats(fabric_stats) => {
                self.fabric_stats = fabric_stats.clone();
            }
            AppStateChange::SetAnimating(animating) => {
                self.animating = animating.clone();
            }
            AppStateChange::SetExperimentTitle {
                title,
                fabric_stats,
            } => {
                self.experiment_title = title.clone();
                self.fabric_stats = Some(fabric_stats.clone());
            }
            AppStateChange::SetKeyboardLegend(legend) => {
                self.keyboard_legend = Some(legend.clone());
            }
            AppStateChange::SetIterationsPerFrame(iterations) => {
                self.iterations_per_frame = *iterations;
            }
            _ => {}
        }
        self.update_sections()
    }

    pub fn sections(&self) -> Vec<&OwnedSection> {
        self.sections.iter().flatten().collect()
    }

    fn update_sections(&mut self) {
        use TextInstance::*;
        let control_state = self.control_state.clone();
        if let Some(fabric_name) = &self.fabric_name {
            self.update_section(
                SectionName::Top,
                match control_state {
                    ControlState::Testing(scenario) => match scenario {
                        Scenario::TensionTest => Normal(format!(
                            "Tension test of {} {}",
                            fabric_name, self.experiment_title
                        )),
                        Scenario::CompressionTest => Normal(format!(
                            "Compression test of {} {}",
                            fabric_name, self.experiment_title
                        )),
                        _ => Nothing,
                    },
                    _ => Large(fabric_name.clone()),
                },
            );
        }

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
                    ControlState::Viewing => Normal("Right-click to select".to_string()),
                    ControlState::ShowingJoint(joint_details) => Large(format!(
                        "{}\n\
                        Click for details\n\
                        ESC to release",
                        Self::joint_format(joint_details.index),
                    )),
                    ControlState::ShowingInterval(interval_details) => {
                        let role = match interval_details.role {
                            Role::Push => "Strut",
                            Role::Pull => "Cable",
                            Role::Spring => "Spring",
                        };
                        let length_string = if let Some(stats) = &self.fabric_stats {
                            format!("{0:.1} mm", interval_details.length * stats.scale)
                        } else {
                            "?".to_string()
                        };
                        Large(format!(
                            "{} {}-{}\n\
                            Length: {}\n\
                            Right-click to jump\n\
                            ESC to release",
                            role,
                            Self::joint_format(interval_details.near_joint),
                            Self::joint_format(interval_details.far_joint),
                            length_string,
                        ))
                    }
                    _ => Nothing,
                },
            );
        } else {
            self.update_section(
                SectionName::Right,
                match control_state {
                    ControlState::Animating => {
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
                        age,
                        scale,
                        joint_count,
                        max_height,
                        push_count,
                        push_total,
                        push_range,
                        pull_count,
                        pull_range,
                        pull_total,
                    } = fabric_stats;
                    Normal(format!(
                        "Stats:\n\
                         Height: {:.1}m\n\
                         Joints: {:?}\n\
                         Bars: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m\n\
                         Cables: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m\n\
                         Age: {}k iterations\n\
                         {} per frame\n\
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
                        age / 1000,
                        self.iterations_per_frame,
                    ))
                }
            },
        );
    }

    fn update_section(&mut self, section_name: SectionName, text_instance: TextInstance) {
        let section = self.create_section(section_name);
        let scale_factor = text_instance.scale_factor();
        self.sections[section_name as usize] = Some(match text_instance {
            TextInstance::Nothing => section,
            TextInstance::Normal(text) | TextInstance::Large(text) => section.add_text(
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
        Layout::default()
            .v_align(match section_name {
                SectionName::Top => VerticalAlign::Top,
                SectionName::Bottom => VerticalAlign::Bottom,
                SectionName::Left => VerticalAlign::Center,
                SectionName::Right => VerticalAlign::Center,
            })
            .h_align(match section_name {
                SectionName::Top => HorizontalAlign::Center,
                SectionName::Bottom => HorizontalAlign::Center,
                SectionName::Left => HorizontalAlign::Left,
                SectionName::Right => HorizontalAlign::Right,
            })
    }

    fn create_bounds(&self, section_name: SectionName) -> [f32; 2] {
        let middle = self.width / 2.0;
        match section_name {
            SectionName::Top => [self.width, self.width],
            SectionName::Bottom => [self.width, self.width],
            SectionName::Left => [middle, self.width],
            SectionName::Right => [middle, self.width],
        }
    }

    fn create_position(&self, section_name: SectionName) -> [f32; 2] {
        let middle_h = self.width / 2.0;
        let middle_v = self.height / 2.0;
        let margin = 50.0;
        match section_name {
            SectionName::Top => [middle_h, margin],
            SectionName::Bottom => [middle_h, self.height - margin],
            SectionName::Left => [margin, middle_v],
            SectionName::Right => [self.width - margin, middle_v],
        }
    }

    fn joint_format(index: usize) -> String {
        format!("J{}", index)
    }
}
