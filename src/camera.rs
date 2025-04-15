use std::f32::consts::PI;

use cgmath::num_traits::abs;
use cgmath::{
    perspective, point3, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rad,
    Rotation, Rotation3, SquareMatrix, Transform, Vector3,
};
use winit::dpi::PhysicalPosition;

use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::{Fabric, UniqueId};
use crate::{PointerChange, Shot};

#[derive(Debug, Clone)]
pub enum Pick {
    Nothing,
    Joint {
        index: usize,
        joint: Joint,
    },
    Interval {
        joint: usize,
        id: UniqueId,
        interval: Interval,
        length: f32,
        distance: f32,
    },
}

const TARGET_HIT: f32 = 0.001;
const TARGET_ATTRACTION: f32 = 0.01;
const DOT_CLOSE_ENOUGH: f32 = 0.92;

pub struct Camera {
    position: Point3<f32>,
    target: Target,
    look_at: Point3<f32>,
    width: f32,
    height: f32,
    mouse_now: Option<PhysicalPosition<f64>>,
    mouse_follower: Option<PhysicalPosition<f64>>,
    mouse_click: Option<PhysicalPosition<f64>>,
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
            mouse_now: None,
            mouse_follower: None,
            mouse_click: None,
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

    pub fn pointer_changed(
        &mut self,
        pointer_change: PointerChange,
        fabric: &Fabric,
    ) -> Option<Pick> {
        match pointer_change {
            PointerChange::NoChange => {}
            PointerChange::Moved(mouse_now) => {
                self.mouse_now = Some(mouse_now);
                if let Some(mouse_follower) = self.mouse_follower {
                    let diff = (
                        (mouse_now.x - mouse_follower.x) as f32,
                        (mouse_now.y - mouse_follower.y) as f32,
                    );
                    if let Some(rotation) = self.rotation(diff) {
                        self.position =
                            self.look_at - rotation.transform_vector(self.look_at - self.position);
                    }
                    self.mouse_follower = Some(mouse_now)
                }
            }
            PointerChange::Zoomed(delta) => {
                let gaze = self.look_at - self.position;
                if gaze.magnitude() - delta > 1.0 {
                    self.position += gaze.normalize() * delta;
                }
            }
            PointerChange::Pressed => {
                self.mouse_follower = self.mouse_now;
                self.mouse_click = self.mouse_now;
            }
            PointerChange::Released(shot) => {
                self.mouse_follower = None;
                if let (Some(mouse_click), Some(mouse_now)) = (self.mouse_click, self.mouse_now) {
                    let (dx, dy) = (
                        (mouse_now.x - mouse_click.x) as f32,
                        (mouse_now.y - mouse_click.y) as f32,
                    );
                    if dx * dx + dy * dy > 32.0 {
                        // they're dragging
                        return None;
                    }
                    self.mouse_click = None;
                    self.current_pick =
                        self.pick_ray((mouse_now.x as f32, mouse_now.y as f32), shot, fabric);
                    return Some(self.current_pick.clone());
                }
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
            Shot::NoPick => Pick::Nothing,
            Shot::Joint => match self.current_pick {
                Pick::Nothing => match self.best_joint(ray, fabric) {
                    None => Pick::Nothing,
                    Some((index, joint)) => Pick::Joint { index, joint },
                },
                Pick::Joint { index, .. } => match self.best_joint_around(index, ray, fabric) {
                    None => Pick::Nothing,
                    Some((index, joint)) => Pick::Joint { index, joint },
                },
                Pick::Interval {
                    joint, interval, ..
                } => {
                    let index = interval.other_joint(joint);
                    let joint = fabric.joints[index];
                    Pick::Joint { index, joint }
                }
            },
            Shot::Interval => match self.current_pick {
                Pick::Nothing => Pick::Nothing,
                Pick::Joint { index, .. } => match self.best_interval_around(index, ray, fabric) {
                    None => Pick::Nothing,
                    Some(id) => {
                        let interval = *fabric.interval(id);
                        let length = interval.ideal();
                        let distance = interval.length(fabric.joints.as_ref());
                        Pick::Interval {
                            joint: index,
                            id,
                            interval,
                            length,
                            distance,
                        }
                    }
                },
                Pick::Interval { joint, .. } => {
                    match self.best_interval_around(joint, ray, fabric) {
                        None => Pick::Nothing,
                        Some(id) => {
                            let interval = *fabric.interval(id);
                            let length = interval.ideal();
                            let distance = interval.length(fabric.joints.as_ref());
                            Pick::Interval {
                                joint,
                                id,
                                interval,
                                length,
                                distance,
                            }
                        }
                    }
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
        let rot_x = Matrix4::from_axis_angle(Vector3::unit_y(), Deg(dx * -0.5));
        let intermediate_pos = self.look_at - rot_x.transform_vector(self.look_at - self.position);
        let view_dir = (self.look_at - intermediate_pos).normalize();
        let up_dot_view = Vector3::unit_y().dot(view_dir);
        let angle_limit = 0.7;
        if (up_dot_view >= angle_limit && dy < 0.0) || (up_dot_view <= -angle_limit && dy > 0.0) {
            return Some(rot_x);
        }
        let axis = Vector3::unit_y().cross(view_dir).normalize();
        let rot_y = Matrix4::from_axis_angle(axis, Deg(dy * 0.4));
        Some(rot_x * rot_y)
    }

    fn best_joint_around(
        &self,
        joint: usize,
        ray: Vector3<f32>,
        fabric: &Fabric,
    ) -> Option<(usize, Joint)> {
        fabric
            .intervals
            .iter()
            .filter(|(_, interval)| interval.touches(joint))
            .map(|(interval_id, interval)| {
                let midpoint = interval.midpoint(&fabric.joints);
                let dot = (midpoint.to_vec() - self.position.to_vec())
                    .normalize()
                    .dot(ray);
                (interval_id, dot)
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b))
            .filter(|(_, dot)| *dot > DOT_CLOSE_ENOUGH)
            .map(|(id, _)| {
                let joint_index = fabric.interval(*id).other_joint(joint);
                let joint = &fabric.joints[joint_index];
                (joint_index, *joint)
            })
    }

    fn best_interval_around(
        &self,
        joint: usize,
        ray: Vector3<f32>,
        fabric: &Fabric,
    ) -> Option<UniqueId> {
        fabric
            .intervals
            .iter()
            .filter(|(_, interval)| interval.touches(joint))
            .map(|(interval_id, interval)| {
                let midpoint = interval.midpoint(&fabric.joints);
                let dot = (midpoint.to_vec() - self.position.to_vec())
                    .normalize()
                    .dot(ray);
                (interval_id, dot)
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b))
            .map(|(id, _)| *id)
    }

    fn best_joint(&self, ray: Vector3<f32>, fabric: &Fabric) -> Option<(usize, Joint)> {
        fabric
            .joints
            .iter()
            .enumerate()
            .map(|(index, joint)| {
                (
                    index,
                    (joint.location.to_vec() - self.position.to_vec())
                        .normalize()
                        .dot(ray),
                    joint,
                )
            })
            .max_by(|(_, dot_a, _), (_, dot_b, _)| dot_a.total_cmp(dot_b))
            .filter(|(_, dot, _)| *dot > DOT_CLOSE_ENOUGH)
            .map(|(index, _, joint)| (index, *joint))
    }
}

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
            Target::AroundJoint(joint_id) => fabric.joints[*joint_id].location,
            Target::AroundInterval(interval_id) => {
                fabric.interval(*interval_id).midpoint(&fabric.joints)
            }
        }
    }
}
