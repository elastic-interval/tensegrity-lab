use std::collections::HashSet;
use std::f32::consts::PI;

use cgmath::{Deg, EuclideanSpace, InnerSpace, Matrix4, perspective, Point3, point3, Quaternion, Rad, Rotation, Rotation3, SquareMatrix, Transform, vec3, Vector3};
use cgmath::num_traits::abs;
use winit::dpi::PhysicalSize;
use winit_input_helper::WinitInputHelper;

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
    pub size: PhysicalSize<f32>,
    pub pick_mode: bool,
    pub multiple: bool,
    pub pick_cursor: Option<(f32, f32)>,
}

impl Camera {
    pub fn new(position: Point3<f32>, size: PhysicalSize<f32>) -> Self {
        Self {
            position,
            target: Target::default(),
            look_at: point3(0.0, 3.0, 0.0),
            picked: None,
            size,
            pick_mode: false,
            multiple: false,
            pick_cursor: None,
        }
    }

    pub fn handle_input(&mut self, input: &WinitInputHelper, fabric: &Fabric) {
        if input.mouse_held(0) {
            if let Some(rotation) = self.rotation(input.mouse_diff()) {
                self.position = self.look_at - rotation.transform_vector(self.look_at - self.position);
                self.pick_cursor = None;
            }
        }
        self.pick_mode = false; // TODO
        self.multiple = input.held_shift();
        if input.mouse_pressed(0) {
            self.pick_cursor = input.cursor();
        }
        if input.mouse_released(0) {
            if let Some(pick_cursor) = self.pick_cursor {
                self.pick(pick_cursor, self.multiple, fabric)
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

    pub fn set_size(&mut self, size: PhysicalSize<f32>) {
        self.size = size;
    }

    pub fn mvp_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn pick(&mut self, (px, py): (f32, f32), multiple: bool, fabric: &Fabric) {
        let width = self.size.width / 2.0;
        let height = self.size.height / 2.0;
        let x = (px - width) / width;
        let y = (height - py) / height;
        let position = Point3::new(x, y, 1.0);
        let point3d = self.mvp_matrix().invert().unwrap().transform_point(position);
        let ray = (point3d - self.position).normalize();
        let best = fabric.faces.iter()
            .map(|(face_id, face)|
                (face_id, (face.midpoint(fabric) - self.position.to_vec()).normalize().dot(ray)))
            .max_by(|(_, dot_a), (_, dot_b)| dot_a.total_cmp(dot_b));
        if let Some((face_id, _)) = best {
            self.picked = Some(Pick { face_id: *face_id, multiple });
            println!("Picked {:?}", face_id);
        } else {
            println!("Nothing picked");
        }
    }

    fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.look_at, Vector3::unit_y())
    }

    fn projection_matrix(&self) -> Matrix4<f32> {
        let aspect = self.size.width / self.size.height;
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
