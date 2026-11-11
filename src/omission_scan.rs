//! Which cables of a tension triangle can go? Omitting Z10–Z4 (and its
//! mirror) from Headless Hug broke a triangle of cables and cleared
//! connector collisions. This scan tries that for every edge of every
//! all-cable triangle: omit the edge and its mirror image, run the whole
//! plan, and compare collisions, short cables and settled height.
//!
//! Z10–Z4 was the long side of a nearly flat triangle (apex angle 168°): a
//! cable running almost alongside two others that already span the same
//! joints. So each edge is listed with the widest angle it faces across any
//! of its triangles — near 180° means it's redundant in just that way.
//!
//! The bridle search goes after the collisions that remain: two forks that
//! collide on one stack are taken off through a bridle node instead (see
//! `Fabric::bridle`), mirror pair by mirror pair.
//!
//! The shape search goes the other way: change the design itself — its
//! shrinks, prisms, spacers and omissions — until it builds with standard
//! rings, needing as few special parts as possible.

#[cfg(test)]
mod tests {
    use crate::build::dsl::fabric_dsl::Pct;
    use crate::build::dsl::joint_names::{Bridle, Link};
    use crate::build::dsl::fabric_library::{self, headless_hug_plan, FabricName, HeadlessHugShape};
    use crate::connector::parts::{placements, Part};
    use crate::connector::system::is_hanger;
    use crate::fabric::interval::Role;
    use crate::fabric::joint::JointLabel;
    use crate::build::dsl::fabric_plan_executor::FabricPlanExecutor;
    use crate::fabric::{Fabric, IntervalKey, JointKey};
    use crate::units::{Meters, Percent, Seconds, Unit};
    use crate::Age;
    use glam::Vec3;
    use std::collections::HashSet;

    /// A cable to omit, with its images: `(link, length, widest apex angle)`.
    type Candidate = (Link, f32, f32);

    struct Outcome {
        intervals: usize,
        culprits: usize,
        short_cables: usize,
        /// Cables as built: a bridle's hanger and a fork rod aren't
        /// cables.
        cables: usize,
        height: f32,
        finite: bool,
    }

    impl Outcome {
        /// Everything that stands between this fabric and a build.
        fn problems(&self) -> usize {
            self.culprits + self.short_cables
        }
    }

    fn run(name: FabricName, omit: &[Link]) -> (Fabric, Outcome) {
        run_plan(name, omit, &[])
    }

    fn run_plan(name: FabricName, omit: &[Link], bridles: &[Bridle]) -> (Fabric, Outcome) {
        let mut plan = fabric_library::get_fabric_plan(name);
        plan.pretense_phase.omit_pairs.extend(omit.iter().copied());
        plan.pretense_phase.bridles.extend(bridles.iter().copied());
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let fabric = executor.fabric;
        let connector = fabric.connector.as_ref().expect("fabric has connectors");
        let (low, high) = fabric.altitude_range();
        let outcome = Outcome {
            intervals: fabric.intervals.len(),
            culprits: connector.culprits.len(),
            short_cables: connector.short_cables.len(),
            cables: fabric
                .intervals
                .iter()
                .filter(|(key, i)| {
                    i.role.is_pull_like() && !is_hanger(&fabric, *key) && connector.link_length(*key).is_none()
                })
                .count(),
            height: high - low,
            finite: fabric.joints.values().all(|joint| joint.location.is_finite()),
        };
        (fabric, outcome)
    }

    /// The joint at the position reflected across x = 0, if there is one.
    fn mirror_of(fabric: &Fabric, joint: JointKey) -> Option<JointKey> {
        let symmetry = fabric.label_symmetry.as_ref()?;
        let label = fabric.joints[joint].label?;
        fabric.joint_key(symmetry.map_label(label))
    }

