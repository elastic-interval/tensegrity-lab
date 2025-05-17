use cgmath::num_traits::abs;
use cgmath::{
    ortho, perspective, point3, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rad,
    Rotation, Rotation3, SquareMatrix, Transform, Vector3,
};
use winit::dpi::PhysicalPosition;

use crate::fabric::interval::Interval;
use crate::fabric::joint_incident::JointIncident;
use crate::fabric::Fabric;
use crate::fabric::IntervalEnd;
use crate::fabric::UniqueId;
use crate::{ControlState, IntervalDetails, JointDetails, PickIntent, PointerChange, Radio, Role};

#[derive(Debug, Clone)]
pub enum Pick {
    Nothing,
    Joint(JointDetails),
    Interval(IntervalDetails),
}

const TARGET_HIT: f32 = 0.001;
const TARGET_ATTRACTION: f32 = 0.01;

/// Defines the type of projection used by the camera
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionType {
    /// Perspective projection (default)
    Perspective,
    /// Orthogonal projection
    Orthogonal,
}

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
    projection_type: ProjectionType,
    // Store the last ray origin for use in picking calculations
    last_ray_origin: Point3<f32>,
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
            projection_type: ProjectionType::Perspective, // Default to perspective projection
            last_ray_origin: position,                    // Initialize with camera position
        }
    }

    pub fn set_target(&mut self, target: Target) {
        self.target = target
    }

    /// Toggle between perspective and orthogonal projection
    pub fn toggle_projection(&mut self) {
        self.projection_type = match self.projection_type {
            ProjectionType::Perspective => ProjectionType::Orthogonal,
            ProjectionType::Orthogonal => ProjectionType::Perspective,
        };
    }

    /// Returns a reference to the current pick state
    pub fn current_pick(&self) -> &Pick {
        &self.current_pick
    }

    /// Main picking method that determines what the user is selecting based on mouse position and intent
    fn pick_ray(&mut self, (px, py): (f32, f32), pick_intent: PickIntent, fabric: &Fabric) -> Pick {
        let width = self.width / 2.0;
        let height = self.height / 2.0;
        let (ray_origin, ray_direction) = self.calculate_ray(px, py, width, height);
        self.last_ray_origin = ray_origin;
        match pick_intent {
            PickIntent::Reset => Pick::Nothing,
            PickIntent::Select => self.select(ray_direction, fabric),
            PickIntent::Traverse => self.traverse_interval(fabric),
        }
    }

    // Calculate ray based on projection type
    fn calculate_ray(
        &self,
        px: f32,
        py: f32,
        width: f32,
        height: f32,
    ) -> (Point3<f32>, Vector3<f32>) {
        let x = (px - width) / width;
        let y = (height - py) / height;

        match self.projection_type {
            ProjectionType::Perspective => self.calculate_perspective_ray(x, y),
            ProjectionType::Orthogonal => self.calculate_orthogonal_ray(x, y),
        }
    }

    // Calculate ray for perspective projection
    fn calculate_perspective_ray(&self, x: f32, y: f32) -> (Point3<f32>, Vector3<f32>) {
        let position = Point3::new(x, y, 1.0);
        let point3d = self
            .mvp_matrix()
            .invert()
            .unwrap()
            .transform_point(position);
        (self.position, (point3d - self.position).normalize())
    }

    // Calculate ray for orthogonal projection
    fn calculate_orthogonal_ray(&self, x: f32, y: f32) -> (Point3<f32>, Vector3<f32>) {
        let view_dir = (self.look_at - self.position).normalize();
        let right = view_dir.cross(Vector3::unit_y()).normalize();
        let up = right.cross(view_dir).normalize();

        let distance = (self.look_at - self.position).magnitude();
        let view_size = distance * 0.5;

        let x_offset = x * view_size * self.width / self.height; // Adjust for aspect ratio
        let y_offset = y * view_size;

        let center_point = self.position + view_dir * distance;
        let ray_origin = center_point + right * x_offset + up * y_offset;

        (ray_origin, view_dir)
    }

    fn select(&self, ray: Vector3<f32>, fabric: &Fabric) -> Pick {
        if let Some(best_incident) = self.best_joint(ray, fabric) {
            match self.current_pick {
                Pick::Nothing => Pick::Joint(JointDetails {
                    index: best_incident.index,
                    location: fabric.location(best_incident.index),
                    scale: fabric.scale,
                    selected_push: best_incident.push.map(|(unique_id, _)| unique_id),
                }),
                Pick::Joint(details) => {
                    let (id, interval) = best_incident.interval_to(details.index).unwrap();
                    Pick::Interval(self.create_interval_details(
                        id,
                        interval,
                        details.index,
                        fabric,
                        details.selected_push,
                    ))
                }
                Pick::Interval(details) => {
                    let (id, interval) = best_incident.interval_to(details.near_joint).unwrap();
                    Pick::Interval(self.create_interval_details(
                        id,
                        interval,
                        details.near_joint,
                        fabric,
                        details.selected_push,
                    ))
                }
            }
        } else {
            Pick::Nothing
        }
    }

    fn traverse_interval(&self, fabric: &Fabric) -> Pick {
        match self.current_pick() {
            Pick::Nothing | Pick::Joint(_) => Pick::Nothing,
            Pick::Interval(old) => {
                let mut new = old.clone();
                new.near_joint = old.far_joint;
                new.near_slot = old.far_slot;
                new.far_joint = old.near_joint;
                new.far_slot = old.near_slot;
                new.selected_push = fabric.find_push_at(old.far_joint);
                Pick::Interval(new)
            }
        }
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
            PointerChange::Released(pick_intent) => {
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
                    let pick = self.pick_ray((x as f32, y as f32), pick_intent, fabric);
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

    fn create_interval_details(
        &self,
        id: UniqueId,
        interval: Interval,
        near_joint: usize,
        fabric: &Fabric,
        selected_push: Option<UniqueId>,
    ) -> IntervalDetails {
        let far_joint = interval.other_joint(near_joint);

        // Calculate slot indices for pull intervals
        let (near_slot, far_slot) = if interval.material.properties().role == Role::Pulling {
            // Helper function to find slot index for a joint
            let find_slot_for_joint = |joint_index: usize| -> Option<usize> {
                fabric
                    .intervals
                    .iter()
                    .filter_map(|interval_opt| interval_opt.as_ref())
                    .filter(|interval| {
                        interval.material.properties().role == Role::Pushing
                            && interval.touches(joint_index)
                    })
                    .find_map(|push_interval| {
                        // Determine which end of the push interval is connected to this joint
                        let end = if push_interval.alpha_index == joint_index {
                            IntervalEnd::Alpha
                        } else {
                            IntervalEnd::Omega
                        };

                        // Get the connections for this end
                        let connections_array = match push_interval.connections.as_ref() {
                            Some(connections) => connections.connections(end),
                            None => return None,
                        };

                        // Look for a connection to this pull interval
                        for (idx, conn_opt) in connections_array.iter().enumerate() {
                            if let Some(conn) = conn_opt {
                                if conn.pull_interval_id == id {
                                    return Some(idx);
                                }
                            }
                        }

                        None
                    })
            };

            // Find slots for both joints
            let near_slot = find_slot_for_joint(near_joint);
            let far_slot = find_slot_for_joint(far_joint);

            (near_slot, far_slot)
        } else {
            (None, None)
        };

        IntervalDetails {
            id,
            near_joint,
            far_joint,
            length: interval.ideal(),
            strain: interval.strain,
            distance: fabric.distance(near_joint, far_joint),
            role: interval.material.properties().role,
            scale: fabric.scale,
            selected_push,
            near_slot,
            far_slot,
        }
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.look_at, Vector3::unit_y())
    }

    fn projection_matrix(&self) -> Matrix4<f32> {
        let aspect = self.width / self.height;
        let proj_matrix = match self.projection_type {
            ProjectionType::Perspective => perspective(Deg(45.0), aspect, 0.1, 100.0),
            ProjectionType::Orthogonal => {
                // For orthographic projection, calculate a reasonable view size based on distance
                let distance = (self.look_at - self.position).magnitude();
                let view_size = distance * 0.5;
                ortho(
                    -view_size * aspect,
                    view_size * aspect, // left, right
                    -view_size,
                    view_size, // bottom, top
                    0.1,
                    distance * 10.0, // near, far
                )
            }
        };
        OPENGL_TO_WGPU_MATRIX * proj_matrix
    }

    fn rotation(&self, (dx, dy): (f32, f32)) -> Option<Matrix4<f32>> {
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        let gaze = (self.look_at - self.position).normalize();
        let right = gaze.cross(Vector3::unit_y()).normalize();
        let up = right.cross(gaze).normalize();

        // Vertical angle limit (about 37 degrees from vertical, arc cos(0.8) ≈ 37°)
        let angle_limit = 0.8;

        // Calculate yaw (horizontal rotation)
        let yaw = Quaternion::from_axis_angle(up, Rad(dx / 100.0));

        // Apply yaw rotation first to get intermediate gaze direction
        let intermediate_gaze = yaw.rotate_vector(gaze);
        let intermediate_up_dot = Vector3::unit_y().dot(intermediate_gaze);

        // Only apply pitch if it won't exceed the limits
        let pitch = if (intermediate_up_dot >= angle_limit && dy > 0.0)
            || (intermediate_up_dot <= -angle_limit && dy < 0.0)
        {
            // At limit - don't apply any pitch
            Quaternion::from_axis_angle(right, Rad(0.0))
        } else {
            // Not at limit - apply requested pitch
            Quaternion::from_axis_angle(right, Rad(dy / 100.0))
        };

        let rotation = yaw * pitch;
        let matrix = Matrix4::from(rotation);
        Some(matrix)
    }

    fn best_joint(&self, ray: Vector3<f32>, fabric: &Fabric) -> Option<JointIncident> {
        let current_joint = match self.current_pick {
            Pick::Nothing => None,
            Pick::Joint(JointDetails { index, .. }) => Some(index),
            Pick::Interval(IntervalDetails { near_joint, .. }) => Some(near_joint),
        };
        self.find_best_by_dot(fabric.joint_incidents(), current_joint, ray)
    }

    fn find_best_by_dot(
        &self,
        joint_incidents: Vec<JointIncident>,
        current_joint: Option<usize>,
        ray: Vector3<f32>,
    ) -> Option<JointIncident> {
        // Use the ray origin that was calculated in pick_ray
        let ray_origin = self.last_ray_origin;

        joint_incidents
            .iter()
            .filter_map(|joint_incident| {
                if let Some(current) = current_joint {
                    if joint_incident.interval_to(current).is_none() {
                        return None;
                    }
                }
                let position = joint_incident.location;

                // Calculate selection score based on projection type
                let score = match self.projection_type {
                    ProjectionType::Perspective => {
                        // For perspective, use the dot product method
                        let to_position = (position - self.position).normalize();
                        ray.dot(to_position)
                    }
                    ProjectionType::Orthogonal => {
                        // For orthogonal, calculate the distance from the ray to the point
                        // The closer the point is to the ray, the higher the score
                        let ray_to_point = position - ray_origin;
                        let projection = ray_to_point.dot(ray) * ray;
                        let perpendicular = ray_to_point - projection;

                        // Negative distance (closer points have higher scores)
                        -perpendicular.magnitude()
                    }
                };

                Some((joint_incident, score))
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(item, _)| item.clone())
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
