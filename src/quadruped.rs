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

    /// Seed(4), four faces down, a leg on each, its feet pulled toward each
    /// other to `feet` of their distance.
    fn four_down(legs: usize, feet: Pct) -> FabricPlan {
        let leg = |face: FaceName, label| on(face).column(legs).shrink_by(Pct(13.0)).prism(Pct(400.0)).label(label);
        FabricName::Quadruped
            .build(dimensions())
            .seed(OmniSymmetrical, Seed(4))
            .faces([
                leg(LeftFrontBottom, Foot(Side::Left)),
                leg(RightFrontBottom, Foot(Side::Right)),
                leg(LeftBackBottom, HindFoot(Side::Left)),
                leg(RightBackBottom, HindFoot(Side::Right)),
            ])
            .prepare_vulcanize(0.5, VulcanizeMode::Linear)
            .space(Sec(2.8), FEET, feet)
            .vulcanize(Sec(2.1))
            .down(Sec(1.0), FEET)
            .pretense(Sec(0.3), Pct(0.5))
            .surface_frozen()
            .fall(Sec(1.5))
            .settle(Sec(10.0))
    }

    /// Seed(2), Headless Hug's seed: legs off the four side faces, which
    /// face fore and aft, front and hind legs of their own lengths, bent down
    /// by pulling each foot toward the lower face on its side, and the feet
    /// of each pair optionally pulled toward each other, `across`.
    fn legs_apart(front: usize, hind: usize, under: Pct, feet: Pct, across: Option<Pct>) -> FabricPlan {
        let leg = |face: FaceName, legs: usize, label| {
            on(face).column(legs).shrink_by(Pct(13.0)).prism(Pct(400.0)).label(label)
        };
        let builder = FabricName::Quadruped
            .build(dimensions())
            .seed(OmniSymmetrical, Seed(2))
            .faces([
                leg(ForeLeft, front, Foot(Side::Left)).into(),
                leg(ForeRight, front, Foot(Side::Right)).into(),
                leg(AftLeft, hind, HindFoot(Side::Left)).into(),
                leg(AftRight, hind, HindFoot(Side::Right)).into(),
                on(LowerLeft).label(ChestLower(Side::Left)),
                on(LowerRight).label(ChestLower(Side::Right)),
            ])
            .prepare_vulcanize(0.5, VulcanizeMode::Linear)
            .space_parallel(
                Sec(2.8),
                [
                    spacer([Foot(Side::Left), ChestLower(Side::Left)], under),
                    spacer([Foot(Side::Right), ChestLower(Side::Right)], under),
                    spacer([HindFoot(Side::Left), ChestLower(Side::Left)], under),
                    spacer([HindFoot(Side::Right), ChestLower(Side::Right)], under),
                    spacer([Foot(Side::Left), HindFoot(Side::Left)], feet),
                    spacer([Foot(Side::Right), HindFoot(Side::Right)], feet),
                ],
            );
        let builder = match across {
            Some(across) => builder.space_parallel(
                Sec(2.0),
                [
                    spacer([Foot(Side::Left), Foot(Side::Right)], across),
                    spacer([HindFoot(Side::Left), HindFoot(Side::Right)], across),
                ],
            ),
            None => builder,
        };
        builder
            .vulcanize(Sec(2.1))
            .pretense(Sec(0.3), Pct(0.5))
            .surface_frozen()
            .fall(Sec(1.5))
            .settle(Sec(10.0))
    }

    /// The Torque seed with four corner faces down, a leg on each. `along`
    /// runs the figure along the brick's long struts (its Left and Right
    /// ends become front and back); otherwise across them.
    fn torque(legs: usize, along: bool, feet: Option<Pct>) -> FabricPlan {
        let leg = |face: FaceName, label| on(face).column(legs).shrink_by(Pct(13.0)).prism(Pct(400.0)).label(label);
        let (fore_left, fore_right, hind_left, hind_right) = if along {
            (LeftFrontBottom, LeftBackBottom, RightFrontBottom, RightBackBottom)
        } else {
            (LeftFrontBottom, RightFrontBottom, LeftBackBottom, RightBackBottom)
        };
        let builder = FabricName::Quadruped
            .build(dimensions())
            .seed(TorqueSymmetrical, Seed(4))
            .faces([
                leg(fore_left, Foot(Side::Left)),
                leg(fore_right, Foot(Side::Right)),
                leg(hind_left, HindFoot(Side::Left)),
                leg(hind_right, HindFoot(Side::Right)),
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
        report("omni", legs_apart(2, 2, Pct(40.0), Pct(80.0), None), folder);
        report("omni-four-down", four_down(2, Pct(60.0)), folder);
        report("torque-along-2", torque(2, true, None), folder);
        report("torque-along-2-55", torque(2, true, Some(Pct(55.0))), folder);
        report("torque-along-3-70", torque(3, true, Some(Pct(70.0))), folder);
        report("torque-along-2-gathered", torque(2, true, Some(Pct(70.0))), folder);
    }
}