    /// Every cable that is an edge of a triangle whose three edges are all
    /// cables, one per set of images, with its length and the widest apex
    /// angle it faces, in degrees.
    fn triangle_edges(fabric: &Fabric) -> Vec<Candidate> {
        let cables: HashSet<(JointKey, JointKey)> = fabric
            .intervals
            .values()
            .filter(|i| i.role.is_pull_like())
            .flat_map(|i| [(i.alpha_key, i.omega_key), (i.omega_key, i.alpha_key)])
            .collect();
        let mut seen: HashSet<(JointKey, JointKey)> = HashSet::new();
        let mut edges = Vec::new();
        for interval in fabric.intervals.values().filter(|i| i.role.is_pull_like()) {
            let (a, b) = (interval.alpha_key, interval.omega_key);
            let apex = fabric
                .joints
                .keys()
                .filter(|&c| cables.contains(&(a, c)) && cables.contains(&(b, c)))
                .map(|c| {
                    let at = |k: JointKey| fabric.joints[k].location - fabric.joints[c].location;
                    at(a).angle_between(at(b)).to_degrees()
                })
                .max_by(f32::total_cmp);
            let Some(apex) = apex else { continue };
            if seen.contains(&(a, b)) {
                continue;
            }
            let symmetry = fabric.label_symmetry.as_ref().expect("a symmetric fabric");
            let (Some(alpha), Some(omega)) = (fabric.joints[a].label, fabric.joints[b].label) else { continue };
            let Some(link) = Link::between(alpha, omega, symmetry) else { continue };
            for (x, y) in link.pairs(Some(symmetry)).unwrap_or_default() {
                if let (Some(x), Some(y)) = (fabric.joint_key(x), fabric.joint_key(y)) {
                    seen.insert((x, y));
                    seen.insert((y, x));
                }
            }
            edges.push((link, interval.length(&fabric.joints).f32(), apex));
        }
        edges
    }

