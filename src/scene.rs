use crate::camera::{Camera, Framing, Pick};
use crate::wgpu::button::{Button, Buttons, Press};
use crate::fabric::quake::DECK_HEIGHT;
use crate::fabric::Fabric;
use crate::gallery::{self, Gallery};
use glam::Mat4;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, OwnedSection, OwnedText, VerticalAlign};
use crate::wgpu::surface_vertex::DeckVertex;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::parts_view::PartsView;
use crate::wgpu::shadow_renderer::ShadowRenderer;
use crate::wgpu::sky_renderer::SkyRenderer;
use crate::wgpu::surface_renderer::SurfaceRenderer;
use crate::wgpu::label_renderer::LabelRenderer;
use crate::wgpu::text_renderer::TextRenderer;
use crate::wgpu::Wgpu;
use crate::{
    ControlState, LabEvent, PickIntent, PointerChange, Radio, RenderStyle, StateChange,
    SHOW_ATTACHMENT_POINTS,
};
use std::rc::Rc;
use winit::dpi::PhysicalSize;

/// The gallery's instructions: from the window's bottom, their colour and
/// size.
const CAPTION_MARGIN: f32 = 40.0;
const CAPTION_COLOR: [f32; 4] = [0.45, 0.45, 0.45, 1.0];
const CAPTION_SIZE: f32 = 26.0;

// From the window's sides, and from its bottom, clear of the time.
const BUTTON_MARGIN: f32 = 50.0;
const BUTTON_BOTTOM: f32 = 100.0;

pub struct Scene {
    pub wgpu: Wgpu,
    camera: Camera,
    sky_renderer: SkyRenderer,
    fabric_renderer: FabricRenderer,
    surface_renderer: SurfaceRenderer,
    shadow_renderer: ShadowRenderer,
    text_renderer: TextRenderer,
    label_renderer: LabelRenderer,
    buttons: Buttons<LabEvent>,
    render_style: RenderStyle,
    show_attachment_points: bool,
    /// White, or the starry sky (B).
    white: bool,
    /// The parts view while it has the window, and the part it last showed.
    parts_view: Option<PartsView>,
    last_part: usize,
    pick_allowed: bool,
    /// No picking while the wobble table evolves and plays.
    quaking: bool,
    model_scale: Option<f32>,
    radio: Radio,
}

impl Scene {
    pub fn new(mobile_device: bool, wgpu: Wgpu, radio: Radio, model_scale: Option<f32>) -> Self {
        let camera = wgpu.create_camera();
        let sky_renderer = wgpu.create_sky_renderer();
        let fabric_renderer = wgpu.create_fabric_renderer();
        let surface_renderer = wgpu.create_surface_renderer();
        let shadow_renderer = ShadowRenderer::new(&wgpu);
        let text_renderer = wgpu.create_text_renderer(mobile_device, model_scale);
        let label_renderer = LabelRenderer::new(&wgpu);
        let buttons = Buttons::new(&wgpu);
        let render_style = RenderStyle::Normal;
        SHOW_ATTACHMENT_POINTS.with(|cell| {
            *cell.borrow_mut() = false;
        });

        Self {
            wgpu,
            camera,
            sky_renderer,
            fabric_renderer,
            surface_renderer,
            shadow_renderer,
            text_renderer,
            label_renderer,
            buttons,
            render_style,
            show_attachment_points: false,
            white: false,
            parts_view: None,
            last_part: 0,
            pick_allowed: false,
            quaking: false,
            model_scale,
            radio,
        }
    }

    fn toggle_attachment_points(&mut self) {
        self.show_attachment_points = !self.show_attachment_points;
        SHOW_ATTACHMENT_POINTS.with(|cell| {
            *cell.borrow_mut() = self.show_attachment_points;
        });
    }

    pub fn toggle_parts_view(&mut self, fabric: &Fabric) {
        match self.parts_view.take() {
            Some(view) => self.last_part = view.chosen(),
            None => self.parts_view = PartsView::new(&self.wgpu, fabric, self.last_part, self.white),
        }
    }

