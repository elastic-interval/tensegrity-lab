#[cfg(test)]
mod tests {
    use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName, FaceName::*, Side};
    use crate::build::dsl::fabric_dsl::*;
    use crate::build::dsl::fabric_library::{quadruped_plan, FabricName};
    use crate::build::dsl::fabric_plan::FabricPlan;
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::connector::strut::HEADLESS_HUG_TUBES;
    use crate::fabric::dimensions::Hardware;
    use crate::fabric::interval::Role;
    use crate::fabric::Fabric;
    use crate::units::{Seconds, Unit};
    use crate::Age;
    use crate::wgpu::figure;
    use glam::Vec3;
    use std::path::Path;

    fn dimensions() -> FabricDimensions {
        FabricDimensions::default()
            .with_altitude(M(4.0))
            .with_scale(M(0.75))
            .with_connector()
            .with_tube_sets(HEADLESS_HUG_TUBES)
            .with_connectors_shown()
            .with_hardware(Hardware::OPEN_CLAW)
    }

    const FEET: [crate::build::dsl::brick_dsl::FaceLabel; 4] =
        [Foot(Side::Left), Foot(Side::Right), HindFoot(Side::Left), HindFoot(Side::Right)];

    /// The Torque seed with four corner faces down, its long struts nose to
    /// tail, a leg on each corner, the feet optionally drawn toward each
    /// other to `feet` of their distance.
    fn torque(legs: usize, feet: Option<Pct>) -> FabricPlan {
        let leg = |face: FaceName, label| on(face).column(legs).shrink_by(Pct(13.0)).prism(Pct(400.0)).label(label);
        let builder = FabricName::Quadruped
            .build(dimensions())
            .seed(TorqueSymmetrical, Seed(4))
            .faces([
                leg(LeftFrontBottom, Foot(Side::Left)),
                leg(LeftBackBottom, Foot(Side::Right)),
                leg(RightFrontBottom, HindFoot(Side::Left)),
                leg(RightBackBottom, HindFoot(Side::Right)),
            ])
            .prepare_vulcanize(0.5, VulcanizeMode::Linear);
        let builder = match feet {
            Some(feet) => builder.space(Sec(2.8), FEET, feet),
            None => builder.vulcanize(Sec(0.0)),
        };
        builder
            .vulcanize(Sec(2.1))
            .down(Sec(1.0), FEET)
            .pretense(Sec(0.3), Pct(0.5))
            .surface_frozen()
            .fall(Sec(1.5))
            .settle(Sec(10.0))
    }

    fn report(name: &str, plan: FabricPlan, folder: &Path) {
        let mut executor = FabricPlanExecutor::new(plan);
        let mut iterations = 0u64;
        while !executor.is_complete() && iterations < 5_000_000 {
            let _ = executor.iterate();
            iterations += 1;
        }
        let unsymmetric: Vec<Vec3> = executor.fabric.joints.values().map(|j| j.location).collect();
        executor.fabric.update_all_attachment_connections();
        let fabric: &Fabric = &executor.fabric;
        let connector = fabric.connector.as_ref().expect("connector");
        let mut unfit: Vec<String> = fabric
            .intervals
            .iter()
            .filter(|(_, i)| i.has_role(Role::Pushing))
            .filter_map(|(key, _)| {
                let [(alpha, _), (omega, _)] = connector.stack_ends(fabric, key);
                let length = alpha.distance(omega);
                let (_, _, fits) = crate::connector::strut::tube_set(connector.dimensions.tube_sets, crate::units::Meters(length));
                (!fits).then(|| format!("{length:.2}"))
            })
            .collect();
        unfit.sort();
        eprintln!("{name}: struts no tube fits {unfit:?}");
        let off_mirror = unsymmetric.iter().zip(fabric.joints.values()).map(|(a, j)| a.distance(j.location)).fold(0.0, f32::max);
        let struts = fabric.intervals.values().filter(|i| i.has_role(Role::Pushing)).count();
        let tube: f32 = fabric
            .intervals
            .values()
            .filter(|i| i.has_role(Role::Pushing))
            .map(|i| fabric.joints[i.alpha_key].location.distance(fabric.joints[i.omega_key].location))
            .sum();
        let feet: Vec<String> = fabric.connector.as_ref().map_or(vec![], |c| {
            let mut names: Vec<String> = c.feet.iter().map(|&j| fabric.joint_label(j)).collect();
            names.sort();
            names
        });
        let (low, high) = fabric.altitude_range();
        let mass: f32 = fabric.joints.values().map(|j| j.accumulated_mass.f32()).sum();
        let gravity = fabric.joints.values().map(|j| j.location * j.accumulated_mass.f32()).sum::<Vec3>() / mass;
        let foot_spots: Vec<Vec3> = fabric
            .connector
            .as_ref()
            .map_or(vec![], |c| c.feet.iter().map(|&j| fabric.joints[j].location).collect());
        let (min_x, max_x, min_z, max_z) = foot_spots.iter().fold(
            (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
            |(a, b, c, d), p| (a.min(p.x), b.max(p.x), c.min(p.z), d.max(p.z)),
        );
        eprintln!(
            "{name}: off mirror {:.0} mm, complete {} failures {:?}, {struts} struts {tube:.1} m, mass {:.0} kg, height {:.2} m, feet {feet:?}, \
             footprint x {:.2}..{:.2} z {:.2}..{:.2}, CoG ({:.2}, {:.2}, {:.2})",
            off_mirror * 1000.0,
            executor.is_complete(),
            executor.plan_failures(),
            mass / 1000.0,
            high - low,
            min_x, max_x, min_z, max_z,
            gravity.x, gravity.y, gravity.z,
        );
        figure::write_figures(fabric, &folder.join(name));
    }

    /// Quadruped stands on its four feet: four strut ends on the ground,
    /// its centre of gravity inside them, and at rest.
    #[test]
    fn quadruped_stands_on_four_feet() {
        let mut executor = FabricPlanExecutor::new(quadruped_plan());
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let fabric = &executor.fabric;
        let feet: Vec<Vec3> = fabric.connector.as_ref().expect("connector").feet.iter().map(|&j| fabric.joints[j].location).collect();
        assert_eq!(feet.len(), 4, "standing on {} feet", feet.len());
        let mass: f32 = fabric.joints.values().map(|j| j.accumulated_mass.f32()).sum();
        let gravity = fabric.joints.values().map(|j| j.location * j.accumulated_mass.f32()).sum::<Vec3>() / mass;
        let centre = feet.iter().sum::<Vec3>() / 4.0;
        let mut around = feet.clone();
        around.sort_by(|a, b| {
            let angle = |p: &Vec3| (p.z - centre.z).atan2(p.x - centre.x);
            angle(a).total_cmp(&angle(b))
        });
        for (i, a) in around.iter().enumerate() {
            let b = around[(i + 1) % 4];
            let side = (b.x - a.x) * (gravity.z - a.z) - (b.z - a.z) * (gravity.x - a.x);
            let centre_side = (b.x - a.x) * (centre.z - a.z) - (b.z - a.z) * (centre.x - a.x);
            assert!(side * centre_side > 0.0, "centre of gravity outside the feet");
        }
        let before: Vec<Vec3> = executor.fabric.joints.values().map(|j| j.location).collect();
        let physics = executor.physics.clone();
        for _ in 0..(Seconds(5.0) / Age::iteration_duration()) as usize {
            executor.fabric.iterate(&physics);
        }
        let moved = before.iter().zip(executor.fabric.joints.values()).map(|(a, j)| a.distance(j.location)).fold(0.0, f32::max);
        assert!(moved < 0.01, "still moving {:.1} mm", moved * 1000.0);
    }

    /// ```
    /// QUADRUPED_FIGURES=<folder> cargo test --release --lib quadruped_candidates -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn quadruped_candidates() {
        let folder = std::env::var("QUADRUPED_FIGURES").expect("QUADRUPED_FIGURES");
        let folder = Path::new(&folder);
        report("quadruped", quadruped_plan(), folder);
        report("legs-free", torque(2, None), folder);
        report("feet-70", torque(2, Some(Pct(70.0))), folder);
        report("legs-3", torque(3, Some(Pct(50.0))), folder);
    }
}
