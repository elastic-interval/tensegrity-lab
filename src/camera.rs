use std::f32::consts::PI;

use cgmath::{
    Deg, EuclideanSpace, InnerSpace, Matrix4, perspective, point3, Point3, Quaternion, Rad, Rotation,
    Rotation3, SquareMatrix, Transform, vec3, Vector3,
};
use cgmath::num_traits::abs;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::event::MouseScrollDelta::PixelDelta;

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
    position: Point3<f32>,
    target: Target,
    look_at: Point3<f32>,
    width: f32,
    height: f32,
    cursor_position: Option<PhysicalPosition<f64>>,
    mouse_anchor: Option<PhysicalPosition<f64>>,
    pick: Pick,
}

impl Camera {
    pub fn new(position: Point3<f32>, width: f32, height: f32) -> Self {
        Self {
            position,
            target: Target::default(),
            look_at: point3(0.0, 3.0, 0.0),
            width,
            height,
            cursor_position: None,
            mouse_anchor: None,
            pick: Pick::Nothing,
        }
    }

    pub fn set_target(&mut self, target: Target) {
        self.target = target
    }

    pub fn current_pick(&self) -> &Pick {
        &self.pick
    }

    pub fn set_pick(&mut self, pick: Pick) {
        self.pick = pick;
    }

    pub fn reset(&self) {
        // get back to a good position
    }

    pub fn cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = Some(position);
        if let Some(mouse_anchor) = self.mouse_anchor {
            let diff = ((position.x - mouse_anchor.x) as f32, (position.y - mouse_anchor.y) as f32);
            if let Some(rotation) = self.rotation(diff) {
                self.position = self.look_at - rotation.transform_vector(self.look_at - self.position);
            }
            self.mouse_anchor = Some(position)
        }
    }

    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if let PixelDelta(position) = delta {
            let sy = position.y as f32;
            let scroll = sy * SPEED.z;
            let gaze = self.look_at - self.position;
            if gaze.magnitude() - scroll > 1.0 {
                self.position += gaze.normalize() * scroll;
            }
        }
    }

    pub fn mouse_input(&mut self, state: ElementState, button: MouseButton, fabric: &Fabric) {
        if button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    self.mouse_anchor = self.cursor_position
                }
                ElementState::Released => {
                    if let (Some(anchor), Some(position)) = (self.mouse_anchor, self.cursor_position) {
                        let (dx, dy) = ((position.x - anchor.x) as f32, (position.y - anchor.y) as f32);
                        let diff = dx * dx + dy * dy;
                        if diff < 32.0 {
                            self.pick = self.pick((anchor.x as f32, anchor.y as f32), fabric);
                        }
                    }
                    self.mouse_anchor = None
                }
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

const SPEED: Vector3<f32> = vec3(-0.5, 0.4, 0.1);

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