    /// The buttons along the bottom, above the time.
    pub fn set_buttons(&mut self, buttons: Vec<Vec<Button<LabEvent>>>) {
        let (width, height) = self.wgpu.size();
        let text_renderer = &mut self.text_renderer;
        let sections = self.buttons.lay_out(
            &self.wgpu,
            &mut |section| text_renderer.glyph_bounds(section),
            buttons,
            BUTTON_MARGIN,
            width - BUTTON_MARGIN,
            height - BUTTON_BOTTOM,
            self.white,
        );
        self.text_renderer.set_buttons(sections);
    }

    pub fn parts_view_showing(&self) -> bool {
        self.parts_view.is_some()
    }

    pub fn update_state(&mut self, state_change: StateChange) {
        use ControlState::*;
        use RenderStyle::*;
        use StateChange::*;
        self.text_renderer.update_state(&state_change);
        match state_change {
            ToggleProjection => {
                self.camera.toggle_projection();
            }
            ToggleBackground => {
                self.white = !self.white;
                self.surface_renderer.set_white(&self.wgpu.queue, self.white);
                if let Some(view) = &mut self.parts_view {
                    view.set_white(&self.wgpu, self.white);
                }
            }
            ToggleAttachmentPoints => {
                // In model-scale mode, attachment points are not available
                if self.model_scale.is_none() {
                    self.toggle_attachment_points();
                }
            }
            SetControlState(control_state) => match control_state {
                Waiting | Building => {
                    self.parts_view = None;
                    self.reset();
                    self.surface_renderer.reset_radius(&self.wgpu.queue);
                }
                Animating { .. } => {
                    self.reset();
                    self.pick_allowed = true;
                }
                // No picking: parts come and go.
                Assembling { .. } => {
                    self.reset();
                }
                Baking => {
                    self.render_style = WithAppearanceFunction {
                        function: Rc::new(|_| None),
                    }
                }
                Viewing { .. } => {
                    self.reset();
                    self.pick_allowed = true;
                }
                ShowingJoint(_) => {
                    self.pick_allowed = true;
                }
                ShowingInterval(_) => {
                    self.pick_allowed = true;
                }
                PhysicsTesting => {
                    self.reset();
                    self.render_style = WithAppearanceFunction {
                        function: Rc::new(|_| None),
                    }
                }
            },
            SetQuaking(quaking) => {
                let starting = quaking.is_some() && !self.quaking;
                self.quaking = quaking.is_some();
                if starting && !matches!(self.camera.current_pick(), Pick::Nothing) {
                    LabEvent::Crucible(crate::CrucibleAction::ClearSelection).send(&self.radio);
                }
            }
            SetAnimating(_) => {}
            ResetView => {
                self.render_style = Normal;
            }
            RestartApproach => {
                self.camera.restart_approach();
            }
            ToggleColorByRole => {
                self.render_style = match &self.render_style {
                    ColorByRole => Normal,
                    _ => ColorByRole,
                };
            }
            SetAppearanceFunction(appearance) => match &mut self.render_style {
                WithAppearanceFunction { .. } => {
                    self.render_style = WithAppearanceFunction {
                        function: appearance.clone(),
                    }
                }
                _ => {
                    panic!("Cannot set color function")
                }
            },
            SetIntervalColor { key, color } => match &mut self.render_style {
                WithPullMap { map, .. } => {
                    map.insert(key, color);
                }
                WithPushMap { map, .. } => {
                    map.insert(key, color);
                }
                _ => {
                    panic!("Cannot set interval color")
                }
            },
            _ => {}
        }
    }

    pub fn pick_allowed(&self) -> bool {
        self.pick_allowed
    }

    /// Returns the current pick state from the camera
    pub fn current_pick(&self) -> &Pick {
        &self.camera.current_pick()
    }

    pub fn render_style_shows_attachment_points(&self) -> bool {
        self.show_attachment_points
    }

    /// With `chrome`, the sky behind and the highlight, labels, buttons and
    /// text over the fabric; with `shadows`, the gallery's shadow maps first,
    /// as its spots see.
    fn render(&mut self, show_surface: bool, chrome: bool, shadows: Option<[Mat4; 3]>) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.wgpu.get_surface_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.wgpu.create_depth_view();
        let mut encoder = self.wgpu.create_encoder();
        if let Some(views) = shadows {
            self.shadow_renderer.render(&self.wgpu, &mut encoder, &self.fabric_renderer, views);
        }

