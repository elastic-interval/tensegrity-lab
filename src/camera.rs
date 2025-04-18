use std::f32::consts::PI;

use cgmath::num_traits::abs;
use cgmath::{
    perspective, point3, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rad,
    Rotation, Rotation3, SquareMatrix, Transform, Vector3,
};
use winit::dpi::PhysicalPosition;

use crate::fabric::joint::Joint;
use crate::fabric::{Fabric, UniqueId};
use crate::{ControlState, IntervalDetails, JointDetails, PointerChange, Radio, Shot};

#[derive(Debug, Clone)]
pub enum Pick {
    Nothing,
    Joint(JointDetails),
    Interval(IntervalDetails),
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
    radio: Radio,
}

impl Camera {
    pub fn new(position: Point3<f32>, width: f32, height: f32, radio: Radio) -> Self {
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
            radio,
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
        self.set_target(Target::FabricMidpoint);
    }

    pub fn pointer_changed(&mut self, pointer_change: PointerChange, fabric: &Fabric) {
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
                        return;
                    }
                    self.mouse_click = None;
                    let PhysicalPosition { x, y } = mouse_now;
                    let pick = self.pick_ray((x as f32, y as f32), shot, fabric);
                    match pick {
                        Pick::Nothing => {
                            self.set_target(Target::FabricMidpoint);
                        }
                        Pick::Joint(details) => {
                            self.set_target(Target::AroundJoint(details.index));
                            ControlState::ShowingJoint(details).send(&self.radio);
                        }
                        Pick::Interval(details) => {
                            self.set_target(Target::AroundInterval(details.id));
                            ControlState::ShowingInterval(details).send(&self.radio);
                        }
                    }
                    self.current_pick = pick;
                }
            }
        }
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
        let scale = fabric.scale;
        match shot {
            Shot::NoPick => Pick::Nothing,
            Shot::Joint => match self.current_pick {
                Pick::Nothing => match self.best_joint(ray, fabric) {
                    None => Pick::Nothing,
                    Some((index, joint)) => {
                        let location = joint.location;
                        Pick::Joint(JointDetails {
                            index,
                            location,
                            scale,
                        })
                    }
                },
                Pick::Joint(JointDetails { index, .. }) => {
                    match self.best_joint_around(index, ray, fabric) {
                        None => Pick::Nothing,
                        Some((index, joint)) => {
                            let location = joint.location;
                            Pick::Joint(JointDetails {
                                index,
                                location,
                                scale,
                            })
                        }
                    }
                }
                Pick::Interval(details) => {
                    let index = details.far_joint;
                    let location = fabric.location(index);
                    Pick::Joint(JointDetails {
                        index,
                        location,
                        scale,
                    })
                }
            },
            Shot::Interval => match self.current_pick {
                Pick::Nothing => Pick::Nothing,
                Pick::Joint(JointDetails { index, .. }) => {
                    match self.best_interval_around(index, ray, fabric) {
                        None => Pick::Nothing,
                        Some(id) => {
                            let interval = *fabric.interval(id);
                            let length = interval.ideal();
                            let distance = interval.length(fabric.joints.as_ref());
                            let role = interval.material.properties().role;
                            let near_joint = if interval.alpha_index == index {
                                interval.alpha_index
                            } else {
                                interval.omega_index
                            };
                            let far_joint = interval.other_joint(near_joint);
                            let strain = interval.strain;
                            Pick::Interval(IntervalDetails {
                                id,
                                near_joint,
                                far_joint,
                                length,
                                role,
                                strain,
                                distance,
                                scale,
                            })
                        }
                    }
                }
                Pick::Interval(details) => {
                    match self.best_interval_around(details.near_joint, ray, fabric) {
                        None => Pick::Nothing,
                        Some(id) => {
                            let interval = *fabric.interval(id);
                            let role = interval.material.properties().role;
                            let length = interval.ideal();
                            let distance = interval.length(fabric.joints.as_ref());
                            let near_joint = details.near_joint;
                            let far_joint = interval.other_joint(near_joint);
                            let strain = interval.strain;
                            let scale = fabric.scale;
                            Pick::Interval(IntervalDetails {
                                id,
                                near_joint,
                                far_joint,
                                length,
                                strain,
                                distance,
                                role,
                                scale,
                            })
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
        self.find_best_by_dot(
            fabric
                .intervals
                .iter()
                .filter(|(_, interval)| interval.touches(joint))
                .map(|(id, _)| *id),
            ray,
            |&interval_id| fabric.interval(interval_id).midpoint(&fabric.joints),
            |dot| dot > DOT_CLOSE_ENOUGH,
        )
        .map(|id| {
            let joint_index = fabric.interval(id).other_joint(joint);
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
        self.find_best_by_dot(
            fabric
                .intervals
                .iter()
                .filter(|(_, interval)| interval.touches(joint))
                .map(|(id, _)| *id),
            ray,
            |&interval_id| fabric.interval(interval_id).midpoint(&fabric.joints),
            |_| true, // No dot product filtering needed here
        )
    }

    fn best_joint(&self, ray: Vector3<f32>, fabric: &Fabric) -> Option<(usize, Joint)> {
        self.find_best_by_dot(
            fabric.joints.iter().enumerate(),
            ray,
            |&(_, joint)| joint.location,
            |dot| dot > DOT_CLOSE_ENOUGH,
        )
        .map(|(index, joint)| (index, *joint))
    }

    // Helper function to find the best item in a collection based on dot product with a ray
    fn find_best_by_dot<T, F, G>(
        &self,
        items: impl Iterator<Item = T>,
        ray: Vector3<f32>,
        get_position: F,
        dot_filter: G,
    ) -> Option<T>
    where
        F: Fn(&T) -> Point3<f32>,
        G: Fn(f32) -> bool,
    {
        items
            .map(|item| {
                let dot = (get_position(&item).to_vec() - self.position.to_vec())
                    .normalize()
                    .dot(ray);
                (item, dot)
            })
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b))
            .filter(|(_, dot)| dot_filter(*dot))
            .map(|(item, _)| item)
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
