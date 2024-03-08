use std::f32::consts::PI;

use cgmath::num_traits::abs;
use cgmath::{
    perspective, point3, vec3, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rad,
    Rotation, Rotation3, SquareMatrix, Transform, Vector3,
};
use leptos::{SignalUpdate, WriteSignal};
use winit::event::MouseButton;
use winit_input_helper::WinitInputHelper;

use crate::control_overlay;
use crate::fabric::{Fabric, UniqueId};

const TARGET_ATTRACTION: f32 = 0.01;

pub struct Camera {
    pub position: Point3<f32>,
    pub target: Target,
    pub look_at: Point3<f32>,
    pub picked_interval: Option<UniqueId>,
    pub width: f32,
    pub height: f32,
    pub pick_mode: bool,
    pub pick_cursor: Option<(f32, f32)>,
    pub set_control_state: Option<WriteSignal<control_overlay::ControlState>>,
}

impl Camera {
    pub fn new(
        position: Point3<f32>,
        width: f32,
        height: f32,
        set_control_state: Option<WriteSignal<control_overlay::ControlState>>,
    ) -> Self {
        Self {
            position,
            set_control_state,
            target: Target::default(),
            look_at: point3(0.0, 3.0, 0.0),
            picked_interval: None,
            width,
            height,
            pick_mode: false,
            pick_cursor: None,
        }
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper, fabric: &Fabric) {
        if input.mouse_held(MouseButton::Left) {
            if let Some(rotation) = self.rotation(input.mouse_diff()) {
                self.position =
                    self.look_at - rotation.transform_vector(self.look_at - self.position);
                self.pick_cursor = None;
            }
        }
        self.pick_mode = false; // TODO
        if input.mouse_pressed(MouseButton::Left) {
            self.pick_cursor = input.cursor();
        }
        if input.mouse_released(MouseButton::Left) {
            if let Some(pick_cursor) = self.pick_cursor {
                self.pick(pick_cursor, fabric)
            }
        }
        let (_sx, sy) = input.scroll_diff();
        if abs(sy) > 0.1 {
            let scroll = sy * SPEED.z;
            let gaze = self.look_at - self.position;
            if gaze.magnitude() - scroll > 1.0 {
                self.position += gaze.normalize() * scroll;
            }
        }
    }

    pub fn target_approach(&mut self, fabric: &Fabric) {
        let look_at = self.target.look_at(fabric);
        self.look_at += (look_at - self.look_at) * TARGET_ATTRACTION;
        let gaze = (self.look_at - self.position).normalize();
        let up_dot_gaze = Vector3::unit_y().dot(gaze);
        if !(-0.9..=0.9).contains(&up_dot_gaze) {
            let axis = Vector3::unit_y().cross(gaze).normalize();
            self.position = Point3::from_vec(
                Quaternion::from_axis_angle(axis, Rad(0.01 * up_dot_gaze / abs(up_dot_gaze)))
                    .rotate_vector(self.position.to_vec()),
            );
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn mvp_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn pick(&mut self, (px, py): (f32, f32), fabric: &Fabric) {
        let width = self.width / 2.0;
        let height = self.height / 2.0;
        let x = (px - width) / width;
        let y = (height - py) / height;
        let position = Point3::new(x, y, 1.0);
        let point3d = self
            .mvp_matrix()
            .invert()
            .unwrap()
            .transform_point(position);
        let ray = (point3d - self.position).normalize();
        let best = fabric
            .intervals
            .iter()
            .map(|(interval_id, interval)| {
                (
                    interval_id,
                    (interval.midpoint(&fabric.joints).to_vec() - self.position.to_vec())
                        .normalize()
                        .dot(ray),
                )
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b));
        if let Some((interval_id, _)) = best {
            self.picked_interval = Some(*interval_id);
            let interval = fabric.interval(*interval_id);
            if let Some(set_control_state) = self.set_control_state {
                set_control_state.update(move |state| {
                    state.picked_interval = Some(*interval);
                });
            }
        }
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.look_at, Vector3::unit_y())
    }

    fn projection_matrix(&self) -> Matrix4<f32> {
        let aspect = self.width / self.height;
        OPENGL_TO_WGPU_MATRIX * perspective(Rad(2.0 * PI / 5.0), aspect, 0.1, 100.0)
    }

    fn rotation(&self, (dx, dy): (f32, f32)) -> Option<Matrix4<f32>> {
        if (dx, dy) == (0.0, 0.0) {
            return None;
        }
        let rot_x = Matrix4::from_axis_angle(Vector3::unit_y(), Deg(dx * SPEED.x));
        let axis = Vector3::unit_y().cross((self.look_at - self.position).normalize());
        let rot_y = Matrix4::from_axis_angle(axis, Deg(dy * SPEED.y));
        Some(rot_x * rot_y)
    }
}

const SPEED: Vector3<f32> = vec3(-0.5, 0.4, 1.0);

const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
);

#[derive(Clone, Debug, Default)]
pub enum Target {
    Origin,
    #[default]
    FabricMidpoint,
    AroundInterval(UniqueId),
}

impl Target {
    pub fn look_at(&self, fabric: &Fabric) -> Point3<f32> {
        match self {
            Target::Origin => point3(0.0, 0.0, 0.0),
            Target::FabricMidpoint => fabric.midpoint(),
            Target::AroundInterval(interval_id) => {
                fabric.interval(*interval_id).midpoint(&fabric.joints)
            }
        }
    }
}
