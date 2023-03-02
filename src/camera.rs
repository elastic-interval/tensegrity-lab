use std::collections::HashSet;
use std::f32::consts::PI;

use cgmath::{Deg, EuclideanSpace, InnerSpace, Matrix4, perspective, Point3, point3, Quaternion, Rad, Rotation, Rotation3, SquareMatrix, Transform, vec3, Vector3};
use cgmath::num_traits::abs;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};

use crate::fabric::{Fabric, UniqueId};

const TARGET_ATTRACTION: f32 = 0.01;
const TARGET_DISTANCE_MARGIN: f32 = 0.3;

#[derive(Clone, Debug)]
pub struct Pick {
    pub face_id: UniqueId,
    pub multiple: bool,
}

impl Pick {
    pub fn just(face_id: UniqueId) -> Self {
        Self { face_id, multiple: false }
    }
}

pub struct Camera {
    pub position: Point3<f32>,
    pub target: Target,
    pub look_at: Point3<f32>,
    pub picked: Option<Pick>,
    pub size: PhysicalSize<f64>,
    pub moving_mouse: PhysicalPosition<f64>,
    pub pick_mode: bool,
    pub multiple: bool,
    pub pressed_mouse: Option<PhysicalPosition<f64>>,
}

impl Camera {
    pub fn new(position: Point3<f32>, size: PhysicalSize<f64>) -> Self {
        Self {
            position,
            target: Target::default(),
            look_at: point3(0.0, 3.0, 0.0),
            picked: None,
            size,
            moving_mouse: PhysicalPosition::new(0.0, 0.0),
            pressed_mouse: None,
            pick_mode: false,
            multiple: false,
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent, fabric: &Fabric) {
        match event {
            WindowEvent::ModifiersChanged(state) => {
                self.pick_mode = state.logo();
                self.multiple = state.shift();
            }
            WindowEvent::MouseInput { state, .. } => {
                match state {
                    ElementState::Pressed if self.pick_mode => { self.pick(self.moving_mouse, self.multiple, fabric) }
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
        self.look_at += (look_at - self.look_at) * TARGET_ATTRACTION;
        let gaze = (self.look_at - self.position).normalize();
        let up_dot_gaze = Vector3::unit_y().dot(gaze);
        if !(-0.9..=0.9).contains(&up_dot_gaze) {
            let axis = Vector3::unit_y().cross(gaze).normalize();
            self.position = Point3::from_vec(
                Quaternion::from_axis_angle(axis, Rad(0.01 * up_dot_gaze / abs(up_dot_gaze)))
                    .rotate_vector(self.position.to_vec())
            );
        }
    }

    pub fn set_size(&mut self, size: PhysicalSize<f64>) {
        self.size = size;
    }

    pub fn mvp_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn pick(&mut self, position: PhysicalPosition<f64>, multiple: bool, fabric: &Fabric) {
        let width = self.size.width / 2.0;
        let height = self.size.height / 2.0;
        let x = (position.x - width) / width;
        let y = (height - position.y) / height;
        let position = Point3::new(x as f32, y as f32, 1.0);
        let point3d = self.mvp_matrix().invert().unwrap().transform_point(position);
        let ray = (point3d - self.position).normalize();
        let best = fabric.faces.iter()
            .map(|(face_id, face)|
                (face_id, (face.midpoint(fabric) - self.position.to_vec()).normalize().dot(ray)))
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b));
        if let Some((face_id, _)) = best {
            self.picked = Some(Pick { face_id: *face_id, multiple });
        }
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.look_at, Vector3::unit_y())
    }

    fn projection_matrix(&self) -> Matrix4<f32> {
        let aspect = self.size.width as f32 / self.size.height as f32;
        OPENGL_TO_WGPU_MATRIX * perspective(Rad(2.0 * PI / 5.0), aspect, 0.1, 100.0)
    }

    fn rotation(&self) -> Option<Matrix4<f32>> {
        let (dx, dy) = self.angles()?;
        let rot_x = Matrix4::from_axis_angle(Vector3::unit_y(), dx);
        let axis = Vector3::unit_y().cross((self.look_at - self.position).normalize());
        let rot_y = Matrix4::from_axis_angle(axis, dy);
        Some(rot_x * rot_y)
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

#[derive(Clone, Debug, Default)]
pub enum Target {
    Origin,
    #[default]
    FabricMidpoint,
    AroundFaces(HashSet<UniqueId>),
}

impl Target {
    pub fn look_at(&self, fabric: &Fabric) -> Option<Point3<f32>> {
        match self {
            Target::Origin => Some(point3(0.0, 0.0, 0.0)),
            Target::FabricMidpoint => Some(fabric.midpoint()),
            Target::AroundFaces(face_set) => {
                let midpoints = face_set
                    .iter()
                    .filter_map(|face_id|
                        fabric.faces
                            .get(face_id)
                            .map(|face| face.midpoint(fabric)));
                let count = midpoints.clone().count();
                (count > 0).then_some(
                    Point3::from_vec(midpoints.sum::<Vector3<f32>>() / (count as f32))
                )
            }
        }
    }
}
