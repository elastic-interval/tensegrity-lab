//! Connector dimensions and the pure geometry helpers shared by attachment
//! assignment, rendering, and export.
//!
//! The connector (see `docs/connectors.md`) is a flat steel ring that turns on
//! an axial bolt at the strut end, carrying a radial boss that ends in a
//! cross-tube. The cable's fork pivots on a pin through that tube, so a cable
//! can take any orientation: azimuth from the ring turning on the bolt,
//! elevation from the fork pivoting on the pin. Every connector is
//! geometrically identical — there are no per-position variants.

use crate::connector::strut::{TubeSet, OPEN_CLAW_TUBES};
use crate::units::{Degrees, Meters, Unit};
use glam::Vec3;

/// Connector dimensions for physical construction.
#[derive(Clone, Debug)]
pub struct ConnectorDimensions {
    /// Ring thickness along the bolt axis (t_ring).
    pub ring_thickness: Meters,
    /// Divider washer between cap and first ring and between adjacent rings,
    /// so steel never bears on steel. Slot step = t_ring + washer.
    pub washer_thickness: Meters,
    /// The strut cap's disc, from the strut end to the first washer. The
    /// bolt is turned into its thread from inside the cap's tube.
    pub cap_thickness: Meters,
    /// Radial distance from the strut/bolt axis to the cross-tube (pin) axis
    /// where the cable attaches (R_tube — the cable's moment arm).
    pub pivot_radius: Meters,
    /// The tubes the struts are made from (`connector::strut`).
    pub tube_sets: &'static [TubeSet],
}

impl Default for ConnectorDimensions {
    fn default() -> Self {
        Self {
            ring_thickness: Meters(0.005),
            washer_thickness: Meters(0.002), // POM
            cap_thickness: Meters(0.005),
            pivot_radius: Meters(0.030), // ring OD 40/2 + 1.4 clearance + tube 17.2/2
            tube_sets: OPEN_CLAW_TUBES,
        }
    }
}

impl ConnectorDimensions {
    /// Axial offset (strut end → ring centre) for 0-indexed `slot`.
    /// = cap + washer + t_ring/2 + slot × (t_ring + washer).
    pub fn ring_center_offset(&self, slot: usize) -> Meters {
        let step = self.ring_thickness + self.washer_thickness;
        self.cap_thickness + self.washer_thickness + self.ring_thickness / 2.0
            + step * slot as f32
    }

    pub fn ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize) -> Vec3 {
        push_end + push_axis * self.ring_center_offset(slot).f32()
    }

    /// `(pivot_pos, elevation_deg)`: the pin position where the cable's fork
    /// pivots, and the free elevation angle the fork takes toward the cable's
    /// far end (0° = radial, positive tilts outward along the strut axis).
    pub fn pivot_geometry(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, f32) {
        self.pivot_geometry_reaching(push_end, push_axis, slot, pull_other_end, self.pivot_radius)
    }

    /// `pivot_geometry` for a boss reaching `reach` from the axis instead of
    /// the standard ring's `pivot_radius`.
    pub fn pivot_geometry_reaching(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
        reach: Meters,
    ) -> (Vec3, f32) {
        let ring_center = self.ring_center(push_end, push_axis, slot);
        Self::pivot_from_center(ring_center, push_axis, pull_other_end, reach)
    }

    /// `pivot_geometry` for a ring already placed at `ring_center`.
    pub fn pivot_from_center(ring_center: Vec3, push_axis: Vec3, pull_other_end: Vec3, reach: Meters) -> (Vec3, f32) {
        let to_pull = pull_other_end - ring_center;
        let radial_unit = radial_unit_from_axis(push_axis, to_pull);

        let pivot_pos = ring_center + radial_unit * reach.f32();

        let pull_direction = (pull_other_end - pivot_pos).normalize();
        let elevation_deg = pull_direction.dot(push_axis).asin().to_degrees();

        (pivot_pos, elevation_deg)
    }
}

const NEAR_PARALLEL_THRESHOLD: f32 = 1e-10;
const AXIS_ALIGNMENT_THRESHOLD: f32 = 0.9;

