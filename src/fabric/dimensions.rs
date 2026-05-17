//! Fabric and connector dimensions, plus the geometry helpers that turn a strut
//! endpoint and a cable direction into 3D positions.

use crate::fabric::physics::Physics;
use crate::fabric::material::Material;
use crate::fabric::{attachment, bend_optimizer};
use crate::units::{Degrees, Grams, GramsPerMeter, Meters, Unit};
use glam::Vec3;

/// Connector dimensions for physical construction.
#[derive(Clone, Debug)]
pub struct ConnectorDimensions {
    pub push_radius: Meters,
    pub push_radius_margin: Meters,
    pub disc_thickness: Meters,
    pub disc_separator_thickness: Meters,
    pub cap_thickness: Meters,
    pub tab_extension: Meters,
    pub tab_hole_diameter: Meters,
    pub bend_count: usize,
    /// Empty = no snap (use continuous ideal). Populated by `Fabric::recompute_bend_magnitudes`.
    pub bend_magnitudes: Vec<f32>,
    /// When true, `Fabric::recompute_bend_magnitudes` is a no-op and `bend_magnitudes`
    /// is taken as authoritative (e.g. matching a factory plate inventory already in
    /// production). Set via `FabricDimensions::with_locked_bend_magnitudes`.
    pub bend_magnitudes_locked: bool,
}

impl Default for ConnectorDimensions {
    fn default() -> Self {
        Self {
            push_radius: Meters(0.02),
            push_radius_margin: Meters(0.001),
            disc_thickness: Meters(0.005),
            disc_separator_thickness: Meters(0.002),
            cap_thickness: Meters(0.005),
            tab_extension: Meters(0.030),
            tab_hole_diameter: Meters(0.014),
            bend_count: 4,
            bend_magnitudes: Vec::new(),
            bend_magnitudes_locked: false,
        }
    }
}

impl ConnectorDimensions {
    pub fn offset(&self) -> Meters {
        self.push_radius + self.push_radius_margin + self.disc_thickness / 2.0
    }

    pub fn length(&self) -> Meters {
        self.disc_thickness / 2.0 + self.tab_extension + self.tab_hole_diameter
    }

    /// Axial offset (strut end → disc centre) for 0-indexed `slot`.
    /// = cap + separator + t1/2 + slot × (t1 + separator).
    pub fn disc_center_offset(&self, slot: usize) -> Meters {
        let step = self.disc_thickness + self.disc_separator_thickness;
        self.cap_thickness + self.disc_separator_thickness + self.disc_thickness / 2.0
            + step * slot as f32
    }
}

const NEAR_PARALLEL_THRESHOLD: f32 = 1e-10;
const AXIS_ALIGNMENT_THRESHOLD: f32 = 0.9;

pub fn tab_angle(push_axis: Vec3, pull_direction: Vec3) -> Degrees {
    let sin_angle = pull_direction.dot(push_axis);
    Degrees(sin_angle.asin().to_degrees())
}

pub(crate) fn radial_unit_from_axis(push_axis: Vec3, direction: Vec3) -> Vec3 {
    let axial_component = push_axis * direction.dot(push_axis);
    let radial_direction = direction - axial_component;

    if radial_direction.length_squared() < NEAR_PARALLEL_THRESHOLD {
        let arbitrary = if push_axis.x.abs() < AXIS_ALIGNMENT_THRESHOLD {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        push_axis.cross(arbitrary).normalize()
    } else {
        radial_direction.normalize()
    }
}

/// All physical dimensions for a fabric: structure size and interval geometry.
#[derive(Clone, Debug)]
pub struct FabricDimensions {
    pub altitude: Meters,
    pub scale: Meters,
    pub pull_radius: Meters,
    pub connector: ConnectorDimensions,
    /// Head + per-joint hardware share. Back-calibrated to total mass; refine when the full parts list lands.
    pub joint_mass: Grams,
    pub push_density: GramsPerMeter,
    /// Combined linear density of telescoping inner tubes (one outer-length per strut). Mass-reporting only.
    pub inner_push_density: GramsPerMeter,
    /// Mass of one cable-end fork termination; counted twice per `Role::Pulling` interval.
    pub pull_end_mass: Grams,
}

impl Default for FabricDimensions {
    fn default() -> Self {
        Self {
            altitude: Meters(7.5),
            scale: Meters(1.0),
            pull_radius: Meters(0.007),
            connector: ConnectorDimensions::default(),
            joint_mass: Grams(1800.0),
            push_density: GramsPerMeter(800.0),
            inner_push_density: GramsPerMeter(560.0),
            pull_end_mass: Grams(160.0),
        }
    }
}

impl FabricDimensions {
    pub fn with_altitude(mut self, altitude: Meters) -> Self {
        self.altitude = altitude;
        self
    }

    pub fn with_scale(mut self, scale: Meters) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_joint_mass(mut self, mass: Grams) -> Self {
        self.joint_mass = mass;
        self
    }

    pub fn with_push_density(mut self, density: GramsPerMeter) -> Self {
        self.push_density = density;
        self
    }

    /// Lock the snap magnitudes to a fixed inventory (e.g. parts already in production).
    /// `Fabric::recompute_bend_magnitudes` becomes a no-op and these values are used
    /// for snapping at every CSV export. Values are whole non-negative degrees, sorted
    /// ascending; the optimiser's normal output respects the same shape.
    pub fn with_locked_bend_magnitudes(mut self, magnitudes: Vec<f32>) -> Self {
        self.connector.bend_count = magnitudes.len();
        self.connector.bend_magnitudes = magnitudes;
        self.connector.bend_magnitudes_locked = true;
        self
    }

    /// Linear density for a material type, using configurable push density.
    pub fn linear_density(&self, material: Material, physics: &Physics) -> GramsPerMeter {
        let base = match material {
            Material::Push => self.push_density,
            _ => material.base_linear_density(),
        };
        GramsPerMeter(base.0 * physics.mass_multiplier())
    }

    pub fn ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize) -> Vec3 {
        push_end + push_axis * self.connector.disc_center_offset(slot).f32()
    }

    pub fn tab_position(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> Vec3 {
        let (tab_pos, _, _, _) = self.tab_geometry(push_end, push_axis, slot, pull_other_end);
        tab_pos
    }

    /// `(tab_pos, tab_bend, pull_end_pos, ideal_deg)`. `tab_bend` is snapped when
    /// `bend_magnitudes` is populated, else equals `ideal_deg`.
    pub fn tab_geometry(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, attachment::TabBend, Vec3, f32) {
        let ring_center = self.ring_center(push_end, push_axis, slot);
        let to_pull = pull_other_end - ring_center;
        let radial_unit = radial_unit_from_axis(push_axis, to_pull);

        let tab_pos = ring_center + radial_unit * self.connector.offset().f32();

        let pull_direction = (pull_other_end - tab_pos).normalize();
        let sin_angle = pull_direction.dot(push_axis);
        let ideal_deg = sin_angle.asin().to_degrees();

        let snapped_deg = if self.connector.bend_magnitudes.is_empty() {
            ideal_deg
        } else {
            bend_optimizer::snap_to_magnitudes(ideal_deg, &self.connector.bend_magnitudes).0
        };
        let tab_bend = attachment::TabBend(snapped_deg);

        let pull_end_pos =
            tab_bend.endpoint(tab_pos, push_axis, radial_unit, self.connector.length().f32());

        (tab_pos, tab_bend, pull_end_pos, ideal_deg)
    }

}
