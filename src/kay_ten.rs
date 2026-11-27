#[cfg(test)]
mod tests {
    use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName, FaceName::*, Side};
    use crate::build::dsl::fabric_dsl::*;
    use crate::build::dsl::fabric_library::{kay_ten_plan, FabricName, KayTenShape};
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
        torque_like(TorqueSymmetrical, legs, feet)
    }

    /// `torque` on any brick with Torque's faces: Torque or Tork.
    fn torque_like(brick: crate::build::dsl::brick_dsl::BrickName, legs: usize, feet: Option<Pct>) -> FabricPlan {
        let leg = |face: FaceName, label| on(face).column(legs).shrink_by(Pct(13.0)).prism(Pct(400.0)).label(label);
        let builder = FabricName::KayTen
            .build(dimensions())
            .seed(brick, Seed(4))
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

    /// Tork with a leg on each corner and Headless Hug's kind of spacers:
    /// each front foot to the hind foot on its side (`along`), front foot to
    /// front foot and hind to hind (`across`), and the diagonals (`crossed`).
    fn tork_spaced(along: Pct, across: Pct, crossed: Pct) -> FabricPlan {
        let leg = |face: FaceName, label| on(face).column(2).shrink_by(Pct(13.0)).prism(Pct(400.0)).label(label);
        let (fore_left, fore_right) = (Foot(Side::Left), Foot(Side::Right));
        let (hind_left, hind_right) = (HindFoot(Side::Left), HindFoot(Side::Right));
        FabricName::KayTen
            .build(dimensions())
            .seed(Tork, Seed(4))
            .faces([
                leg(LeftFrontBottom, fore_left),
                leg(LeftBackBottom, fore_right),
                leg(RightFrontBottom, hind_left),
                leg(RightBackBottom, hind_right),
            ])
            .prepare_vulcanize(0.5, VulcanizeMode::Linear)
            .space_parallel(
                Sec(2.8),
                [
                    spacer([fore_left, hind_left], along),
                    spacer([fore_right, hind_right], along),
                    spacer([fore_left, fore_right], across),
                    spacer([hind_left, hind_right], across),
                    spacer([fore_left, hind_right], crossed),
                    spacer([fore_right, hind_left], crossed),
                ],
            )
            .vulcanize(Sec(2.1))
            .down(Sec(1.0), FEET)
            .pretense(Sec(0.3), Pct(0.5))
            .surface_frozen()
            .fall(Sec(1.5))
            .settle(Sec(10.0))
    }

    /// Where a plan's feet land: (length nose to tail, width), the height,
    /// colliding cable ends, and whether it stands on four feet with its
    /// centre of gravity among them.
    fn stance(plan: FabricPlan) -> (f32, f32, f32, usize, bool, f32) {
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let fabric = &executor.fabric;
        let connector = fabric.connector.as_ref().expect("connector");
        let feet: Vec<Vec3> = connector.feet.iter().map(|&j| fabric.joints[j].location).collect();
        let (low, high) = fabric.altitude_range();
        let extent = |f: fn(&Vec3) -> f32| {
            let values = feet.iter().map(f);
            values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
        };
        let mass: f32 = fabric.joints.values().map(|j| j.accumulated_mass.f32()).sum();
        let gravity = fabric.joints.values().map(|j| j.location * j.accumulated_mass.f32()).sum::<Vec3>() / mass;
        let centre = feet.iter().sum::<Vec3>() / feet.len().max(1) as f32;
        let standing = feet.len() == 4 && (gravity.x - centre.x).abs() < 0.1 && (gravity.z - centre.z).abs() < 0.1;
        let hardware = fabric.dimensions.hardware.expect("real parts");
        let peak = fabric
            .intervals
            .values()
            .filter(|i| i.role.is_pull_like())
            .map(|i| (hardware.cable_stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0)
            .fold(0.0, f32::max);
        (extent(|p| p.z), extent(|p| p.x), high - low, connector.culprits.len(), standing, peak)
    }

    /// Kay Ten's spacers, its hinges and bridles in place: how long and
    /// wide the feet land, how many cable ends collide and the highest cable
    /// force, squarest first.
    ///
    /// ```
    /// cargo test --release --lib sweep_kay_ten_spacers -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn sweep_kay_ten_spacers() {
        let mut shapes = Vec::new();
        for along in [20.0, 35.0, 50.0] {
            for across in [145.0, 165.0, 185.0, 205.0] {
                for crossed in [95.0] {
                    shapes.push(KayTenShape { along: Pct(along), across: Pct(across), crossed: Pct(crossed) });
                }
            }
        }
        let results: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = shapes.iter().map(|&shape| scope.spawn(move || stance(kay_ten_plan(shape)))).collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut rows: Vec<_> = shapes.iter().zip(results).filter(|(_, r)| r.4).collect();
        rows.sort_by(|a, b| ((a.1.0 / a.1.1).ln().abs()).total_cmp(&(b.1.0 / b.1.1).ln().abs()));
        for (shape, (length, width, height, culprits, _, peak)) in rows {
            eprintln!(
                "along {:.0}% across {:.0}% crossed {:.0}%: {length:.2} x {width:.2} m, height {height:.2} m, {culprits} colliding ends, peak cable {peak:.1} kN",
                shape.along.0, shape.across.0, shape.crossed.0
            );
        }
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
                let strut = &fabric.intervals[key];
                (!fits).then(|| format!("{length:.2} {}-{}", fabric.joint_label(strut.alpha_key), fabric.joint_label(strut.omega_key)))
            })
            .collect();
        unfit.sort();
        let longest = fabric
            .intervals
            .iter()
            .filter(|(_, i)| i.has_role(Role::Pushing))
            .map(|(key, _)| {
                let [(alpha, _), (omega, _)] = connector.stack_ends(fabric, key);
                alpha.distance(omega)
            })
            .fold(0.0, f32::max);
        eprintln!("{name}: longest strut {longest:.3} m, struts no tube fits {unfit:?}");
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

    /// Kay Ten walking on the spot: its two actuator cables' lengths
    /// through a cycle, how far it moves, and its highest cable force.
    ///
    /// ```
    /// cargo test --release --lib kay_ten_animation -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_animation() {
        use crate::build::animator::Animator;
        use crate::fabric::physics::presets::ANIMATING;
        use crate::fabric::physics::Physics;
        let plan = kay_ten_plan(KayTenShape::default());
        let animate = plan.animate_phase.clone().expect("the plan animates");
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let physics = Physics { surface: executor.physics.surface, ..ANIMATING };
        let fabric = &mut executor.fabric;
        fabric.update_all_attachment_connections();
        fabric.unfreeze();
        let period = animate.period;
        let before: Vec<Vec3> = fabric.joints.values().map(|j| j.location).collect();
        let feet: Vec<(crate::fabric::JointKey, Vec3)> = fabric
            .connector
            .as_ref()
            .expect("connector")
            .feet
            .iter()
            .map(|&foot| (foot, fabric.joints[foot].location))
            .collect();
        let (mut slid, mut lifted) = (0.0f32, 0.0f32);
        let mut animator = Animator::new(animate, fabric);
        let joint = |fabric: &Fabric, name: &str| fabric.joints.keys().find(|&j| fabric.joint_label(j) == name).expect(name);
        let pairs = [("R7", "L5"), ("L7", "R5"), ("R6", "L4"), ("L6", "R4")];
        let hardware = fabric.dimensions.hardware.expect("real parts");
        let mut peak = 0.0f32;
        let steps = 20;
        for step in 0..steps * 2 {
            animator.iterate(fabric, &physics, (period / Age::iteration_duration()) as usize / steps);
            let lengths: Vec<String> = pairs
                .iter()
                .map(|(a, b)| format!("{}–{} {:.3} m", a, b, fabric.distance(joint(fabric, a), joint(fabric, b)).f32()))
                .collect();
            peak = fabric
                .intervals
                .values()
                .filter(|i| i.role.is_pull_like())
                .map(|i| (hardware.cable_stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0)
                .fold(peak, f32::max);
            for &(foot, at) in &feet {
                let now = fabric.joints[foot].location;
                slid = slid.max(Vec3::new(now.x - at.x, 0.0, now.z - at.z).length());
                lifted = lifted.max(now.y - at.y);
            }
            if step >= steps {
                eprintln!("{:>3}%: {}", (step - steps) * 100 / steps, lengths.join(", "));
            }
        }
        let moved = before.iter().zip(fabric.joints.values()).map(|(a, j)| a.distance(j.location)).fold(0.0, f32::max);
        eprintln!("moved up to {:.0} mm from standing; peak cable {peak:.1} kN", moved * 1000.0);
        eprintln!("feet slid up to {:.1} mm and lifted up to {:.1} mm", slid * 1000.0, lifted * 1000.0);
    }

    /// How far a foot lifts, how far one slides, how far the figure sways
    /// and the peak cable, with the Kay Ten's animation at this strength,
    /// over three cycles from standing.
    fn animated(standing: &Fabric, surface: Option<crate::fabric::physics::Surface>, animate: crate::build::dsl::animate_phase::AnimatePhase) -> (f32, f32, f32, f32) {
        use crate::build::animator::Animator;
        use crate::fabric::physics::presets::ANIMATING;
        use crate::fabric::physics::Physics;
        let mut fabric = standing.clone();
        fabric.unfreeze();
        let physics = Physics { surface, ..ANIMATING };
        let feet: Vec<(crate::fabric::JointKey, Vec3)> =
            fabric.connector.as_ref().expect("connector").feet.iter().map(|&foot| (foot, fabric.joints[foot].location)).collect();
        assert_eq!(feet.len(), 4, "watching four feet");
        let before: Vec<Vec3> = fabric.joints.values().map(|j| j.location).collect();
        let hardware = fabric.dimensions.hardware.expect("real parts");
        let steps = 40;
        let per_step = (animate.period / Age::iteration_duration()) as usize / steps;
        let mut animator = Animator::new(animate, &mut fabric);
        let (mut lifted, mut slid, mut swayed, mut peak) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for _ in 0..steps * 3 {
            animator.iterate(&mut fabric, &physics, per_step);
            for &(foot, at) in &feet {
                let now = fabric.joints[foot].location;
                lifted = lifted.max(now.y - at.y);
                slid = slid.max(Vec3::new(now.x - at.x, 0.0, now.z - at.z).length());
            }
            swayed = before.iter().zip(fabric.joints.values()).map(|(a, j)| a.distance(j.location)).fold(swayed, f32::max);
            peak = fabric
                .intervals
                .values()
                .filter(|i| i.role.is_pull_like())
                .map(|i| (hardware.cable_stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0)
                .fold(peak, f32::max);
        }
        (lifted, slid, swayed, peak)
    }

    /// Just for fun: the animation stronger and stronger until a foot leaves
    /// the ground.
    ///
    /// ```
    /// cargo test --release --lib kay_ten_until_it_jumps -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_until_it_jumps() {
        let plan = kay_ten_plan(KayTenShape::default());
        let animate = plan.animate_phase.clone().expect("the plan animates");
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let surface = executor.physics.surface;
        executor.fabric.update_all_attachment_connections();
        let standing = executor.fabric;
        let mut trials = Vec::new();
        for hertz in [0.5, 1.0] {
            for stiffness in [100.0] {
                for amplitude in [10.0, 10.5, 11.0, 11.5, 12.0, 12.5] {
                    trials.push((hertz, stiffness, amplitude));
                }
            }
        }
        let results: Vec<(f32, f32, f32, f32)> = std::thread::scope(|scope| {
            let handles: Vec<_> = trials
                .iter()
                .map(|&(hertz, stiffness, amplitude)| {
                    let animate = crate::build::dsl::animate_phase::AnimatePhase {
                        period: Seconds(1.0 / hertz),
                        amplitude: crate::units::Percent(amplitude),
                        stiffness: crate::units::Percent(stiffness),
                        ..animate.clone()
                    };
                    let standing = &standing;
                    scope.spawn(move || animated(standing, surface, animate))
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        for ((hertz, _, amplitude), (lifted, slid, swayed, peak)) in trials.iter().zip(results) {
            eprintln!(
                "{hertz} Hz, stroke {amplitude:>4}%: foot lifts {:5.1} mm, slides {:5.1} mm, sway {:4.0} mm, peak cable {peak:5.1} kN",
                lifted * 1000.0,
                slid * 1000.0,
                swayed * 1000.0
            );
        }
    }

    /// A grippy floor holds a foot pressing on it, sideways too, and lets
    /// one rising from it go, never pulling it back down.
    #[test]
    fn grippy_floor_holds_and_lets_go() {
        use crate::fabric::physics::{Surface, SurfaceCharacter, SurfaceInteraction};
        let floor = Surface::new(SurfaceCharacter::Grippy, 1.0);
        let touching = |altitude: f32, velocity: Vec3| {
            floor.interact(SurfaceInteraction {
                altitude,
                velocity,
                force_velocity: Vec3::ZERO,
                drag: 0.0,
                viscosity: 0.0,
                mass: 1.0,
                dt: Age::iteration_duration().f32(),
            })
        };
        let pressing = touching(0.002, Vec3::new(0.3, -0.5, 0.2));
        assert_eq!(pressing.velocity, Vec3::ZERO, "a foot pressing on the floor stays put");
        let rising = touching(0.002, Vec3::new(0.3, 0.5, 0.2));
        assert_eq!(rising.velocity.y, 0.5, "a rising foot keeps rising");
        assert_eq!(rising.clamp_y, None, "a rising foot is left where it is");
        assert_eq!(touching(-0.001, Vec3::ZERO).clamp_y, Some(0.0), "a foot below the floor comes up to it");
    }

    /// Kay Ten must stay buildable, with standard parts but its tube
    /// plates and custom discs: no colliding connectors, no cable too short
    /// for its terminals, none slack, and none past a 6 mm cable's working
    /// load.
    #[test]
    fn kay_ten_is_buildable() {
        let mut executor = FabricPlanExecutor::new(kay_ten_plan(KayTenShape::default()));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let started = std::time::Instant::now();
        executor.fabric.update_all_attachment_connections();
        eprintln!("connectors worked out in {:.2} s", started.elapsed().as_secs_f32());
        let fabric = &executor.fabric;
        let connector = fabric.connector.as_ref().expect("connector");
        assert_eq!(
            (connector.culprits.len(), connector.short_cables.len()),
            (0, 0),
            "Kay Ten has build problems (culprit ends, short cables): rerun kay_ten_connector_problems"
        );
        let hardware = fabric.dimensions.hardware.expect("real parts");
        let forces: Vec<f32> = fabric
            .intervals
            .values()
            .filter(|i| i.role.is_pull_like())
            .map(|i| (hardware.cable_stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0)
            .collect();
        let slack: Vec<String> = fabric
            .intervals
            .values()
            .filter(|i| i.role.is_pull_like() && i.strain <= 0.0)
            .map(|i| format!("{}:{}", fabric.joint_label(i.alpha_key), fabric.joint_label(i.omega_key)))
            .collect();
        assert!(slack.is_empty(), "slack: {slack:?}");
        let peak = forces.iter().copied().fold(0.0, f32::max);
        assert!(peak <= 5.1, "a cable carries {peak:.1} kN");
    }

    /// Kay Ten stands on its four feet: four strut ends on the ground,
    /// its centre of gravity inside them, and at rest.
    #[test]
    fn kay_ten_stands_on_four_feet() {
        let mut executor = FabricPlanExecutor::new(kay_ten_plan(KayTenShape::default()));
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

    /// Kay Ten's connector problems: colliding cable ends by joint,
    /// short cables, and its custom discs.
    ///
    /// ```
    /// cargo test --release --lib kay_ten_connector_problems -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_connector_problems() {
        let mut executor = FabricPlanExecutor::new(kay_ten_plan(KayTenShape::default()));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let mut headless_hug = FabricPlanExecutor::new(crate::build::dsl::fabric_library::get_fabric_plan(FabricName::HeadlessHug));
        while !headless_hug.is_complete() {
            let _ = headless_hug.iterate();
        }
        let headless_hug = headless_hug.fabric;
        let fabric = &executor.fabric;
        let connector = fabric.connector.as_ref().expect("connector");
        let cable = |key| {
            let i = &fabric.intervals[key];
            format!("{}:{}", fabric.joint_label(i.alpha_key), fabric.joint_label(i.omega_key))
        };
        let mut collisions: Vec<String> = connector
            .collisions
            .iter()
            .map(|[(a, ja), (b, jb)]| format!("{} vs {} at {} / {}", cable(*a), cable(*b), fabric.joint_label(*ja), fabric.joint_label(*jb)))
            .collect();
        collisions.sort();
        eprintln!("{} culprit ends, {} collisions:", connector.culprits.len(), collisions.len());
        for c in &collisions {
            eprintln!("  {c}");
        }
        let mut short: Vec<String> = connector.short_cables.iter().map(|&k| cable(k)).collect();
        short.sort();
        eprintln!("short cables: {short:?}");
        let ends: Vec<_> = fabric
            .intervals
            .values()
            .filter(|i| i.has_role(Role::Pushing))
            .flat_map(|i| [(i.alpha_key, i.omega_key), (i.omega_key, i.alpha_key)])
            .collect();
        let mut close: Vec<(f32, String)> = Vec::new();
        for (n, &(a, a_other)) in ends.iter().enumerate() {
            for &(b, b_other) in &ends[n + 1..] {
                if b == a_other || a == b_other {
                    continue;
                }
                let gap = fabric.joints[a].location.distance(fabric.joints[b].location);
                close.push((gap, format!("{} - {}", fabric.joint_label(a), fabric.joint_label(b))));
            }
        }
        close.sort_by(|x, y| x.0.total_cmp(&y.0));
        eprintln!("closest strut ends: {:?}", close.iter().take(8).map(|(gap, pair)| format!("{pair} {:.0} mm", gap * 1000.0)).collect::<Vec<_>>());
        for (name, fabric) in [("Kay Ten", fabric), ("Headless Hug", &headless_hug)] {
            let hardware = fabric.dimensions.hardware.expect("real parts");
            let force = |i: &crate::fabric::interval::Interval| (hardware.cable_stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0;
            let (mut cables, mut hangers) = (0.0f32, 0.0f32);
            for (key, i) in fabric.intervals.iter().filter(|(_, i)| i.role.is_pull_like()) {
                if crate::connector::system::is_hanger(fabric, key) {
                    hangers = hangers.max(force(i));
                } else {
                    cables = cables.max(force(i));
                }
            }
            eprintln!("{name}: peak cable {cables:.2} kN, peak bridle hanger {hangers:.2} kN");
            let mut top: Vec<(f32, String)> = fabric
                .intervals
                .values()
                .filter(|i| i.role.is_pull_like())
                .map(|i| (force(i), format!("{}:{}", fabric.joint_label(i.alpha_key), fabric.joint_label(i.omega_key))))
                .collect();
            top.sort_by(|a, b| b.0.total_cmp(&a.0));
            eprintln!("  highest: {:?}", top.iter().take(8).map(|(f, n)| format!("{n} {f:.1}")).collect::<Vec<_>>());
        }
        for (&node, _) in &fabric.bridle_nodes {
            if let Some(plate) = connector.node_plate(node) {
                eprintln!("plate {}: {} holes, tilt {:.1}°, jammed {}", fabric.joint_label(node), plate.holes.len(), plate.tilt, plate.jammed);
                if plate.tubed {
                    for &(cable, hole) in &plate.holes {
                        if crate::connector::system::is_hanger(fabric, cable) {
                            continue;
                        }
                        let far = fabric.intervals[cable].other_joint(node);
                        let aim = connector.pull_end_pivot(fabric, cable, far).unwrap_or(fabric.joints[far].location);
                        let direction = (aim - hole).normalize();
                        let pin = plate.pin(cable).expect("a pin");
                        eprintln!(
                            "  {}: {:.1}° out of the plate, {:.2}° off square to its tube",
                            fabric.joint_label(far),
                            direction.dot(plate.normal).abs().asin().to_degrees(),
                            direction.dot(pin).abs().asin().to_degrees()
                        );
                    }
                }
            }
        }
        let mut culprits: Vec<String> = connector.culprits.iter().map(|&(k, j)| format!("{} at {}", cable(k), fabric.joint_label(j))).collect();
        culprits.sort();
        eprintln!("culprit ends: {culprits:?}");
        let placements = crate::connector::parts::placements(fabric);
        for (part, count) in crate::connector::parts::parts_list(&placements) {
            eprintln!("  {count} × {}", part.name());
        }
        for placement in &placements {
            let shape = match placement.part {
                crate::connector::parts::Part::NodePlate(shape) | crate::connector::parts::Part::TubePlate(shape) => Some(shape),
                _ => None,
            };
            if let Some(shape) = shape {
                let z = placement.x.cross(placement.y);
                let drawn: Vec<Vec3> = shape
                    .holes()
                    .iter()
                    .map(|&(degrees, mm)| {
                        let (sin, cos) = (degrees as f32).to_radians().sin_cos();
                        placement.origin + (placement.x * cos + z * sin) * (mm as f32 / 1000.0)
                    })
                    .collect();
                let node = fabric.bridle_nodes.keys().copied().find(|&n| {
                    connector.node_plate(n).is_some_and(|p| {
                        let m = p.holes.iter().map(|(_, h)| *h).sum::<Vec3>() / p.holes.len() as f32;
                        m.distance(placement.origin) < 1e-4 || (p.tubed && p.center.distance(placement.origin) < 1e-4)
                    })
                });
                if let Some(plate) = node.and_then(|n| connector.node_plate(n)) {
                    let worst = plate
                        .holes
                        .iter()
                        .map(|(_, pin)| drawn.iter().map(|d| d.distance(*pin)).fold(f32::MAX, f32::min))
                        .fold(0.0, f32::max);
                    eprintln!("plate with {} holes: worst pin {:.1} mm from a drawn hole", shape.count, worst * 1000.0);
                    if plate.tubed {
                        let square = plate
                            .holes
                            .iter()
                            .zip(&plate.pins)
                            .filter(|((cable, _), _)| !crate::connector::system::is_hanger(fabric, *cable))
                            .map(|((_, hole), pin)| (*hole - plate.center).normalize().dot(*pin).abs().asin().to_degrees())
                            .fold(0.0, f32::max);
                        eprintln!("  tubes at most {square:.2}° off square to their bosses");
                    }
                }
            }
        }
        for placement in placements {
            if let crate::connector::parts::Part::CustomDisc(shape) = placement.part {
                eprintln!("custom disc at {:?}: {:?}", placement.origin, shape.bosses());
            }
        }
    }

    /// The median and highest cable force, in kN, in each stage of a plan as
    /// that stage ends.
    fn forces_by_stage(plan: FabricPlan) -> Vec<(String, f32, f32)> {
        let measure = |fabric: &Fabric| {
            let hardware = fabric.dimensions.hardware.expect("real parts");
            let mut forces: Vec<f32> = fabric
                .intervals
                .values()
                .filter(|i| i.role.is_pull_like())
                .map(|i| (hardware.cable_stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0)
                .collect();
            forces.sort_by(f32::total_cmp);
            (forces[forces.len() / 2], forces[forces.len() - 1])
        };
        let mut executor = FabricPlanExecutor::new(plan);
        let mut stages = Vec::new();
        let mut stage = executor.stage().clone();
        while !executor.is_complete() {
            let _ = executor.iterate();
            if *executor.stage() != stage {
                let (median, peak) = measure(&executor.fabric);
                stages.push((format!("{stage:?}"), median, peak));
                stage = executor.stage().clone();
            }
        }
        let (median, peak) = measure(&executor.fabric);
        stages.push((format!("{stage:?}"), median, peak));
        stages
    }

    /// What meets at the joints named in `KAY_TEN_JOINTS` (comma
    /// separated): each interval there, its role and force.
    ///
    /// ```
    /// KAY_TEN_JOINTS=Z22,Z2 cargo test --release --lib kay_ten_joints -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_joints() {
        let names = std::env::var("KAY_TEN_JOINTS").expect("KAY_TEN_JOINTS");
        let mut executor = FabricPlanExecutor::new(kay_ten_plan(KayTenShape::default()));
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let fabric = &executor.fabric;
        let hardware = fabric.dimensions.hardware.expect("real parts");
        let at = |name: &str| {
            fabric.joints.keys().find(|&j| fabric.joint_label(j) == name).map(|j| fabric.joints[j].location)
        };
        if let (Some(l), Some(r)) = (at("Z2bL"), at("Z2bR")) {
            let node = (l + r) / 2.0;
            let directions: Vec<(String, Vec3)> = ["Z2", "L7", "R7", "L8", "R8"]
                .iter()
                .filter_map(|n| at(n).map(|p| (n.to_string(), (p - node).normalize())))
                .collect();
            let mut best = (f32::MAX, Vec3::Y);
            for i in 0..3600 {
                let theta = (i / 60) as f32 * std::f32::consts::PI / 60.0;
                let phi = (i % 60) as f32 * std::f32::consts::TAU / 60.0;
                let n = Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin());
                let worst = directions.iter().map(|(_, d)| d.dot(n).abs().asin()).fold(0.0, f32::max);
                if worst < best.0 {
                    best = (worst, n);
                }
            }
            let out: Vec<String> = directions
                .iter()
                .map(|(name, d)| format!("{name} {:.0}°", d.dot(best.1).asin().to_degrees()))
                .collect();
            eprintln!("one plate at Z2: best plane leaves the cables out of it by {}", out.join(", "));
        }
        for name in names.split(',') {
            let Some(joint) = fabric.joints.keys().find(|&j| fabric.joint_label(j) == name) else {
                eprintln!("{name}: no such joint");
                continue;
            };
            eprintln!("{name} at {:?}:", fabric.joints[joint].location);
            for (key, i) in fabric.intervals.iter().filter(|(_, i)| i.touches(joint)) {
                let stiffness = if i.role == Role::Pushing { hardware.strut_stiffness } else { hardware.cable_stiffness };
                let force = (stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0;
                let hanger = crate::connector::system::is_hanger(fabric, key);
                eprintln!(
                    "  to {:>6}: {:?}{} {:.3} m (rest {:.3}), {force:.1} kN",
                    fabric.joint_label(i.other_joint(joint)),
                    i.role,
                    if hanger { " (hanger)" } else { "" },
                    i.length(&fabric.joints).f32(),
                    i.ideal().f32()
                );
            }
        }
    }

    /// Kay Ten's most loaded members, with and without its bridles:
    /// each one's force in zero gravity as pretensing ends, and standing.
    ///
    /// ```
    /// cargo test --release --lib kay_ten_load_path -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_load_path() {
        for (pretenst, bridled) in [(0.2, false), (0.2, true)] {
            eprintln!("== pretense {pretenst}%, bridles {bridled}");
            load_path(pretenst, bridled);
        }
    }

    fn load_path(pretenst: f32, bridled: bool) {
        use crate::build::dsl::fabric_plan_executor::ExecutorStage;
        let mut plan = kay_ten_plan(KayTenShape::default());
        if !bridled {
            plan.pretense_phase.bridles.clear();
        }
        plan.pretense_phase.pretenst = crate::units::Percent(pretenst);
        let mut executor = FabricPlanExecutor::new(plan);
        while *executor.stage() != ExecutorStage::Falling {
            let _ = executor.iterate();
        }
        let force = |fabric: &Fabric, i: &crate::fabric::interval::Interval| {
            let hardware = fabric.dimensions.hardware.expect("real parts");
            let stiffness = if i.role == Role::Pushing { hardware.strut_stiffness } else { hardware.cable_stiffness };
            (stiffness * (i.stiffness.as_factor() * i.strain)).f32() / 1000.0
        };
        let floating: Vec<(crate::fabric::IntervalKey, f32)> =
            executor.fabric.intervals.iter().map(|(key, i)| (key, force(&executor.fabric, i))).collect();
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let fabric = &executor.fabric;
        let name = |i: &crate::fabric::interval::Interval| format!("{}:{}", fabric.joint_label(i.alpha_key), fabric.joint_label(i.omega_key));
        let mut rows: Vec<(f32, f32, String, f32, Role)> = floating
            .iter()
            .filter_map(|&(key, before)| {
                let i = fabric.intervals.get(key)?;
                Some((force(fabric, i), before, name(i), i.length(&fabric.joints).f32(), i.role))
            })
            .collect();
        for (title, pushing) in [("cables", false), ("struts", true)] {
            let mut members: Vec<_> = rows.iter().filter(|r| (r.4 == Role::Pushing) == pushing).cloned().collect();
            members.sort_by(|a, b| if pushing { a.0.total_cmp(&b.0) } else { b.0.total_cmp(&a.0) });
            eprintln!("most loaded {title} (standing kN, zero-G kN, length m, role):");
            for (standing, floating, name, length, role) in members.iter().take(6) {
                eprintln!("  {name:>10} {standing:6.1} {floating:6.1} {length:5.2} {role:?}");
            }
        }
        rows.retain(|r| r.4 != Role::Pushing);
        let total: usize = rows.len();
        let over = rows.iter().filter(|r| r.0 > 5.1).count();
        let slack = rows.iter().filter(|r| r.0 <= 0.0).count();
        let (low, high) = fabric.altitude_range();
        eprintln!("{over} of {total} cables over 5.1 kN standing, {slack} slack, height {:.2} m", high - low);
    }

    /// Where the Kay Ten's cable forces come from: the plan step by step.
    ///
    /// ```
    /// cargo test --release --lib kay_ten_cable_forces -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_cable_forces() {
        let full = kay_ten_plan(KayTenShape::default());
        let mut no_bridles = full.clone();
        no_bridles.pretense_phase.bridles.clear();
        let mut bare = no_bridles.clone();
        bare.pretense_phase.hinge_pairs.clear();
        for bridled in [0, 2, 4] {
            for percent in [0.15, 0.2, 0.3] {
                let mut plan = full.clone();
                plan.pretense_phase.pretenst = crate::units::Percent(percent);
                plan.pretense_phase.bridles.truncate(bridled);
                let (length, width, height, culprits, standing, peak) = stance(plan.clone());
                let mut executor = FabricPlanExecutor::new(plan);
                while !executor.is_complete() {
                    let _ = executor.iterate();
                }
                let slack = executor.fabric.intervals.values().filter(|i| i.role.is_pull_like() && i.strain <= 0.0).count();
                eprintln!(
                    "bridles {bridled}, pretense {percent}%: peak {peak:.1} kN, {slack} slack, {culprits} colliding ends, \
                     {length:.2} x {width:.2} m, height {height:.2} m, standing {standing}"
                );
            }
        }
        let mut settle_30 = full.clone();
        settle_30.settle_phase.as_mut().expect("settles").seconds = Seconds(30.0);
        let mut hh_settle_10 = crate::build::dsl::fabric_library::get_fabric_plan(FabricName::HeadlessHug);
        hh_settle_10.settle_phase.as_mut().expect("settles").seconds = Seconds(10.0);
        let mut hh_no_guys = crate::build::dsl::fabric_library::get_fabric_plan(FabricName::HeadlessHug);
        hh_no_guys.settle_phase.as_mut().expect("settles").guy_lines.clear();
        for (name, plan) in [
            ("full plan, settle 30 s", settle_30),
            ("Headless Hug, settle 10 s", hh_settle_10),
            ("Headless Hug, no guy lines", hh_no_guys),
            ("full plan", full),
            ("no bridles", no_bridles),
            ("no hinges or bridles", bare),
            ("Tork, 2-twist legs, scale 0.75, 70%", torque_like(Tork, 2, Some(Pct(70.0)))),
            ("Tork, 2-twist legs, scale 0.75, no spacers", torque_like(Tork, 2, None)),
            ("Torque, 2-twist legs, scale 0.75, no spacers", torque(2, None)),
            ("Omni, 2-twist legs, scale 0.75, no spacers", torque_like(OmniSymmetrical, 2, None)),
            ("Torque, 2-twist legs, scale 0.75, 50%", torque(2, Some(Pct(50.0)))),
            ("Headless Hug", crate::build::dsl::fabric_library::get_fabric_plan(FabricName::HeadlessHug)),
        ] {
            let stages: Vec<String> = forces_by_stage(plan)
                .into_iter()
                .map(|(stage, median, peak)| format!("{stage} {median:.1}/{peak:.1}"))
                .collect();
            eprintln!("{name}: median/peak cable kN at the end of {}", stages.join(", "));
        }
    }

    /// ```
    /// KAY_TEN_FIGURES=<folder> cargo test --release --lib kay_ten_candidates -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn kay_ten_candidates() {
        let folder = std::env::var("KAY_TEN_FIGURES").expect("KAY_TEN_FIGURES");
        let folder = Path::new(&folder);
        for brick in [TorqueSymmetrical, Tork] {
            let baked = crate::build::dsl::brick_library::baked_bricks::get_baked_brick(brick);
            let mut lengths: Vec<String> = baked
                .intervals
                .iter()
                .map(|i| {
                    let length = baked.joints[i.alpha_index].location.distance(baked.joints[i.omega_index].location);
                    format!("{} {length:.2} ({:+.1}%)", i.material_name, i.strain * 100.0)
                })
                .collect();
            lengths.sort();
            eprintln!("{brick:?} scale {:.3}: {lengths:?}", baked.scale);
        }
        report("open-claw", crate::build::dsl::fabric_library::get_fabric_plan(FabricName::OpenClaw), folder);
        report("kay_ten", kay_ten_plan(KayTenShape::default()), folder);
        report("tork", torque_like(Tork, 2, Some(Pct(50.0))), folder);
        report("tork-legs-free", torque_like(Tork, 2, None), folder);
        report("tork-70", torque_like(Tork, 2, Some(Pct(70.0))), folder);
        report("tork-45-145", tork_spaced(Pct(45.0), Pct(145.0), Pct(95.0)), folder);
        report("tork-35-165", tork_spaced(Pct(35.0), Pct(165.0), Pct(95.0)), folder);
        report("legs-free", torque(2, None), folder);
        report("feet-70", torque(2, Some(Pct(70.0))), folder);
        report("legs-3", torque(3, Some(Pct(50.0))), folder);
    }
}