    /// `f` over every item, spread across all cores.
    fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let chunk = items.len().div_ceil(threads).max(1);
        let f = &f;
        std::thread::scope(|scope| {
            let handles: Vec<_> = items
                .chunks(chunk)
                .map(|slice| scope.spawn(move || slice.iter().map(f).collect::<Vec<_>>()))
                .collect();
            handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
        })
    }

    /// Runs the plan once per candidate, in parallel, each with the
    /// candidate's cables omitted on top of `base`.
    fn evaluate(
        name: FabricName,
        base: &[Link],
        candidates: &[Candidate],
    ) -> Vec<Outcome> {
        parallel(candidates, |(link, ..)| {
            let all: Vec<Link> = base.iter().copied().chain([*link]).collect();
            run(name, &all).1
        })
    }

    /// Prints every candidate, flattest triangle first. Candidates whose
    /// labels don't exist yet when `omit` runs (renamed by a later hinge) are
    /// reported as having no effect.
    #[test]
    #[ignore]
    fn scan_headless_hug_triangle_omissions() {
        let name = FabricName::HeadlessHug;
        let (baseline_fabric, baseline) = run(name, &[]);
        let candidates = triangle_edges(&baseline_fabric);
        eprintln!(
            "baseline: {} culprit ends, {} short cables, height {:.2} m; {} candidates",
            baseline.culprits,
            baseline.short_cables,
            baseline.height,
            candidates.len()
        );

        let outcomes = evaluate(name, &[], &candidates);
        let mut results: Vec<(String, f32, f32, usize, Outcome)> = candidates
            .into_iter()
            .zip(outcomes)
            .map(|((link, length, apex), outcome)| {
                let omitted = link.pairs(baseline_fabric.label_symmetry.as_ref()).map_or(0, |pairs| pairs.len());
                (link.to_string(), length, apex, omitted, outcome)
            })
            .collect();
        results.sort_by(|a, b| b.2.total_cmp(&a.2));

        eprintln!(" apex  culprits  Δ    short  height  cable (length)");
        for (label, length, apex, omitted, outcome) in &results {
            let effect = if baseline.intervals - outcome.intervals < *omitted {
                "  (not all found when omit runs)"
            } else if !outcome.finite {
                "  (DIVERGED)"
            } else {
                ""
            };
            eprintln!(
                "{:>4.0}° {:>8} {:>+4} {:>6} {:>6.2}  {label} ({:.3} m){effect}",
                apex,
                outcome.culprits,
                outcome.culprits as i64 - baseline.culprits as i64,
                outcome.short_cables,
                outcome.height,
                length,
            );
        }
    }

    /// Greedily omits flat-triangle cables (apex at least `FLAT_APEX`):
    /// each round reruns the plan with every remaining flat candidate on top
    /// of those already chosen, keeps the one clearing the most collisions
    /// without moving the settled height by more than `HEIGHT_TOLERANCE`, and
    /// stops when none helps. Prints the `omit` entries to paste.
    #[test]
    #[ignore]
    fn greedy_headless_hug_flat_omissions() {
        const FLAT_APEX: f32 = 120.0;
        const HEIGHT_TOLERANCE: f32 = 0.05;
        let name = FabricName::HeadlessHug;
        let mut chosen: Vec<Link> = Vec::new();
        loop {
            let (fabric, current) = run(name, &chosen);
            eprintln!(
                "{} culprit ends, {} short cables, height {:.2} m",
                current.culprits, current.short_cables, current.height
            );
            let flat: Vec<Candidate> = triangle_edges(&fabric)
                .into_iter()
                .filter(|(.., apex)| *apex >= FLAT_APEX)
                .collect();
            let outcomes = evaluate(name, &chosen, &flat);
            let best = flat
                .iter()
                .zip(&outcomes)
                .filter(|(_, o)| {
                    o.finite
                        && o.culprits < current.culprits
                        && (o.height - current.height).abs() < HEIGHT_TOLERANCE
                })
                .min_by_key(|(_, o)| o.culprits);
            let Some(((link, length, apex), _)) = best else {
                break;
            };
            eprintln!("  omit {link} ({length:.3} m, apex {apex:.0}°)");
            chosen.push(*link);
        }
        for link in &chosen {
            eprintln!("{link},");
        }
    }

    /// Bridles for the colliding fork pairs that share a joint, and for
    /// each too-short cable together with each other cable at either of its
    /// ends, so the short one is replaced by cables to a node plate set out
    /// at a buildable length. A cable may run between two nodes. Only symmetric candidates
    /// count: each stands for itself and its mirror image, and on the mirror
    /// plane a candidate must be its own image. Pairs involving a bridle's
    /// link are left out.
    fn bridle_candidates(fabric: &Fabric) -> Vec<Bridle> {
        let connector = fabric.connector.as_ref().expect("fabric has connectors");
        let links: HashSet<_> = fabric.bridle_nodes.values().copied().collect();
        let mut pairs: Vec<(IntervalKey, IntervalKey, JointKey)> = connector
            .collisions
            .iter()
            .filter(|[(_, joint), (_, other)]| joint == other)
            .map(|[(a, joint), (b, _)]| (*a, *b, *joint))
            .collect();
        for &short in &connector.short_cables {
            let interval = &fabric.intervals[short];
            for end in [interval.alpha_key, interval.omega_key] {
                for (other, i) in fabric.intervals.iter() {
                    if other != short && i.role.is_pull_like() && i.touches(end) {
                        pairs.push((short, other, end));
                    }
                }
            }
        }
        // Bridles as (joint, a, b). On the mirror plane a pair's image can
        // share a cable with it, so there the symmetric choice is a cable
        // together with its own mirror twin.
        let cable = |j: JointKey, far: JointKey| {
            fabric
                .joining((j, far))
                .filter(|key| fabric.intervals[*key].role.is_pull_like() && !links.contains(key))
        };
        let mut triples: Vec<(JointKey, JointKey, JointKey)> = Vec::new();
        for (cable_a, cable_b, joint) in &pairs {
            if links.contains(cable_a) || links.contains(cable_b) || fabric.bridle_nodes.contains_key(joint) {
                continue;
            }
            let a = fabric.intervals[*cable_a].other_joint(*joint);
            let b = fabric.intervals[*cable_b].other_joint(*joint);
            triples.push((*joint, a, b));
            if mirror_of(fabric, *joint) == Some(*joint) {
                for far in [a, b] {
                    if let Some(image) = mirror_of(fabric, far).filter(|m| *m != far) {
                        if cable(*joint, image).is_some() {
                            triples.push((*joint, far, image));
                        }
                    }
                }
            }
        }
        let symmetry = fabric.label_symmetry.as_ref().expect("a symmetric fabric");
        let mut candidates: Vec<Bridle> = Vec::new();
        for (joint, a, b) in triples {
            let label = |key: JointKey| fabric.joints[key].label;
            let (Some(joint), Some(a), Some(b)) = (label(joint), label(a), label(b)) else { continue };
            let Some(bridle) = Bridle::between(joint, a, b, symmetry) else { continue };
            if !candidates.iter().any(|known| known.to_string() == bridle.to_string()) {
                candidates.push(bridle);
            }
        }
        candidates
    }

    /// Greedily bridles: each round reruns the plan with every candidate on
    /// top of those already chosen, keeps the one leaving the fewest problems
    /// (collisions, short cables) without moving the
    /// settled height by more than `HEIGHT_TOLERANCE`, and stops when none
    /// helps. Prints the `bridle` entries to paste.
    #[test]
    #[ignore]
    fn greedy_headless_hug_bridles() {
        const HEIGHT_TOLERANCE: f32 = 0.05;
        let name = FabricName::HeadlessHug;
        let mut chosen: Vec<Bridle> = Vec::new();
        loop {
            let (fabric, current) = run_plan(name, &[], &chosen);
            eprintln!(
                "{} problems: {} culprit ends, {} short cables; {} cables, height {:.2} m, {} bridles",
                current.problems(),
                current.culprits,
                current.short_cables,
                current.cables,
                current.height,
                chosen.len()
            );
            let candidates = bridle_candidates(&fabric);
            let outcomes = parallel(&candidates, |bridle| {
                let all: Vec<Bridle> = chosen.iter().copied().chain([*bridle]).collect();
                run_plan(name, &[], &all).1
            });
            let best = candidates
                .iter()
                .zip(&outcomes)
                .filter(|(_, o)| {
                    o.finite
                        && o.problems() < current.problems()
                        && (o.height - current.height).abs() < HEIGHT_TOLERANCE
                })
                .min_by_key(|(_, o)| o.problems());
            let Some((bridle, _)) = best else {
                break;
            };
            eprintln!("  bridle {bridle}");
            chosen.push(*bridle);
        }
        for bridle in &chosen {
            eprintln!("{bridle},");
        }
    }

    /// Headless Hug must stay buildable: no colliding connectors, no cable
    /// too short for its terminals. Collision counts are fragile — small changes to the plan
    /// can tip borderline cases — so any change to it must keep this green.
    #[test]
    fn headless_hug_is_buildable() {
        let (_, outcome) = run_plan(FabricName::HeadlessHug, &[], &[]);
        assert!(outcome.finite);
        assert_eq!(
            (outcome.culprits, outcome.short_cables),
            (0, 0),
            "Headless Hug has build problems (culprit ends, short cables): \
             inspect in the app (C key) and rerun greedy_headless_hug_bridles"
        );
    }

    /// A Headless Hug design: its shape, and which of its plan's omissions
    /// (each with its mirror image) are kept.
    #[derive(Clone)]
    struct Design {
        shape: HeadlessHugShape,
        omits: Vec<bool>,
    }

    /// How a design builds with standard hardware: no bridles.
    struct Verdict {
        culprits: usize,
        short_cables: usize,
        slack: usize,
        /// Custom discs and spacers.
        special_parts: usize,
        height: f32,
        valid: bool,
    }

    impl Verdict {
        fn score(&self) -> (usize, usize) {
            (self.culprits + self.short_cables + self.slack, self.special_parts)
        }

        fn describe(&self) -> String {
            format!(
                "{} culprit ends, {} short, {} slack, {} special parts, height {:.2} m",
                self.culprits, self.short_cables, self.slack, self.special_parts, self.height
            )
        }
    }

    /// Every Z joint's label with its place in the build tree.
    fn axial_paths(fabric: &Fabric) -> Vec<(String, String)> {
        let mut paths: Vec<(String, String)> = fabric
            .joints
            .iter()
            .filter(|(_, joint)| matches!(joint.label, Some(JointLabel::Axial { .. })))
            .map(|(key, joint)| (fabric.joint_label(key), joint.path.to_string()))
            .collect();
        paths.sort();
        paths
    }

    /// Build `design` without bridles. Valid only if it stays finite, its
    /// Z joints keep their names (plans refer to them by name), and its
    /// height stays within 15% of `reference_height`.
    fn judge(design: &Design, reference: &[(String, String)], reference_height: f32) -> Verdict {
        let mut plan = headless_hug_plan(design.shape);
        plan.pretense_phase.bridles.clear();
        let omits = std::mem::take(&mut plan.pretense_phase.omit_pairs);
        plan.pretense_phase.omit_pairs =
            omits.into_iter().enumerate().filter(|(i, _)| design.omits[*i]).map(|(_, link)| link).collect();
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor.fabric.update_all_attachment_connections();
        let fabric = &executor.fabric;
        let connector = fabric.connector.as_ref().expect("fabric has connectors");
        let (low, high) = fabric.altitude_range();
        let height = high - low;
        let placed = placements(fabric);
        let count = |matching: fn(Part) -> bool| placed.iter().filter(|p| matching(p.part)).count();
        // Washers beyond the one under each ring or disc are spacers.
        let spacers = count(|part| part == Part::Washer)
            .saturating_sub(count(|part| matches!(part, Part::Ring | Part::CustomDisc(_))));
        let special_parts =
            spacers + count(|part| matches!(part, Part::CustomDisc(_) | Part::NodePlate(_) | Part::RodFork));
        Verdict {
            culprits: connector.culprits.len(),
            short_cables: connector.short_cables.len(),
            slack: fabric
                .intervals
                .values()
                .filter(|i| i.role.is_pull_like() && i.role != Role::GuyLine && i.strain <= 0.0)
                .count(),
            special_parts,
            height,
            valid: fabric.joints.values().all(|j| j.location.is_finite())
                && (reference.is_empty() || axial_paths(fabric) == reference)
                && (reference_height == 0.0 || (height / reference_height - 1.0).abs() < 0.15),
        }
    }

    fn reference(design: &Design) -> (Vec<(String, String)>, f32) {
        let mut plan = headless_hug_plan(design.shape);
        plan.pretense_phase.bridles.clear();
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        let (low, high) = executor.fabric.altitude_range();
        (axial_paths(&executor.fabric), high - low)
    }

    /// The shape numbers by index, for stepping one at a time.
    fn shape_value(shape: &mut HeadlessHugShape, index: usize) -> &mut Pct {
        match index {
            0 => &mut shape.leg_shrink,
            1 => &mut shape.leg_prism,
            2 => &mut shape.shoulder_shrink,
            3 => &mut shape.arm_shrink,
            4 => &mut shape.arm_prism,
            5 => &mut shape.same_side,
            6 => &mut shape.crossed,
            7 => &mut shape.feet,
            8 => &mut shape.hands,
            _ => &mut shape.chest,
        }
    }

    /// Searches Headless Hug's design for one that builds with standard
    /// rings: coordinate steps on every shape number and toggles of each
    /// mirror pair of omissions, keeping a step when it lowers (problems,
    /// special parts), halving the steps when none does.
    #[test]
    #[ignore]
    fn search_headless_hug_shape() {
        const ROUNDS: usize = 30;
        let pairs = headless_hug_plan(HeadlessHugShape::default()).pretense_phase.omit_pairs.len();
        let mut best = Design { shape: HeadlessHugShape::default(), omits: vec![true; pairs] };
        let (paths, height) = reference(&best);
        let mut verdict = judge(&best, &paths, height);
        eprintln!("start: {}", verdict.describe());
        let mut steps = [3.0, 50.0, 5.0, 3.0, 50.0, 4.0, 4.0, 8.0, 8.0, 8.0];
        for round in 0..ROUNDS {
            let mut candidates: Vec<(String, Design)> = Vec::new();
            for index in 0..steps.len() {
                for sign in [-1.0, 1.0] {
                    let mut design = best.clone();
                    let value = shape_value(&mut design.shape, index);
                    *value = Pct((value.0 + sign * steps[index]).max(0.0));
                    candidates.push((format!("value {index} {:+}", sign * steps[index]), design));
                }
            }
            for pair in 0..pairs {
                let mut design = best.clone();
                design.omits[pair] = !design.omits[pair];
                candidates.push((format!("omission pair {pair} {}", if design.omits[pair] { "on" } else { "off" }), design));
            }
            let verdicts = parallel(&candidates, |(_, design)| judge(design, &paths, height));
            let winner = candidates
                .iter()
                .zip(&verdicts)
                .filter(|(_, v)| v.valid && v.score() < verdict.score())
                .min_by_key(|(_, v)| v.score());
            match winner {
                Some(((change, design), v)) => {
                    eprintln!("round {round}: {change} -> {}", v.describe());
                    best = design.clone();
                    verdict = judge(&best, &paths, height);
                }
                None => {
                    steps.iter_mut().for_each(|step| *step /= 2.0);
                    eprintln!("round {round}: no improvement, steps halved");
                }
            }
        }
        eprintln!("best: {}", verdict.describe());
        eprintln!("shape: {:?}", best.shape);
        eprintln!("omissions kept: {:?}", best.omits);
    }

    /// Headless Hug standing, its guy lines' lengths set `front` and `rear`:
    /// the pairs anchored ahead of it and behind.
    fn standing(front: Percent, rear: Percent) -> FabricPlanExecutor {
        let mut plan = headless_hug_plan(HeadlessHugShape::default());
        let settle = plan.settle_phase.as_mut().expect("Headless Hug settles");
        for guy in settle.guy_lines.iter_mut() {
            guy.length = if guy.ground_offset.1.f32() > 0.0 { front } else { rear };
        }
        let mut executor = FabricPlanExecutor::new(plan);
        while !executor.is_complete() {
            let _ = executor.iterate();
        }
        executor
    }

    fn location(fabric: &Fabric, name: &str) -> Vec3 {
        let joint = fabric.joints.keys().find(|&joint| fabric.joint_label(joint) == name).expect("joint by name");
        fabric.joints[joint].location
    }

    /// How far the centre of gravity stands ahead of the line between the
    /// feet, positive toward the front anchors.
    fn lean(fabric: &Fabric) -> Meters {
        let (left, right) = (location(fabric, "L54"), location(fabric, "R54"));
        let mass: f32 = fabric.joints.values().map(|joint| joint.accumulated_mass.f32()).sum();
        let gravity = fabric.joints.values().map(|joint| joint.location * joint.accumulated_mass.f32()).sum::<Vec3>() / mass;
        let along = Vec3::new(right.x - left.x, 0.0, right.z - left.z).normalize();
        let ahead = Vec3::new(-along.z, 0.0, along.x);
        let ahead = if ahead.z < 0.0 { -ahead } else { ahead };
        Meters((gravity - left).dot(ahead))
    }

    /// How far any joint moves in `seconds` more of settling.
    fn still_moving(executor: &mut FabricPlanExecutor, seconds: Seconds) -> Meters {
        let before: Vec<Vec3> = executor.fabric.joints.values().map(|joint| joint.location).collect();
        let physics = executor.physics.clone();
        for _ in 0..(seconds / Age::iteration_duration()) as usize {
            executor.fabric.iterate(&physics);
        }
        let after = executor.fabric.joints.values().map(|joint| joint.location);
        Meters(before.iter().zip(after).map(|(a, b)| a.distance(b)).fold(0.0, f32::max))
    }

    fn guy_forces(fabric: &Fabric) -> Vec<f32> {
        let stiffness = fabric.dimensions.hardware.expect("real parts").cable_stiffness;
        fabric
            .intervals
            .values()
            .filter(|interval| interval.has_role(Role::GuyLine))
            .map(|interval| stiffness.to_kilonewtons() * interval.strain)
            .collect()
    }

    /// The plan's guy line lengths, `(front, rear)`.
    fn planned_guy_lines() -> (Percent, Percent) {
        let plan = headless_hug_plan(HeadlessHugShape::default());
        let guys = &plan.settle_phase.as_ref().expect("Headless Hug settles").guy_lines;
        let length = |front: bool| guys.iter().find(|guy| (guy.ground_offset.1.f32() > 0.0) == front).unwrap().length;
        (length(true), length(false))
    }

    /// Headless Hug stands upright, its centre of gravity over its feet with
    /// every guy line taut, and has come to rest by the end of its settle.
    #[test]
    fn headless_hug_stands_upright() {
        let (front, rear) = planned_guy_lines();
        let mut executor = standing(front, rear);
        let lean = lean(&executor.fabric);
        assert!(lean.f32().abs() < 0.02, "centre of gravity {:.0} mm off the feet", lean.to_mm());
        assert!(guy_forces(&executor.fabric).iter().all(|&force| force > 0.1), "a guy line hangs slack");
        let moved = still_moving(&mut executor, Seconds(10.0));
        assert!(moved.f32() < 0.01, "still moving {:.1} mm", moved.to_mm());
    }

    /// Guy line lengths around the plan's, front shortened as rear is
    /// lengthened: how far each leans the centre of gravity from the feet.
    /// Each 0.5 % moves it about 80 mm.
    ///
    /// ```
    /// cargo test --release --lib search_headless_hug_guy_lines -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore]
    fn search_headless_hug_guy_lines() {
        let (front, rear) = planned_guy_lines();
        for step in [-0.5, -0.25, -0.1, 0.0, 0.1, 0.25, 0.5] {
            let (front, rear) = (front.0 + step, rear.0 - step);
            let mut executor = standing(Percent(front), Percent(rear));
            let lean = lean(&executor.fabric);
            let forces: Vec<String> = guy_forces(&executor.fabric).iter().map(|f| format!("{f:.2}")).collect();
            let moved = still_moving(&mut executor, Seconds(10.0));
            eprintln!(
                "front {front:.2}% rear {rear:.2}%: lean {:+.0} mm, guys {} kN, moves {:.1} mm more",
                lean.to_mm(),
                forces.join(" "),
                moved.to_mm()
            );
        }
    }
}
