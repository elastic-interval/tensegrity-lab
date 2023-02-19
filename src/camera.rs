use std::f32::consts::PI;

use cgmath::{Deg, InnerSpace, Matrix4, MetricSpace, perspective, Point3, point3, Rad, Transform, vec3, Vector3};
use cgmath::num_traits::abs;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

use crate::fabric::{Fabric, UniqueId};

const TARGET_ATTRACTION: f32 = 0.01;
const UP_ATTRACTION: f32 = 0.1;
const TARGET_DOT_UP: f32 = 0.15;
const TARGET_DISTANCE_MARGIN: f32 = 0.3;

pub struct Camera {
    pub position: Point3<f32>,
    pub target: Target,
    pub look_at: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub moving_mouse: PhysicalPosition<f64>,
    pub pressed_mouse: Option<PhysicalPosition<f64>>,
}

impl Camera {
    pub fn new(position: Point3<f32>, aspect: f32) -> Self {
        Self {
            position,
            target: Target::default(),
            look_at: point3(0.0, 3.0, 0.0),
            up: Vector3::unit_y(),
            aspect,
            moving_mouse: PhysicalPosition::new(0.0, 0.0),
            pressed_mouse: None,
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::MouseInput { state, .. } => {
                match state {
                    ElementState::Pressed => { self.pressed_mouse = Some(self.moving_mouse) }
                    ElementState::Released => { self.pressed_mouse = None }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.moving_mouse = *position;
                if let Some(rotation) = self.rotation() {
                    self.position = self.look_at - rotation.transform_vector(self.look_at - self.position);
                    self.pressed_mouse = Some(self.moving_mouse);
                }
            }
            WindowEvent::MouseWheel { delta: MouseScrollDelta::PixelDelta(pos), .. } => {
                let scroll = pos.y as f32 * SPEED.z;
                let gaze = self.look_at - self.position;
                if gaze.magnitude() - scroll > 1.0 {
                    self.position += gaze.normalize() * scroll;
                }
            }
            _ => {}
        }
    }

    pub fn target_approach(&mut self, fabric: &Fabric) {
        let Some(look_at) = self.target.look_at(fabric) else {
            return;
        };
        let Some(up) = self.target.up(fabric) else {
            return;
        };
        self.up = (self.up + up * TARGET_ATTRACTION) / (1.0 + TARGET_ATTRACTION);
        self.look_at += (look_at - self.look_at) * TARGET_ATTRACTION;
        if let Some(distance) = self.target.distance(fabric) {
            let current = self.position.distance(self.look_at);
            if abs(current - distance) > TARGET_DISTANCE_MARGIN {
                let new_distance = (current + distance * TARGET_ATTRACTION) / (1.0 + TARGET_ATTRACTION);
                self.position = self.look_at + (self.position - self.look_at).normalize() * new_distance
            }
        }
        let dot_up = TARGET_DOT_UP - (self.position - self.look_at).normalize().dot(self.up);
        self.position += self.up * UP_ATTRACTION * dot_up;
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    pub fn mvp_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.look_at, self.up)
    }

    fn projection_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(Rad(2.0 * PI / 5.0), self.aspect, 0.1, 100.0)
    }

    fn rotation(&self) -> Option<Matrix4<f32>> {
        let (dx, dy) = self.angles()?;
        let rot_x = Matrix4::from_axis_angle(self.up, dx);
        if self.target.allow_vertical_rotation() {
            let axis = Vector3::unit_y().cross((self.look_at - self.position).normalize());
            let rot_y = Matrix4::from_axis_angle(axis, dy);
            Some(rot_x * rot_y)
        } else {
            Some(rot_x)
        }
    }

    fn angles(&self) -> Option<(Deg<f32>, Deg<f32>)> {
        let pressed = self.pressed_mouse?;
        let PhysicalPosition { x, y } = self.moving_mouse;
        let dx = (pressed.x - x) as f32;
        let dy = (y - pressed.y) as f32;
        Some((Deg(dx * SPEED.x), Deg(dy * SPEED.y)))
    }
}

const SPEED: Vector3<f32> = vec3(0.5, 0.4, 0.01);

const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Copy, Clone, Debug, Default)]
pub enum Target {
    Origin,
    #[default]
    FabricMidpoint,
    SelectedFace(UniqueId),
}

impl Target {
    pub fn look_at(&self, fabric: &Fabric) -> Option<Point3<f32>> {
        match self {
            Target::Origin => Some(point3(0.0, 0.0, 0.0)),
            Target::FabricMidpoint => Some(fabric.midpoint()),
            Target::SelectedFace(face_id) => {
                fabric.faces.get(face_id).map(|face| {
                    let (_, midpoint, _) = face.visible_points(fabric);
                    midpoint
                })
            }
        }
    }

    pub fn up(&self, fabric: &Fabric) -> Option<Vector3<f32>> {
        match self {
            Target::Origin | Target::FabricMidpoint => Some(Vector3::unit_y()),
            Target::SelectedFace(face_id) => Some(fabric.face(*face_id).normal(fabric)),
        }
    }

    pub fn distance(&self, fabric: &Fabric) -> Option<f32> {
        match self {
            Target::Origin | Target::FabricMidpoint => None,
            Target::SelectedFace(face_id) => Some(fabric.face(*face_id).scale * 7.0)
        }
    }

    pub fn allow_vertical_rotation(&self) -> bool {
        match self {
            Target::Origin | Target::FabricMidpoint => true,
            Target::SelectedFace(_) => false
        }
    }
}