pub fn pivot_angle(push_axis: Vec3, pull_direction: Vec3) -> Degrees {
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

#[cfg(test)]
mod pivot_geometry_tests {
    use super::*;
    use crate::units::MM_PER_METER;
    use glam::Vec3;

    const MM: f32 = MM_PER_METER;
    const TOL: f32 = 0.001; // 1 micron tolerance

    fn assert_mm(label: &str, actual_m: f32, expected_mm: f32) {
        let actual_mm = actual_m * MM;
        let diff = (actual_mm - expected_mm).abs();
        assert!(
            diff < TOL * MM,
            "{}: expected {:.3}mm, got {:.3}mm (diff {:.4}mm)",
            label, expected_mm, actual_mm, diff
        );
    }

    /// Test that ConnectorDimensions formulas produce the correct derived values.
    #[test]
    fn connector_dimension_formulas() {
        let h = ConnectorDimensions::default();
        let t_ring = h.ring_thickness.f32(); // 5mm
        let washer = h.washer_thickness.f32(); // 2mm
        let cap = h.cap_thickness.f32(); // 5mm
        let r_pivot = h.pivot_radius.f32(); // 32mm

        // ring_center_offset(0) = cap + washer + t_ring/2
        assert_mm(
            "ring_center_offset(0) = cap + washer + t_ring/2",
            h.ring_center_offset(0).f32(),
            (cap + washer + t_ring / 2.0) * MM,
        );

        // ring_center_offset(1) - ring_center_offset(0) = t_ring + washer
        assert_mm(
            "ring_center_offset(1) - offset(0) = t_ring + washer",
            h.ring_center_offset(1).f32() - h.ring_center_offset(0).f32(),
            (t_ring + washer) * MM,
        );

        // Print summary for engineer verification
        println!("\n=== Connector dimension check (mm) ===");
        println!("t_ring (ring_thickness):   {:.1}", t_ring * MM);
        println!("washer (washer_thickness): {:.1}", washer * MM);
        println!("cap_thickness:             {:.1}", cap * MM);
        println!("pivot_radius (R_tube):     {:.1}", r_pivot * MM);
        println!();
        println!("ring_center_offset(0):     {:.1}", h.ring_center_offset(0).f32() * MM);
        println!("ring_center_offset(1):     {:.1}", h.ring_center_offset(1).f32() * MM);
    }

    /// Test that the 3D positions produced by ring_center / pivot_geometry
    /// have the exact distances the engineer expects to measure between them.
    #[test]
    fn pivot_geometry_distances() {
        let h = ConnectorDimensions::default();

        // Synthetic push interval along +Z axis
        let push_end = Vec3::ZERO;
        let push_axis = Vec3::Z;
        // Pull cable going roughly radially outward in +X
        let pull_other_end = Vec3::new(1.0, 0.0, 0.2);

        // --- Axial distances (along push axis) ---

        let rc0 = h.ring_center(push_end, push_axis, 0);
        let rc1 = h.ring_center(push_end, push_axis, 1);
        let rc2 = h.ring_center(push_end, push_axis, 2);

        // Push end to first ring center
        let axial_0 = (rc0 - push_end).length();
        assert_mm("push_end → ring_center(0)", axial_0, h.ring_center_offset(0).f32() * MM);

        // Between consecutive ring centers = t_ring + washer
        let step = (h.ring_thickness.f32() + h.washer_thickness.f32()) * MM;
        let ring_step = (rc1 - rc0).length();
        assert_mm("ring_center(0) → ring_center(1) = t_ring + washer", ring_step, step);

        let ring_step_2 = (rc2 - rc1).length();
        assert_mm("ring_center(1) → ring_center(2) = t_ring + washer", ring_step_2, step);

        // --- Radial distance (ring center to pivot pin) = pivot_radius ---

        let (pivot_pos, elevation_deg) =
            h.pivot_geometry(push_end, push_axis, 0, pull_other_end);

        let radial_dist = (pivot_pos - rc0).length();
        assert_mm("ring_center → pivot_pos = pivot_radius", radial_dist,
                  h.pivot_radius.f32() * MM);

        // The fork pivots freely, so the reported elevation must equal the
        // actual angle between the cable direction and the radial plane.
        let pull_direction = (pull_other_end - pivot_pos).normalize();
        let expected_deg = pull_direction.dot(push_axis).asin().to_degrees();
        assert!(
            (elevation_deg - expected_deg).abs() < 1e-4,
            "elevation: expected {:.4}°, got {:.4}°",
            expected_deg, elevation_deg
        );

        // Print summary for engineer
        println!("\n=== Geometry distance check (mm) ===");
        println!("push_end → ring_center(0):       {:.3}", axial_0 * MM);
        println!("ring_center(0) → ring_center(1): {:.3} (= t_ring + washer)", ring_step * MM);
        println!("ring_center → pivot_pos:         {:.3} (= pivot_radius)", radial_dist * MM);
        println!("elevation at pivot:              {:.2}°", elevation_deg);
    }
}
