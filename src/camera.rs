use std::f32::consts::PI;

use cgmath::{
    Deg, EuclideanSpace, InnerSpace, Matrix4, perspective, point3, Point3, Quaternion, Rad, Rotation,
    Rotation3, SquareMatrix, Transform, vec3, Vector3,
};
use cgmath::num_traits::abs;
use winit::event::MouseButton;
use winit_input_helper::WinitInputHelper;

use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::Interval;

#[derive(Debug, Clone)]
pub enum Pick {
    Nothing,
    Joint(usize),
    Interval {
        joint: usize,
        id: UniqueId,
        interval: Interval,
    },
}

const TARGET_ATTRACTION: f32 = 0.01;

pub struct Camera {
    pub position: Point3<f32>,
    pub target: Target,
    pub look_at: Point3<f32>,
    pub width: f32,
    pub height: f32,
    pub pick_cursor: Option<(f32, f32)>,
    pub pick: Pick,
}

impl Camera {
    pub fn new(
        position: Point3<f32>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            position,
            target: Target::default(),
            look_at: point3(0.0, 3.0, 0.0),
            width,
            height,
            pick_cursor: None,
            pick: Pick::Nothing,
        }
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper, fabric: &Fabric) -> Option<Pick> {
        if input.mouse_held(MouseButton::Left) {
            if let Some(rotation) = self.rotation(input.mouse_diff()) {
                self.position = self.look_at - rotation.transform_vector(self.look_at - self.position);
                self.pick_cursor = None;
            }
        }
        if input.mouse_pressed(MouseButton::Left) {
            self.pick_cursor = input.cursor();
        }
        if input.mouse_released(MouseButton::Left) {
            if let Some(pick_cursor) = self.pick_cursor {
                self.pick = self.pick(pick_cursor, fabric);
                return Some(self.pick.clone());
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
        None
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

    pub fn pick(&mut self, (px, py): (f32, f32), fabric: &Fabric) -> Pick {
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
        let best_joint = || fabric
            .joints
            .iter()
            .enumerate()
            .map(|(index, joint)| {
                (index, (joint.location.to_vec() - self.position.to_vec()).normalize().dot(ray))
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b));
        let best_interval_around = |joint: usize| fabric
            .intervals
            .iter()
            .filter(|(_, interval)| interval.touches(joint))
            .map(|(interval_id, interval)| {
                let other = fabric.joints[interval.other_joint(joint)];
                let dot = (other.location.to_vec() - self.position.to_vec())
                    .normalize()
                    .dot(ray);
                (interval_id, dot)
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b));
        match self.pick {
            Pick::Nothing => match best_joint() {
                None => Pick::Nothing,
                Some((id, _)) => Pick::Joint(id),
            },
            Pick::Joint(joint) => match best_interval_around(joint) {
                None => Pick::Nothing,
                Some((id, _)) => Pick::Interval { joint, id: *id, interval: *fabric.interval(*id) }
            },
            Pick::Interval { joint, id, .. } => match best_interval_around(joint) {
                None => Pick::Nothing,
                Some((picked_id, _)) if *picked_id == id => {
                    Pick::Nothing
                }
                Some((picked_id, _)) => {
                    Pick::Interval { joint, id: *picked_id, interval: *fabric.interval(*picked_id) }
                }
            },
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
    AroundJoint(usize),
    AroundInterval(UniqueId),
}

impl Target {
    pub fn look_at(&self, fabric: &Fabric) -> Point3<f32> {
        match self {
            Target::Origin => point3(0.0, 0.0, 0.0),
            Target::FabricMidpoint => fabric.midpoint(),
            Target::AroundJoint(joint_id) => {
                fabric.joints[*joint_id].location
            }
            Target::AroundInterval(interval_id) => {
                fabric.interval(*interval_id).midpoint(&fabric.joints)
            }
        }
    }
}