        // First pass: render sky (no depth testing, clears the screen), or
        // just white.
        {
            let clear = if self.white { 1.0 } else { 0.0 };
            let mut sky_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sky Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear,
                            g: clear,
                            b: clear,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                multiview_mask: None,
                depth_stencil_attachment: None, // No depth for sky
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if !self.white && chrome {
                self.sky_renderer.render(&mut sky_pass);
            }
        }

        // Second pass: render everything else with depth testing
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Keep sky background
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            multiview_mask: None,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.wgpu.set_bind_group(&mut render_pass);
        self.fabric_renderer.render(
            &mut render_pass,
            &self.wgpu.uniform_bind_group,
            self.show_attachment_points,
        );
        // Only render surface when gravity is present
        if show_surface {
            self.surface_renderer.render(&mut render_pass);
        }
        self.surface_renderer.render_deck(&mut render_pass);
        if chrome {
            self.fabric_renderer
                .render_highlight(&mut render_pass, &self.wgpu.uniform_bind_group);
            self.label_renderer.render(&mut render_pass);
            self.buttons.render(&mut render_pass);
            self.text_renderer.render(&mut render_pass, &self.wgpu);
        } else {
            let (width, height) = self.wgpu.size();
            let caption = OwnedSection::default()
                .with_layout(Layout::default().h_align(HorizontalAlign::Center).v_align(VerticalAlign::Bottom))
                .with_screen_position([width / 2.0, height - CAPTION_MARGIN])
                .with_bounds([width, height])
                .add_text(OwnedText::new(gallery::INSTRUCTIONS).with_color(CAPTION_COLOR).with_scale(CAPTION_SIZE));
            self.text_renderer.render_caption(&mut render_pass, &self.wgpu, &caption);
        }
        drop(render_pass);

        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    pub fn redraw(
        &mut self,
        fabric: &Fabric,
        has_surface: bool,
        delta_seconds: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        if let Some(view) = &mut self.parts_view {
            return view.render(&self.wgpu);
        }
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.wgpu.update_eye(self.camera.eye());
        self.sky_renderer
            .update_time(&self.wgpu.queue, delta_seconds);
        self.fabric_renderer.update(
            &mut self.wgpu,
            fabric,
            &self.camera.current_pick(),
            &self.render_style,
            self.show_attachment_points,
        );
        let preview = if self.pick_allowed && !self.quaking && !cfg!(target_arch = "wasm32") {
            self.camera.preview()
        } else {
            None
        };
        self.fabric_renderer.update_highlight(
            &self.wgpu,
            fabric,
            self.camera.current_pick(),
            preview,
            self.white,
            self.show_attachment_points,
        );
        let no_pick = Pick::Nothing;
        self.label_renderer.update(
            &self.wgpu,
            &mut self.text_renderer,
            fabric,
            if self.pick_allowed { self.camera.current_pick() } else { &no_pick },
            self.camera.mvp_matrix(),
            self.camera.screen_up(),
            self.model_scale.unwrap_or(1.0),
            self.white,
            self.show_attachment_points,
        );
        self.surface_renderer.set_deck(&self.wgpu.device, &self.wgpu.queue, fabric.deck());
        // Update surface size based on fabric bounding radius
        if has_surface {
            self.surface_renderer
                .update_radius(&self.wgpu.queue, fabric.bounding_radius());
        }
        self.render(has_surface, true, None)?;
        Ok(())
    }

    /// The gallery as the visitor sees it, and nothing else.
    pub fn redraw_gallery(&mut self, gallery: &Gallery, delta_seconds: f32) -> Result<(), wgpu::SurfaceError> {
        let (width, height) = self.wgpu.size();
        self.wgpu.update_mvp_matrix(gallery.walker.mvp(width, height));
        self.wgpu.update_eye(gallery.walker.eye());
        self.sky_renderer.update_time(&self.wgpu.queue, delta_seconds);
        let placed: Vec<_> = gallery.pieces.iter().map(|piece| (&piece.fabric, piece.place)).collect();
        self.fabric_renderer.update_placed(&self.wgpu, &placed);
        self.wgpu.update_lights(gallery.lights());
        let spots = gallery.spots();
        self.wgpu.update_spots(spots.views, spots.places, spots.aims);
        let mut vertices = DeckVertex::room(gallery::WIDTH, gallery::DEPTH, gallery::WALL_HEIGHT, -DECK_HEIGHT);
        for piece in &gallery.pieces {
            if let Some(deck) = piece.fabric.deck() {
                let mut table = DeckVertex::for_radius(deck.radius, DECK_HEIGHT);
                table.extend(DeckVertex::skirt(deck.radius, DECK_HEIGHT));
                vertices.extend(DeckVertex::placed(&table, deck, piece.place));
            }
        }
        self.surface_renderer.set_gallery(&self.wgpu.device, &vertices);
        self.render(false, false, Some(spots.views))
    }

