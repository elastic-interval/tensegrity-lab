//! Connector dimensions and the pure geometry helpers shared by attachment
//! assignment, rendering, and CSV export.

use crate::connector::{attachment, bend_optimizer};
use crate::units::{Degrees, Meters, Unit};
use glam::Vec3;

/// Connector dimensions for physical construction.
#[derive(Clone, Debug)]
pub struct ConnectorDimensions {
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
    /// Radial distance from tube axis to tab pin. The strut tube radius is a
    /// fabric-level dimension (it exists without connectors), so it is passed in.
    pub fn offset(&self, push_radius: Meters) -> Meters {
        push_radius + self.push_radius_margin + self.disc_thickness / 2.0
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

    pub fn ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize) -> Vec3 {
        push_end + push_axis * self.disc_center_offset(slot).f32()
    }

    /// `(tab_pos, tab_bend, pull_end_pos, ideal_deg)`. `tab_bend` is snapped when
    /// `bend_magnitudes` is populated, else equals `ideal_deg`.
    pub fn tab_geometry(
        &self,
        push_radius: Meters,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, attachment::TabBend, Vec3, f32) {
        let ring_center = self.ring_center(push_end, push_axis, slot);
        let to_pull = pull_other_end - ring_center;
        let radial_unit = radial_unit_from_axis(push_axis, to_pull);

        let tab_pos = ring_center + radial_unit * self.offset(push_radius).f32();

        let pull_direction = (pull_other_end - tab_pos).normalize();
        let sin_angle = pull_direction.dot(push_axis);
        let ideal_deg = sin_angle.asin().to_degrees();

        let snapped_deg = if self.bend_magnitudes.is_empty() {
            ideal_deg
        } else {
            bend_optimizer::snap_to_magnitudes(ideal_deg, &self.bend_magnitudes).0
        };
        let tab_bend = attachment::TabBend(snapped_deg);

        let pull_end_pos =
            tab_bend.endpoint(tab_pos, push_axis, radial_unit, self.length().f32());

        (tab_pos, tab_bend, pull_end_pos, ideal_deg)
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

#[cfg(test)]
mod tab_geometry_tests {
    use super::*;
    use crate::fabric::FabricDimensions;
    use glam::Vec3;

    const MM: f32 = 1000.0;
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
    /// These are the numbers shown in the CSV header as "Afgeleide waarden".
    #[test]
    fn connector_dimension_formulas() {
        let dims = FabricDimensions::default();
        let h = ConnectorDimensions::default();
        let a = dims.push_radius.f32();   // 25mm
        let b = h.push_radius_margin.f32(); // 2mm
        let t1 = h.disc_thickness.f32();   // 6mm
        let t2 = h.disc_separator_thickness.f32(); // 1mm
        let cap = h.cap_thickness.f32();   // 6mm
        let d = h.tab_extension.f32();   // 14mm
        let e = h.tab_hole_diameter.f32(); // 12mm
        let c = t1 / 2.0;                 // 3mm

        // offset() = A + B + C (radial distance from tube axis to tab pin)
        assert_mm("offset = A+B+C", h.offset(dims.push_radius).f32(), (a + b + c) * MM);

        // length() = C + D + E (tab length from disc center to cable endpoint)
        assert_mm("length = C+D+E", h.length().f32(), (c + d + e) * MM);

        // disc_center_offset(0) = cap + t2 + t1/2
        assert_mm(
            "disc_center_offset(0) = cap+t2+t1/2",
            h.disc_center_offset(0).f32(),
            (cap + t2 + c) * MM,
        );

        // disc_center_offset(1) = disc_center_offset(0) + t1 + t2
        assert_mm(
            "disc_center_offset(1) - offset(0) = t1+t2",
            h.disc_center_offset(1).f32() - h.disc_center_offset(0).f32(),
            (t1 + t2) * MM,
        );

        // Print summary for engineer verification
        println!("\n=== Connector dimension check (mm) ===");
        println!("A  (push_radius):       {:.1}", a * MM);
        println!("B  (margin):            {:.1}", b * MM);
        println!("C  (t1/2):              {:.1}", c * MM);
        println!("D  (tab_extension):   {:.1}", d * MM);
        println!("E  (hole_diameter):     {:.1}", e * MM);
        println!("t1 (disc_thickness):    {:.1}", t1 * MM);
        println!("t2 (disc_separator):    {:.1}", t2 * MM);
        println!("cap_thickness:          {:.1}", cap * MM);
        println!();
        println!("A + B + C = offset():           {:.1}", h.offset(dims.push_radius).f32() * MM);
        println!("C + D + E = length():           {:.1}", h.length().f32() * MM);
        println!("t1 + t2:                        {:.1}", (t1 + t2) * MM);
        println!("disc_center_offset(0):          {:.1}", h.disc_center_offset(0).f32() * MM);
        println!("disc_center_offset(1):          {:.1}", h.disc_center_offset(1).f32() * MM);
    }

    /// Test that the 3D positions produced by ring_center / tab_geometry
    /// have the exact distances the engineer expects to measure between them.
    #[test]
    fn tab_geometry_distances() {
        let dims = FabricDimensions::default();
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

        // Push end to first disc center
        let axial_0 = (rc0 - push_end).length();
        assert_mm("push_end → ring_center(0)", axial_0, h.disc_center_offset(0).f32() * MM);

        // Between consecutive disc centers = t1 + t2
        let disc_step = (rc1 - rc0).length();
        assert_mm("ring_center(0) → ring_center(1) = t1+t2", disc_step,
                  (h.disc_thickness.f32() + h.disc_separator_thickness.f32()) * MM);

        let disc_step_2 = (rc2 - rc1).length();
        assert_mm("ring_center(1) → ring_center(2) = t1+t2", disc_step_2,
                  (h.disc_thickness.f32() + h.disc_separator_thickness.f32()) * MM);

        // --- Radial distance (ring center to tab pin) ---

        let (tab_pos, _bend, pull_end_pos, _ideal) =
            h.tab_geometry(dims.push_radius, push_end, push_axis, 0, pull_other_end);

        let radial_dist = (tab_pos - rc0).length();
        assert_mm("ring_center → tab_pos = offset() = A+B+C", radial_dist,
                  h.offset(dims.push_radius).f32() * MM);

        // --- Tab length (tab pin to cable endpoint) = C + D + E ---

        let tab_len = (pull_end_pos - tab_pos).length();
        assert_mm("tab_pos → pull_end_pos = length() = C+D+E", tab_len,
                  h.length().f32() * MM);

        // Print summary for engineer
        println!("\n=== Geometry distance check (mm) ===");
        println!("push_end → ring_center(0):     {:.3}", axial_0 * MM);
        println!("ring_center(0) → ring_center(1): {:.3} (= t1+t2)", disc_step * MM);
        println!("ring_center → tab_pos:       {:.3} (= A+B+C = offset)", radial_dist * MM);
        println!("tab_pos → pull_end_pos:      {:.3} (= C+D+E = length)", tab_len * MM);
    }
}
