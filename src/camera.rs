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

    fn create_joint_details(&self, index: usize, location: Point3<f32>, scale: f32) -> JointDetails {
        JointDetails {
            index,
            location,
            scale,
        }
    }

    fn create_interval_details(&self, id: UniqueId, near_joint: usize, fabric: &Fabric) -> IntervalDetails {
        let interval = fabric.interval(id);
        let far_joint = interval.other_joint(near_joint);
        
        IntervalDetails {
            id,
            near_joint,
            far_joint,
            length: interval.ideal(),
            strain: interval.strain,
            distance: fabric.distance(near_joint, far_joint),
            role: interval.material.properties().role,
            scale: fabric.scale,
        }
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
                        Pick::Joint(self.create_joint_details(index, joint.location, scale))
                    }
                },
                Pick::Joint(JointDetails { index, .. }) => {
                    match self.best_joint_around(index, ray, fabric) {
                        None => Pick::Nothing,
                        Some((index, joint)) => {
                            Pick::Joint(self.create_joint_details(index, joint.location, scale))
                        }
                    }
                }
                Pick::Interval(details) => {
                    let index = details.far_joint;
                    Pick::Joint(self.create_joint_details(index, fabric.location(index), scale))
                }
            },
            Shot::Interval => match self.current_pick {
                Pick::Nothing => Pick::Nothing,
                Pick::Joint(JointDetails { index, .. }) => {
                    match self.best_interval_around(index, ray, fabric) {
                        None => Pick::Nothing,
                        Some(id) => Pick::Interval(self.create_interval_details(id, index, fabric))
                    }
                }
                Pick::Interval(details) => {
                    // Try to find an interval near the current one
                    match self.best_interval_around(details.near_joint, ray, fabric) {
                        None => match self.best_interval_around(details.far_joint, ray, fabric) {
                            None => Pick::Nothing,
                            Some(id) => Pick::Interval(self.create_interval_details(id, details.far_joint, fabric))
                        },
                        Some(id) => Pick::Interval(self.create_interval_details(id, details.near_joint, fabric))
                    }
                },
            },
        }
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.look_at, Vector3::unit_y())
    }

    fn projection_matrix(&self) -> Matrix4<f32> {
        let aspect = self.width / self.height;
        OPENGL_TO_WGPU_MATRIX * perspective(Deg(45.0), aspect, 0.1, 100.0)
    }

    fn rotation(&self, (dx, dy): (f32, f32)) -> Option<Matrix4<f32>> {
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        let gaze = (self.look_at - self.position).normalize();
        let right = gaze.cross(Vector3::unit_y()).normalize();
        let up = right.cross(gaze).normalize();
        let yaw = Quaternion::from_axis_angle(up, Rad(dx / 100.0));
        let pitch = Quaternion::from_axis_angle(right, Rad(dy / 100.0));
        let rotation = yaw * pitch;
        let matrix = Matrix4::from(rotation);
        Some(matrix)
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
                .enumerate()
                .filter_map(|(index, interval_opt)| {
                    interval_opt.as_ref().and_then(|interval| {
                        if interval.touches(joint) {
                            Some(UniqueId(index))
                        } else {
                            None
                        }
                    })
                }),
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
                .enumerate()
                .filter_map(|(index, interval_opt)| {
                    interval_opt.as_ref().and_then(|interval| {
                        if interval.touches(joint) {
                            Some(UniqueId(index))
                        } else {
                            None
                        }
                    })
                }),
            ray,
            |&interval_id| fabric.interval(interval_id).midpoint(&fabric.joints),
            |_| true, // No dot product filtering needed here
        )
    }

    fn best_joint(&self, ray: Vector3<f32>, fabric: &Fabric) -> Option<(usize, Joint)> {
        self.find_best_by_dot(
            fabric.joints.iter().enumerate(),
            ray,
            |(_, joint)| joint.location,
            |_| true,
        )
        .map(|(index, joint)| (index, *joint))
    }

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
        let mut best_item = None;
        let mut best_dot = -1.0;
        for item in items {
            let position = get_position(&item);
            let to_target = (position - self.position).normalize();
            let dot = ray.dot(to_target);
            if dot > best_dot && dot_filter(dot) {
                best_dot = dot;
                best_item = Some(item);
            }
        }
        best_item
    }
}

const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug, Clone, Copy)]
pub enum Target {
    FabricMidpoint,
    AroundJoint(usize),
    AroundInterval(UniqueId),
}

impl Default for Target {
    fn default() -> Self {
        Self::FabricMidpoint
    }
}

impl Target {
    pub fn look_at(&self, fabric: &Fabric) -> Point3<f32> {
        match self {
            Target::FabricMidpoint => fabric.midpoint(),
            Target::AroundJoint(index) => fabric.location(*index),
            Target::AroundInterval(id) => fabric.interval(*id).midpoint(&fabric.joints),
        }
    }
}
