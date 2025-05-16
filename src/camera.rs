use cgmath::num_traits::abs;
use cgmath::{
    ortho, perspective, point3, Deg, EuclideanSpace, InnerSpace, Matrix4, Point3, Quaternion, Rad,
    Rotation, Rotation3, SquareMatrix, Transform, Vector3,
};
use winit::dpi::PhysicalPosition;

use crate::fabric::joint::Joint;
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
const DOT_CLOSE_ENOUGH: f32 = 0.92;

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

    // This method is no longer needed as we have a reference-returning version below
    // pub fn current_pick(&self) -> Pick {
    //     self.current_pick.clone()
    // }

    /// Toggle between perspective and orthogonal projection
    pub fn toggle_projection(&mut self) {
        self.projection_type = match self.projection_type {
            ProjectionType::Perspective => ProjectionType::Orthogonal,
            ProjectionType::Orthogonal => ProjectionType::Perspective,
        };
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

    fn create_joint_details(
        &self,
        index: usize,
        location: Point3<f32>,
        scale: f32,
    ) -> JointDetails {
        JointDetails {
            index,
            location,
            scale,
        }
    }

    fn create_interval_details(
        &self,
        id: UniqueId,
        near_joint: usize,
        fabric: &Fabric,
        original_interval_id: Option<UniqueId>,
    ) -> IntervalDetails {
        let interval = fabric.interval(id);
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
            original_interval_id,
            near_slot,
            far_slot,
        }
    }

    /// Returns a reference to the current pick state
    pub fn current_pick(&self) -> &Pick {
        &self.current_pick
    }

    fn pick_ray(&mut self, (px, py): (f32, f32), pick_intent: PickIntent, fabric: &Fabric) -> Pick {
        let width = self.width / 2.0;
        let height = self.height / 2.0;
        let x = (px - width) / width;
        let y = (height - py) / height;

        // For picking, we need both a ray origin and direction
        // In perspective mode, the origin is always the camera position
        // In orthogonal mode, we need to calculate a different origin point

        let (ray_origin, ray_direction) = match self.projection_type {
            ProjectionType::Perspective => {
                // For perspective, use the standard ray calculation
                let position = Point3::new(x, y, 1.0);
                let point3d = self
                    .mvp_matrix()
                    .invert()
                    .unwrap()
                    .transform_point(position);
                (self.position, (point3d - self.position).normalize())
            }
            ProjectionType::Orthogonal => {
                // For orthographic projection, we need to calculate a ray origin that's offset from the camera
                // based on the screen coordinates

                // First, get the view direction and perpendicular vectors
                let view_dir = (self.look_at - self.position).normalize();
                let right = view_dir.cross(Vector3::unit_y()).normalize();
                let up = right.cross(view_dir).normalize();

                // Calculate the distance from camera to look_at point
                let distance = (self.look_at - self.position).magnitude();

                // Calculate the view size at the look_at distance
                let view_size = distance * 0.5;

                // Calculate the offset from the center of the screen
                let x_offset = x * view_size * self.width / self.height; // Adjust for aspect ratio
                let y_offset = y * view_size;

                // Calculate the ray origin by offsetting from a point in front of the camera
                let center_point = self.position + view_dir * distance;
                let ray_origin = center_point + right * x_offset + up * y_offset;

                (ray_origin, view_dir)
            }
        };

        // Store ray origin in the Camera struct for use in find_best_by_dot
        self.last_ray_origin = ray_origin;
        let ray = ray_direction;

        let scale = fabric.scale;

        // Determine if this is a right-click (which allows "traveling" to the other side of an interval)
        let is_right_click = matches!(
            pick_intent,
            PickIntent::TravelToJoint | PickIntent::TravelThroughInterval
        );

        // The main logic for picking is based on whether we're trying to select a joint or an interval
        match pick_intent {
            PickIntent::None => Pick::Nothing,
            PickIntent::SelectJoint | PickIntent::TravelToJoint => match self.current_pick {
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
                    // Only jump to the far joint if this is explicitly a right-click on a joint
                    // This prevents automatic jumping when selecting intervals
                    if matches!(pick_intent, PickIntent::TravelToJoint) {
                        let index = details.far_joint;
                        Pick::Joint(self.create_joint_details(index, fabric.location(index), scale))
                    } else {
                        // For left-clicks, keep the current interval selection
                        self.current_pick.clone()
                    }
                }
            },
            PickIntent::SelectInterval | PickIntent::TravelThroughInterval => match self
                .current_pick
            {
                Pick::Nothing => Pick::Nothing,
                Pick::Joint(JointDetails { index, .. }) => {
                    match self.best_interval_around(index, ray, fabric) {
                        None => Pick::Nothing,
                        Some(id) => {
                            Pick::Interval(self.create_interval_details(id, index, fabric, None))
                        }
                    }
                }
                Pick::Interval(details) => {
                    // Check if this is a click on the same interval
                    let current_id = details.id;
                    let current_near_joint = details.near_joint;

                    // Get the original interval ID (for nested selection)
                    // If this is a push interval or we already have an original interval, use it
                    let original_interval_id = if details.role == Role::Pushing {
                        // For push intervals, we want to remember them as the original interval
                        Some(details.id)
                    } else {
                        // For other intervals, use the existing original_interval_id if available
                        details.original_interval_id
                    };

                    if !is_right_click {
                        // For push intervals, treat intervals from both near and far joints equally
                        // For other interval types, maintain the original behavior
                        if details.role == Role::Pushing {
                            // Get the near and far joints
                            let near_joint = current_near_joint;
                            let far_joint = details.far_joint;

                            // Collect all adjacent intervals from both near and far joints
                            let mut adjacent_intervals: Vec<(UniqueId, usize)> = Vec::new();

                            // Find all intervals connected to either joint (except the current one)
                            for (index, interval_opt) in fabric.intervals.iter().enumerate() {
                                if let Some(interval) = interval_opt {
                                    let id = UniqueId(index);
                                    // Skip the current interval
                                    if id == current_id {
                                        continue;
                                    }

                                    // Add intervals connected to either joint
                                    if interval.touches(near_joint) {
                                        adjacent_intervals.push((id, near_joint));
                                    } else if interval.touches(far_joint) {
                                        adjacent_intervals.push((id, far_joint));
                                    }
                                }
                            }

                            // If we found adjacent intervals, find the best one
                            if !adjacent_intervals.is_empty() {
                                // Calculate the closest interval to the ray
                                let mut best_interval = None;
                                let mut best_score = f32::NEG_INFINITY;

                                for (id, joint_index) in &adjacent_intervals {
                                    let interval = fabric.interval(*id);
                                    let midpoint = interval.midpoint(&fabric.joints);

                                    // Calculate a score based on how close the interval is to the ray
                                    let score = match self.projection_type {
                                        ProjectionType::Perspective => {
                                            // For perspective, use the dot product method
                                            let to_midpoint =
                                                (midpoint - self.position).normalize();
                                            ray.dot(to_midpoint)
                                        }
                                        ProjectionType::Orthogonal => {
                                            // For orthogonal, calculate the distance from the ray to the midpoint
                                            let ray_to_point = midpoint - self.last_ray_origin;
                                            let projection = ray_to_point.dot(ray) * ray;
                                            let perpendicular = ray_to_point - projection;

                                            // Negative distance (closer points have higher scores)
                                            -perpendicular.magnitude()
                                        }
                                    };

                                    if score > best_score {
                                        best_score = score;
                                        best_interval = Some((*id, *joint_index));
                                    }
                                }

                                // If we found a good interval, select it
                                if let Some((id, joint_index)) = best_interval {
                                    return Pick::Interval(self.create_interval_details(
                                        id,
                                        joint_index,
                                        fabric,
                                        original_interval_id,
                                    ));
                                }
                            }
                        } else {
                            // For non-push intervals, use the original selection logic
                            // First try to find intervals around the current near joint
                            if let Some(id) =
                                self.best_interval_around(current_near_joint, ray, fabric)
                            {
                                if id == current_id {
                                    // If we're selecting the same interval again, keep the current near joint
                                    return Pick::Interval(self.create_interval_details(
                                        id,
                                        current_near_joint,
                                        fabric,
                                        original_interval_id,
                                    ));
                                } else {
                                    // New interval from the current near joint
                                    return Pick::Interval(self.create_interval_details(
                                        id,
                                        current_near_joint,
                                        fabric,
                                        original_interval_id,
                                    ));
                                }
                            }

                            // If no interval was found around the current near joint, try the far joint
                            let far_joint = details.far_joint;
                            if let Some(id) = self.best_interval_around(far_joint, ray, fabric) {
                                // Allow selecting any interval from the far joint
                                return Pick::Interval(self.create_interval_details(
                                    id,
                                    far_joint,
                                    fabric,
                                    original_interval_id,
                                ));
                            }
                        }

                        // If we get here, we couldn't find a new interval, so keep the current selection
                        self.current_pick.clone()
                    } else {
                        // For right-clicks, use the full interval selection logic which allows traveling
                        let selection =
                            self.best_interval_from_details(&details, ray, fabric, true);

                        match selection {
                            None => Pick::Nothing,
                            Some((id, near_joint)) => {
                                // Pass along the original_interval_id for nested selection
                                Pick::Interval(self.create_interval_details(
                                    id,
                                    near_joint,
                                    fabric,
                                    original_interval_id,
                                ))
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

        // Vertical angle limit (about 37 degrees from vertical, arccos(0.8) ≈ 37°)
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
            |dot| *dot > DOT_CLOSE_ENOUGH,
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

    /// Helper method to find the best interval when an interval is already selected
    /// This handles the logic for selecting intervals around both the near and far joints
    /// while preventing unintended "traveling" to the other side of the same interval
    fn best_interval_from_details(
        &self,
        details: &IntervalDetails,
        ray: Vector3<f32>,
        fabric: &Fabric,
        is_right_click: bool,
    ) -> Option<(UniqueId, usize)> {
        // Try both the near joint and far joint
        let near_result = self.best_interval_around(details.near_joint, ray, fabric);
        let far_result = self.best_interval_around(details.far_joint, ray, fabric);

        match (near_result, far_result) {
            (None, None) => None,
            (Some(id), None) => Some((id, details.near_joint)),
            (None, Some(id)) => Some((id, details.far_joint)),
            (Some(near_id), Some(far_id)) => {
                // Both joints have intervals - compare which one is more aligned with the ray
                let near_midpoint = fabric.interval(near_id).midpoint(&fabric.joints);
                let far_midpoint = fabric.interval(far_id).midpoint(&fabric.joints);

                let to_near = (near_midpoint - self.position).normalize();
                let to_far = (far_midpoint - self.position).normalize();

                let near_dot = ray.dot(to_near);
                let far_dot = ray.dot(to_far);

                // Check if either result is the same as the current interval (which would indicate "traveling")
                let near_is_same = near_id == details.id;
                let far_is_same = far_id == details.id;

                // Check if we're trying to "travel" to the other side of the same interval
                // (These variables were already declared above, so we'll use them directly)

                if is_right_click {
                    // Right-click explicitly allows "traveling" to the other side
                    // We'll pick the interval based on which one is more aligned with the ray
                    if near_dot > far_dot {
                        Some((near_id, details.near_joint))
                    } else {
                        Some((far_id, details.far_joint))
                    }
                } else {
                    // For left-clicks, we need to be careful to prevent traveling

                    // If one of the intervals is the same as the current one, we need to handle it specially
                    if near_is_same || far_is_same {
                        // If both intervals are available, pick the one that's NOT the same as the current one
                        // This ensures we don't travel to the other side with a left-click
                        if near_is_same && far_result.is_some() {
                            // The near interval is the same as current, so pick the far one
                            Some((far_id, details.far_joint))
                        } else if far_is_same && near_result.is_some() {
                            // The far interval is the same as current, so pick the near one
                            Some((near_id, details.near_joint))
                        } else {
                            // Only one interval is available, and it's the same as the current one
                            // In this case, we'll just return the current interval with the same joint
                            // to prevent traveling
                            if near_is_same {
                                Some((near_id, details.near_joint))
                            } else {
                                // far_is_same
                                Some((far_id, details.far_joint))
                            }
                        }
                    } else {
                        // Neither interval is the same as the current one
                        // Pick the one more aligned with the ray
                        if near_dot > far_dot {
                            Some((near_id, details.near_joint))
                        } else {
                            Some((far_id, details.far_joint))
                        }
                    }
                }
            }
        }
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
        G: Fn(&f32) -> bool,
    {
        // Use the ray origin that was calculated in pick_ray
        let ray_origin = self.last_ray_origin;

        items
            .map(|item| {
                let position = get_position(&item);

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

                (item, score)
            })
            .filter(|(_, score)| dot_filter(score))
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(item, _)| item)
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