    pub fn resize(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        self.wgpu.resize((width, height));
        self.camera.set_size(width as f32, height as f32);
        if let Some(view) = &mut self.parts_view {
            view.resize(&self.wgpu);
        }
    }

    /// What a button pressed asks for.
    pub fn pointer_changed(&mut self, pointer_changed: PointerChange, fabric: &Fabric) -> Option<LabEvent> {
        if let Some(view) = &mut self.parts_view {
            let leave = view.pointer_changed(pointer_changed, &self.wgpu);
            return leave.then_some(LabEvent::UpdateState(StateChange::TogglePartsView));
        }
        match self.buttons.pointer_changed(&pointer_changed) {
            Press::Button(event) => return Some(event),
            Press::Held => return None,
            Press::Free => {}
        }
        // When picking is not allowed (or always on the web — pure mouse-driven, no
        // selection), convert pick intents to Reset (release without picking). Camera
        // orbit/zoom (Moved/Pressed/Zoomed) still pass through.
        let pointer_changed = if !self.pick_allowed || self.quaking || cfg!(target_arch = "wasm32") {
            match pointer_changed {
                PointerChange::Released(_) => PointerChange::Released(PickIntent::Reset),
                PointerChange::TouchReleased(_) => PointerChange::TouchReleased(PickIntent::Reset),
                other => other,
            }
        } else {
            pointer_changed
        };

        if let Some(control_state) = self.camera.pointer_changed(pointer_changed, fabric) {
            control_state.send(&self.radio);
        }
        None
    }

    pub fn update_camera(&mut self, dt: f32, fabric: &Fabric, framing: Framing) -> bool {
        self.camera.update(dt, fabric, framing)
            || matches!(self.camera.current_pick(), Pick::Nothing)
            || self.parts_view.is_some()
    }

    pub fn normal_rendering(&mut self) {
        self.render_style = RenderStyle::Normal;
        self.show_attachment_points = false;
    }

    pub fn reset(&mut self) {
        self.pick_allowed = false;
        self.camera.reset();
    }

    pub fn back_to_joint(&mut self, fabric: &Fabric) {
        if let Some(control_state) = self.camera.back_to_joint(fabric) {
            control_state.send(&self.radio);
        }
    }

    /// Jump camera to ideal viewing position for the given fabric
    pub fn jump_to_fabric(&mut self, fabric: &Fabric) {
        self.camera.jump_to_fabric(fabric);
    }

    pub fn resume_camera_following(&mut self) {
        self.camera.resume_following();
    }

    pub fn restart_approach(&mut self) {
        self.camera.restart_approach();
    }

    /// A drag is in progress (pointer or touch held down).
    pub fn camera_dragging(&self) -> bool {
        self.camera.is_dragging()
    }

    /// Position camera for watching a sphere drop. Places the camera
    /// at ground level, far enough back to see the full drop.
    pub fn position_camera_for_drop(&mut self, radius: f32) {
        use glam::Vec3;
        // Sphere center starts at ~2*radius. Camera at ground level,
        // far enough to see the whole sphere + its drop path.
        let distance = radius * 5.0;
        let camera_pos = Vec3::new(distance, radius * 0.2, 0.0);
        let look_at = Vec3::new(0.0, radius, 0.0);
        self.camera.set_position_and_hold(camera_pos, look_at);
    }

    /// Get camera view for export (position, look_at)
    pub fn export_view(&self) -> (glam::Vec3, glam::Vec3) {
        self.camera.export_view()
    }
}
