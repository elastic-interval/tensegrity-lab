use crate::camera::{Camera, Pick};
use crate::fabric::Fabric;
use crate::wgpu::fabric_renderer::FabricRenderer;
use crate::wgpu::surface_renderer::SurfaceRenderer;
use crate::wgpu::text_renderer::TextRenderer;
use crate::wgpu::Wgpu;
use crate::{ControlState, PointerChange, Radio, RenderStyle, StateChange, TestScenario};
use std::collections::HashMap;
use std::rc::Rc;
use winit::dpi::PhysicalSize;

pub struct Scene {
    wgpu: Wgpu,
    camera: Camera,
    fabric_renderer: FabricRenderer,
    surface_renderer: SurfaceRenderer,
    text_renderer: TextRenderer,
    render_style: RenderStyle,
    pick_allowed: bool,
}

impl Scene {
    pub fn new(mobile_device: bool, wgpu: Wgpu, radio: Radio) -> Self {
        let camera = wgpu.create_camera(radio);
        let fabric_renderer = wgpu.create_fabric_renderer();
        let surface_renderer = wgpu.create_surface_renderer();
        let text_renderer = wgpu.create_text_renderer(mobile_device);
        Self {
            wgpu,
            camera,
            fabric_renderer,
            surface_renderer,
            text_renderer,
            render_style: RenderStyle::Normal,
            pick_allowed: false,
        }
    }

    pub fn update_state(&mut self, state_change: StateChange) {
        use ControlState::*;
        use RenderStyle::*;
        use StateChange::*;
        use TestScenario::*;
        self.text_renderer.update_state(&state_change);
        match state_change {
            ToggleProjection => {
                self.camera.toggle_projection();
            },
            SetControlState(control_state) => match control_state {
                Waiting | UnderConstruction | Animating => self.reset(),
                Baking => self.render_style = WithAppearanceFunction(Rc::new(|_| None)),
                Viewing => {
                    self.reset();
                    self.pick_allowed = true;
                }
                ShowingJoint(_) | ShowingInterval(_) => {
                    self.pick_allowed = true;
                }
                FailureTesting(scenario) => {
                    self.reset();
                    match scenario {
                        TensionTest => self.render_style = WithPullMap(HashMap::new()),
                        CompressionTest => self.render_style = WithPushMap(HashMap::new()),
                        _ => unreachable!(),
                    }
                }
                PhysicsTesting(scenario) => {
                    self.reset();
                    match scenario {
                        PhysicsTest => {
                            self.render_style = WithAppearanceFunction(Rc::new(|_| None))
                        }
                        _ => unreachable!(),
                    }
                }
                BoxingTesting(_) => {
                    self.reset();
                }
            },
            SetAnimating(active) => self.pick_allowed = !active,
            ResetView => {
                self.render_style = Normal;
            }
            SetAppearanceFunction(appearance) => match &mut self.render_style {
                WithAppearanceFunction(_) => {
                    self.render_style = WithAppearanceFunction(appearance.clone())
                }
                _ => {
                    panic!("Cannot set color function")
                }
            },
            SetIntervalColor { key, color } => match &mut self.render_style {
                WithPullMap(map) | WithPushMap(map) => {
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
        // The camera's current_pick method already returns a reference
        self.camera.current_pick()
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let surface_texture = self.wgpu.get_surface_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self.wgpu.create_depth_view();
        let mut encoder = self.wgpu.create_encoder();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
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
        self.fabric_renderer
            .render(&mut render_pass, &self.wgpu.uniform_bind_group);
        self.surface_renderer.render(&mut render_pass);
        self.text_renderer.render(&mut render_pass, &self.wgpu);
        drop(render_pass);
        self.wgpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    pub fn redraw(&mut self, fabric: &Fabric) -> Result<(), wgpu::SurfaceError> {
        self.wgpu.update_mvp_matrix(self.camera.mvp_matrix());
        self.fabric_renderer.update_from_fabric(
            &mut self.wgpu,
            fabric,
            &self.camera.current_pick(),
            &mut self.render_style,
        );
        self.render()?;
        Ok(())
    }

    pub fn resize(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        self.wgpu.resize((width, height));
        self.camera.set_size(width as f32, height as f32);
        // the texture!
    }

    pub fn pointer_changed(&mut self, pointer_changed: PointerChange, fabric: &Fabric) {
        self.camera.pointer_changed(pointer_changed, fabric);
    }

    pub fn animate(&mut self, fabric: &Fabric) -> bool {
        self.camera.target_approach(fabric) || matches!(self.camera.current_pick(), Pick::Nothing)
    }

    pub fn normal_rendering(&mut self) {
        self.render_style = RenderStyle::Normal;
    }

    pub fn reset(&mut self) {
        self.pick_allowed = false;
        self.camera.reset();
    }
}
