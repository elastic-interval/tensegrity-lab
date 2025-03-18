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
    width: f32,
    height: f32,
    fabric_name: String,
    control_state: ControlState,
    fabric_stats: Option<FabricStats>,
    sections: [Option<OwnedSection>; SectionName::count()],
}

impl TextState {
    pub fn new(fabric_name: String, width: u32, height: u32) -> Self {
        let mut fresh = Self {
            width: width as f32,
            height: height as f32,
            fabric_name,
            control_state: ControlState::default(),
            fabric_stats: None,
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
        }
        self.update_sections()
    }

    pub fn sections(&self) -> Vec<&OwnedSection> {
        self.sections.iter().flatten().collect()
    }

    fn update_sections(&mut self) {
        self.update_section(SectionName::Top, Some(self.fabric_name.clone()));
        self.update_section(
            SectionName::Bottom,
            Some(match &self.fabric_stats {
                Some(_) => "Press D to toggle dancing".to_string(),
                None => "Please wait while the tensegrity is constructed".to_string(),
            }),
        );
        self.update_section(
            SectionName::Left,
            match &self.fabric_stats {
                Some(fabric_stats) => Some(format!("{fabric_stats}")),
                None => None,
            },
        );
        self.update_section(
            SectionName::Right,
            match self.control_state {
                ControlState::Waiting => None,
                ControlState::Viewing => Some("Right-click to select".to_string()),
                ControlState::ShowingJoint(joint_details) => {
                    Some(format!(
                        "Joint: {}\n\
                        Click an interval for details\n\
                        Right-click for an adjacent joint\n\
                        Press ESC to release",
                        Self::joint_format(joint_details.index),
                    ))
                }
                ControlState::ShowingInterval(interval_details) => {
                    let role = match interval_details.role {
                        Role::Push => "strut",
                        Role::Pull => "cable",
                        Role::Spring => "spring",
                    };
                    let length = if let Some(stats) = &self.fabric_stats {
                            format!("{0:.1} mm", interval_details.length * stats.scale)
                        } else {
                            "?".to_string()
                        };
                    Some(format!(
                        "Joint: {}\n\
                        Green {} to: {}\n\
                        Length: {}\n\
                        Right-click\n\
                        to jump across",
                        Self::joint_format(interval_details.near_joint),
                        role,
                        Self::joint_format(interval_details.far_joint),
                        length,
                    ))
                }
            },
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
            SectionName::Top => [middle, self.width],
            SectionName::Bottom => [middle, self.width],
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
                SectionName::Top => 80.0,
                _ => 40.0,
            })
    }

    fn joint_format(index: usize) -> String {
        format!("J{}", index + 1)
    }
}
