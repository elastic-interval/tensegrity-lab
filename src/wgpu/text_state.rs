use crate::application::OverlayChange;
use crate::fabric::FabricStats;
use crate::messages::ControlState;
use std::default::Default;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};

#[derive(Clone, Debug, Copy)]
pub enum SectionName {
    Top = 0,
    Bottom = 1,
}

impl SectionName {
    const fn count() -> usize {
        2
    }
}

#[derive(Clone, Debug)]
pub struct TextState {
    width: f32,
    height: f32,
    fabric_name: String,
    control_state: ControlState,
    fabric_stats: Option<FabricStats>,
    show_details: bool,
    show_stats: bool,
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
            show_stats: false,
            show_details: false,
            sections: Default::default(),
        };
        fresh.update_sections();
        fresh
    }

    pub fn change_happened(&mut self, app_change: OverlayChange) {
        match app_change {
            OverlayChange::SetControlState(control_state) => {
                self.control_state = control_state;
            }
            OverlayChange::SetFabricStats(fabric_stats) => {
                self.control_state = if fabric_stats.is_some() {
                    ControlState::Viewing
                } else {
                    ControlState::Waiting
                };
                self.fabric_stats = fabric_stats;
            }
            OverlayChange::ToggleShowDetails => {
                self.show_details = !self.show_details;
                if self.show_details {
                    self.control_state = ControlState::Viewing;
                }
            }
            OverlayChange::ToggleShowStats => {
                self.show_stats = !self.show_stats;
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
            match &self.fabric_stats {
                Some(fabric_stats) => Some(format!("{fabric_stats:?}")),
                None => Some("Building...".to_string()),
            },
        );
    }

    fn update_section(&mut self, section_name: SectionName, new_text: Option<String>) {
        self.sections[section_name as usize] = new_text.map(|new_text| {
            self.create_section(section_name)
                .add_text(Self::create_text(new_text))
        });
    }

    fn create_section(&self, section_name: SectionName) -> OwnedSection {
        let middle = self.width / 2.0;
        let position = match section_name {
            SectionName::Top => [middle, 100.0],
            SectionName::Bottom => [middle, self.height - 100.0],
        };
        let bounds = match section_name {
            SectionName::Top => [middle, 300.0],
            SectionName::Bottom => [middle, 300.0],
        };
        OwnedSection::default()
            .with_layout(
                Layout::default()
                    .v_align(VerticalAlign::Center)
                    .h_align(HorizontalAlign::Center),
            )
            .with_bounds(bounds)
            .with_screen_position(position)
    }

    fn create_text(text: String) -> OwnedText {
        OwnedText::new(text)
            .with_color([0.8, 0.8, 0.8, 1.0])
            .with_scale(40.0)
    }
}
