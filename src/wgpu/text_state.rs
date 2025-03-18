use crate::application::AppStateChange;
use crate::fabric::interval::Role;
use crate::fabric::FabricStats;
use crate::messages::ControlState;
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
    fabric_name: String,
    control_state: ControlState,
    fabric_stats: Option<FabricStats>,
    muscles_active: bool,
    sections: [Option<OwnedSection>; SectionName::count()],
}

impl TextState {
    pub fn new(mobile_device: bool, fabric_name: String, width: u32, height: u32) -> Self {
        let mut fresh = Self {
            mobile_device,
            width: width as f32,
            height: height as f32,
            fabric_name,
            control_state: ControlState::default(),
            fabric_stats: None,
            muscles_active: false,
            sections: Default::default(),
        };
        fresh.update_sections();
        fresh
    }

    pub fn change_happened(&mut self, app_change: AppStateChange) {
        match app_change {
            AppStateChange::SetControlState(control_state) => {
                self.control_state = control_state;
            }
            AppStateChange::SetFabricStats(fabric_stats) => {
                self.control_state = if fabric_stats.is_some() {
                    ControlState::Viewing
                } else {
                    ControlState::Waiting
                };
                self.fabric_stats = fabric_stats;
            }
            AppStateChange::SetMusclesActive(muscles_active) => {
                self.muscles_active = muscles_active;
            }
        }
        self.update_sections()
    }

    pub fn sections(&self) -> Vec<&OwnedSection> {
        self.sections.iter().flatten().collect()
    }

    fn update_sections(&mut self) {
        self.update_section(SectionName::Top, Some(self.fabric_name.clone()));
        if !self.mobile_device {
            self.update_section(
                SectionName::Bottom,
                match &self.fabric_stats {
                    Some(_) => match &self.control_state {
                        ControlState::Waiting => None,
                        ControlState::Viewing => {
                            if self.muscles_active {
                                Some("Press SPACE to leave it alone".to_string())
                            } else {
                                Some(
                                    "Press SPACE to start pulling on the crossed guy lines"
                                        .to_string(),
                                )
                            }
                        }
                        ControlState::ShowingJoint(_) | ControlState::ShowingInterval(_) => None,
                    },
                    None => Some("Under construction...".to_string()),
                },
            );
            self.update_section(
                SectionName::Right,
                match self.control_state {
                    ControlState::Waiting => None,
                    ControlState::Viewing => {
                        if !self.muscles_active {
                            Some("Right-click to select".to_string())
                        } else {
                            None
                        }
                    }
                    ControlState::ShowingJoint(joint_details) => Some(format!(
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
                        let length = if let Some(stats) = &self.fabric_stats {
                            format!("{0:.1} mm", interval_details.length * stats.scale)
                        } else {
                            "?".to_string()
                        };
                        Some(format!(
                            "{} {}-{}\n\
                            Length: {}mm\n\
                            Right-click to jump\n\
                            ESC to release",
                            role,
                            Self::joint_format(interval_details.near_joint),
                            Self::joint_format(interval_details.far_joint),
                            length,
                        ))
                    }
                },
            );
        } else {
            self.update_section(
                SectionName::Right,
                match self.control_state {
                    ControlState::Viewing => Some("2025\nGerald de Jong\npretenst.com".to_string()),
                    _ => None,
                },
            );
        }

        self.update_section(
            SectionName::Left,
            self.fabric_stats.clone().map(
                |FabricStats {
                     scale,
                     joint_count,
                     max_height,
                     push_count,
                     push_total,
                     push_range,
                     pull_count,
                     pull_range,
                     pull_total,
                 }| {
                    format!(
                        "Stats:\n\
                         Height: {:.1}m\n\
                         Joints: {:?}\n\
                         Bars: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m\n\
                         Cables: {:?}\n\
                         → {:.1}-{:.1}mm\n\
                         → total {:.1}m",
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
                    )
                },
            ),
        );
    }

    fn update_section(&mut self, section_name: SectionName, new_text: Option<String>) {
        self.sections[section_name as usize] = new_text.map(|new_text| {
            self.create_section(section_name)
                .add_text(Self::create_text(section_name, new_text))
        });
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

    fn create_text(section_name: SectionName, text: String) -> OwnedText {
        OwnedText::new(text)
            .with_color([0.8, 0.8, 0.8, 1.0])
            .with_scale(match section_name {
                SectionName::Top => 60.0,
                _ => 30.0,
            })
    }

    fn joint_format(index: usize) -> String {
        format!("J{}", index + 1)
    }
}
