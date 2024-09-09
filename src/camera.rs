use std::f32::consts::PI;

use cgmath::{
    Deg, EuclideanSpace, InnerSpace, Matrix4, perspective, point3, Point3, Quaternion, Rad, Rotation,
    Rotation3, SquareMatrix, Transform, vec3, Vector3,
};
use cgmath::num_traits::abs;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseScrollDelta};

use crate::fabric::{Fabric, UniqueId};
use crate::fabric::interval::Interval;

#[derive(Debug, Clone)]
pub enum Pick {
    Nothing,
    Joint {
        index: usize,
        height: f32,
    },
    Interval {
        joint: usize,
        id: UniqueId,
        interval: Interval,
    },
}

pub enum Shot {
    Joint,
    Interval,
}

const TARGET_HIT: f32 = 0.001;
const TARGET_ATTRACTION: f32 = 0.01;

pub struct Camera {
    position: Point3<f32>,
    target: Target,
    look_at: Point3<f32>,
    width: f32,
    height: f32,
    cursor_position: Option<PhysicalPosition<f64>>,
    mouse_anchor: Option<PhysicalPosition<f64>>,
    current_pick: Pick,
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
            current_pick: Pick::Nothing,
        }
    }

    pub fn set_target(&mut self, target: Target) {
        self.target = target
    }

    pub fn current_pick(&self) -> Pick {
        self.current_pick.clone()
    }

    pub fn reset(&mut self) {
        self.current_pick = Pick::Nothing; // more?
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
        let mut wheel = |scroll: f32| {
            let gaze = self.look_at - self.position;
            if gaze.magnitude() - scroll > 1.0 {
                self.position += gaze.normalize() * scroll;
            }
        };
        match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                wheel(y * SPEED.z * 5.0)
            }
            MouseScrollDelta::PixelDelta(position) => {
                wheel((position.y as f32) * SPEED.z);
            }
        }
    }

    pub fn mouse_input(&mut self, state: ElementState, shot: Shot, fabric: &Fabric) -> Option<Pick> {
        match state {
            ElementState::Pressed => {
                self.mouse_anchor = self.cursor_position;
            }
            ElementState::Released => {
                if let (Some(anchor), Some(position)) = (self.mouse_anchor, self.cursor_position) {
                    let (dx, dy) = ((position.x - anchor.x) as f32, (position.y - anchor.y) as f32);
                    if dx * dx + dy * dy > 64.0 { // they're dragging
                        return None;
                    }
                    self.mouse_anchor = None;
                    self.current_pick = self.pick_ray((anchor.x as f32, anchor.y as f32), shot, fabric);
                    return Some(self.current_pick.clone());
                }
                self.mouse_anchor = None;
            }
        }
        None
    }

    pub fn target_approach(&mut self, fabric: &Fabric) -> bool {
        let look_at = self.target.look_at(fabric);
        let delta = (look_at - self.look_at) * TARGET_ATTRACTION;
        let working = delta.magnitude() > TARGET_HIT;
        self.look_at += delta;
        let gaze = (self.look_at - self.position).normalize();
        let up_dot_gaze = Vector3::unit_y().dot(gaze);
        if !(-0.9..=0.9).contains(&up_dot_gaze) {
            let axis = Vector3::unit_y().cross(gaze).normalize();
            self.position = Point3::from_vec(
                Quaternion::from_axis_angle(axis, Rad(0.01 * up_dot_gaze / abs(up_dot_gaze)))
                    .rotate_vector(self.position.to_vec()),
            );
        }
        working
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn mvp_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    fn pick_ray(&mut self, (px, py): (f32, f32), shot: Shot, fabric: &Fabric) -> Pick {
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
        match shot {
            Shot::Joint => {
                match self.current_pick {
                    Pick::Nothing => {
                        match self.best_joint(ray, fabric) {
                            None => Pick::Nothing,
                            Some((index, height)) => Pick::Joint { index, height },
                        }
                    }
                    Pick::Joint { index, .. } => {
                        match self.best_joint_around(index, ray, fabric) {
                            None => Pick::Nothing,
                            Some((index, height)) => Pick::Joint { index, height },
                        }
                    }
                    Pick::Interval { joint, .. } => {
                        match self.best_joint_around(joint, ray, fabric) {
                            None => Pick::Nothing,
                            Some((index, height)) => Pick::Joint { index, height },
                        }
                    }
                }
            }
            Shot::Interval => {
                match self.current_pick {
                    Pick::Nothing => Pick::Nothing,
                    Pick::Joint { index, .. } => {
                        match self.best_interval_around(index, ray, fabric) {
                            None => Pick::Nothing,
                            Some(id) => Pick::Interval { joint: index, id, interval: *fabric.interval(id) }
                        }
                    }
                    Pick::Interval { joint, .. } => {
                        match self.best_interval_around(joint, ray, fabric) {
                            None => Pick::Nothing,
                            Some(id) => Pick::Interval { joint, id, interval: *fabric.interval(id) }
                        }
                    }
                }
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

    fn best_joint_around(&self, joint: usize, ray: Vector3<f32>, fabric: &Fabric) -> Option<(usize, f32)> {
        match self.best_interval_around(joint, ray, fabric) {
            None => None,
            Some(id) => {
                let index = fabric.interval(id).other_joint(joint);
                let height = fabric.joints[index].location.y;
                Some((index, height))
            }
        }
    }

    fn best_interval_around(&self, joint: usize, ray: Vector3<f32>, fabric: &Fabric) -> Option<UniqueId> {
        fabric
            .intervals
            .iter()
            .filter(|(_, interval)| interval.touches(joint))
            .map(|(interval_id, interval)| {
                let midpoint = interval.midpoint(&fabric.joints);
                let dot = (midpoint.to_vec() - self.position.to_vec()).normalize().dot(ray);
                (interval_id, dot)
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b))
            .map(|(id, _)| *id)
    }

    fn best_joint(&self, ray: Vector3<f32>, fabric: &Fabric) -> Option<(usize, f32)> {
        fabric
            .joints
            .iter()
            .enumerate()
            .map(|(index, joint)| {
                (index, (joint.location.to_vec() - self.position.to_vec()).normalize().dot(ray), joint.location.y)
            })
            .max_by(|(_, dot_a, _), (_, dot_b, _)| dot_a.total_cmp(dot_b))
            .map(|(index, _, height)| (index, height))
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
