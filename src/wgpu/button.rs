//! Buttons on screen, for the mouse and the touch screen alike. A row of
//! labelled ones is laid out and drawn here, its text by the caller's
//! brush; any other rect can be a button too, drawn by its owner.

use crate::wgpu::label_renderer::{
    create_box_pipeline, push_box, BoxVertex, BORDER_WIDTH, CORNER_RADIUS, ON_DARK, ON_LIGHT,
};
use crate::wgpu::Wgpu;
use crate::PointerChange;
use glam::Vec2;
use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::Rect;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};

// In physical pixels.
const FONT_SIZE: f32 = 32.0;
const PADDING: Vec2 = Vec2::new(22.0, 16.0);
const GAP: f32 = 14.0;

/// A button to show: what it says and what pressing it does, or no action
/// for a caption between buttons. A disabled one is drawn dimmed and does
/// nothing, keeping its place.
pub struct Button<T> {
    pub label: String,
    pub action: Option<T>,
    pub enabled: bool,
}

impl<T> Button<T> {
    pub fn new(label: impl Into<String>, action: T) -> Self {
        Self { label: label.into(), action: Some(action), enabled: true }
    }

    pub fn caption(label: impl Into<String>) -> Self {
        Self { label: label.into(), action: None, enabled: true }
    }

    pub fn disabled(self) -> Self {
        Self { enabled: false, ..self }
    }
}

/// How much of its colour a disabled button keeps.
const DISABLED: f32 = 0.35;

/// What a pointer change did to the buttons.
pub enum Press<T> {
    /// Released over a button that was pressed.
    Button(T),
    /// Part of a press on a button, for nothing else to see.
    Held,
    /// For whatever lies under the buttons.
    Free,
}

pub struct Buttons<T> {
    pipeline: wgpu::RenderPipeline,
    vertices: Option<(wgpu::Buffer, u32)>,
    /// Each button's centre and half size, and what it does.
    hits: Vec<(Vec2, Vec2, T)>,
    pointer: Option<Vec2>,
    held: bool,
}

impl<T: Clone> Buttons<T> {
    pub fn new(wgpu: &Wgpu) -> Self {
        Self { pipeline: create_box_pipeline(wgpu), vertices: None, hits: Vec::new(), pointer: None, held: false }
    }

    /// Lay `groups` of buttons out in rows centred between `left` and
    /// `right`, the last row ending at `bottom`, each group within a row, and
    /// return their text to draw. Clears any other buttons.
    pub fn lay_out(
        &mut self,
        wgpu: &Wgpu,
        measure: &mut dyn FnMut(&OwnedSection) -> Option<Rect>,
        groups: Vec<Vec<Button<T>>>,
        left: f32,
        right: f32,
        bottom: f32,
        white: bool,
    ) -> Vec<OwnedSection> {
        let palette = if white { &ON_LIGHT } else { &ON_DARK };
        let text_in = |label: &str, center: Vec2, enabled: bool| {
            let mut color = palette.text;
            if !enabled {
                color[3] *= DISABLED;
            }
            OwnedSection::default()
                .with_layout(Layout::default().h_align(HorizontalAlign::Center).v_align(VerticalAlign::Center))
                .with_screen_position((center.x, center.y))
                .add_text(OwnedText::new(label).with_scale(FONT_SIZE).with_color(color))
        };
        let text = |label: &str, center: Vec2| text_in(label, center, true);
        let border = Vec2::splat(BORDER_WIDTH);
        let mut rows: Vec<Vec<(Button<T>, Vec2)>> = vec![Vec::new()];
        let mut row_width = 0.0;
        for group in groups {
            let sized: Vec<(Button<T>, Vec2)> = group
                .into_iter()
                .filter_map(|button| {
                    let bounds = measure(&text(&button.label, Vec2::ZERO))?;
                    let half = Vec2::new(bounds.width(), FONT_SIZE) / 2.0 + PADDING + border;
                    Some((button, half))
                })
                .collect();
            let width: f32 = sized.iter().map(|(_, half)| 2.0 * half.x + GAP).sum();
            if row_width + width > right - left && !rows.last().unwrap().is_empty() {
                rows.push(Vec::new());
                row_width = 0.0;
            }
            row_width += width;
            rows.last_mut().unwrap().extend(sized);
        }
        self.hits.clear();
        let mut vertices: Vec<BoxVertex> = Vec::new();
        let mut sections = Vec::new();
        let (width, height) = wgpu.size();
        let row_height = 2.0 * (FONT_SIZE / 2.0 + PADDING.y + BORDER_WIDTH) + GAP;
        let mut middle = bottom - row_height * rows.len() as f32 + row_height / 2.0;
        for row in rows {
            let total: f32 = row.iter().map(|(_, half)| 2.0 * half.x + GAP).sum::<f32>() - GAP;
            let mut x = (left + right - total) / 2.0;
            for (button, half) in row {
                let center = Vec2::new(x + half.x, middle);
                x += 2.0 * half.x + GAP;
                sections.push(text_in(&button.label, center, button.enabled));
                let Some(action) = button.action else {
                    continue;
                };
                let mut leader = palette.leader;
                if !button.enabled {
                    leader[3] *= DISABLED;
                }
                push_box(&mut vertices, center, half, CORNER_RADIUS + BORDER_WIDTH, leader, width, height);
                push_box(&mut vertices, center, half - border, CORNER_RADIUS, palette.background, width, height);
                if button.enabled {
                    self.hits.push((center, half, action));
                }
            }
            middle += row_height;
        }
        self.vertices = (!vertices.is_empty()).then(|| {
            let buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Button Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            (buffer, vertices.len() as u32)
        });
        sections
    }

    /// A button its owner draws, at `center` and `half` its size.
    pub fn add(&mut self, center: Vec2, half: Vec2, action: T) {
        self.hits.push((center, half, action));
    }

    /// The top of the highest button, if there are any.
    pub fn top(&self) -> Option<f32> {
        self.hits.iter().map(|(center, half, _)| center.y - half.y).reduce(f32::min)
    }

    /// What the button at `point` does, if one is there.
    pub fn at(&self, point: Vec2) -> Option<T> {
        let (_, _, action) = self.hits.iter().find(|(center, half, _)| (point - *center).abs().cmple(*half).all())?;
        Some(action.clone())
    }

    fn under_pointer(&self) -> Option<T> {
        let pointer = self.pointer?;
        let (_, _, action) =
            self.hits.iter().find(|(center, half, _)| (pointer - *center).abs().cmple(*half).all())?;
        Some(action.clone())
    }

    /// A button acts when released over it, so the press and release both
    /// stay here, whatever else listens to the pointer.
    pub fn pointer_changed(&mut self, change: &PointerChange) -> Press<T> {
        match change {
            PointerChange::Moved(position) => {
                self.pointer = Some(Vec2::new(position.x as f32, position.y as f32));
                Press::Free
            }
            PointerChange::TouchPressed(position) => {
                self.pointer = Some(Vec2::new(position.x as f32, position.y as f32));
                self.press()
            }
            PointerChange::Pressed => self.press(),
            PointerChange::Released(_) | PointerChange::TouchReleased(_) if self.held => {
                self.held = false;
                self.under_pointer().map_or(Press::Held, Press::Button)
            }
            _ => Press::Free,
        }
    }

    fn press(&mut self) -> Press<T> {
        self.held = self.under_pointer().is_some();
        if self.held {
            Press::Held
        } else {
            Press::Free
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let Some((vertices, count)) = &self.vertices else {
            return;
        };
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.draw(0..*count, 0..1);
    }
}
