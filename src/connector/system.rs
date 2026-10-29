//! Runtime connector state for a fabric: hardware dimensions plus the
//! cable-to-slot assignments per push interval. Present on a `Fabric` only
//! when a physical build is intended; absent, connectors play no role.

use crate::connector::attachment::{
    calculate_interval_attachment_points, PullConnection, PullConnections, PullIntervalData,
    ATTACHMENT_POINTS,
};
use crate::connector::fork::Assembly;
use crate::connector::node_plate::NodePlate;
use crate::connector::parts::Part;
use crate::connector::{fork, ConnectorDimensions};
use crate::build::dsl::labelling::LabelSymmetry;
use crate::fabric::interval::Role;
use crate::fabric::joint::JointLabel;
use crate::fabric::{Fabric, IntervalEnd, IntervalKey, JointKey};
use crate::units::{Meters, Unit};
use glam::Vec3;
use slotmap::SecondaryMap;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ConnectorSystem {
    pub dimensions: ConnectorDimensions,
    /// Slot assignments per push interval, rebuilt on demand from current
    /// geometry (`update_all_connections`) — never during the physics tick.
    pub connections: SecondaryMap<IntervalKey, PullConnections>,
    /// Cable ends `(pull key, near joint)` whose assembled connectors would
    /// physically collide with another assembly — real construction issues.
    /// Marked once per `update_all_connections`, so the renderer never has
    /// to run the pairwise scan per frame.
    pub culprits: HashSet<(IntervalKey, JointKey)>,
    /// The colliding pairs behind `culprits`, each end as in `culprits`.
    pub collisions: Vec<[(IntervalKey, JointKey); 2]>,
    /// Cables whose pivot-to-pivot span can't hold a terminal at each end
    /// plus the minimum free wire — they can't be built as cables at all.
    /// Marked alongside `culprits`.
    pub short_cables: HashSet<IntervalKey>,
    /// Cables from a node plate too short to make as cables, built as swivel
    /// links instead: each with its pin-to-pin length when the hardware was
    /// committed (`update_all_connections`).
    links: HashMap<IntervalKey, f32>,
    /// Each bridle node's plate, built with the slot assignments:
    /// they aim at the pivots the assignments put the far ends at.
    plates: HashMap<JointKey, NodePlate>,
    /// Every placement's part as the parts list stood at the last rebuild,
    /// in placement order (see `parts::placements`): shaped parts keep
    /// these shapes until the next rebuild.
    pub frozen_parts: Vec<Part>,
}

impl ConnectorSystem {
    pub fn new(dimensions: ConnectorDimensions) -> Self {
        Self {
            dimensions,
            connections: SecondaryMap::new(),
            culprits: HashSet::new(),
            collisions: Vec::new(),
            short_cables: HashSet::new(),
            links: HashMap::new(),
            plates: HashMap::new(),
            frozen_parts: Vec::new(),
        }
    }

    pub fn connections(
        &self,
        push_key: IntervalKey,
        end: IntervalEnd,
    ) -> Option<&[Option<PullConnection>; ATTACHMENT_POINTS]> {
        self.connections.get(push_key).map(|conn| conn.connections(end))
    }

    pub fn connections_mut(&mut self, push_key: IntervalKey) -> Option<&mut PullConnections> {
        self.connections.get_mut(push_key)
    }

    /// Where a strut's two stacks start, with their outward axes:
    /// `[(alpha_end, alpha_axis), (omega_end, omega_axis)]`. A stack normally
    /// starts at the strut end and builds outward. At a hinge the strut ends
    /// at the hinge pin instead, so the stack is set back along the strut
    /// until the top of its highest ring sits `hinge_clearance` below the pin.
    pub fn stack_ends(&self, fabric: &Fabric, push_key: IntervalKey) -> [(Vec3, Vec3); 2] {
        let push = &fabric.intervals[push_key];
        let alpha = fabric.joints[push.alpha_key].location;
        let omega = fabric.joints[push.omega_key].location;
        let push_dir = (omega - alpha).normalize();
        [
            (self.stack_end(fabric, push_key, push.alpha_key, alpha, -push_dir), -push_dir),
            (self.stack_end(fabric, push_key, push.omega_key, omega, push_dir), push_dir),
        ]
    }

    fn stack_end(
        &self,
        fabric: &Fabric,
        push_key: IntervalKey,
        joint: JointKey,
        location: Vec3,
        outward: Vec3,
    ) -> Vec3 {
        if !is_hinge_pin(fabric, joint) {
            return location;
        }
        // Slots fill from 0, so the highest occupied one is the cable count less one.
        let cables = fabric
            .intervals
            .iter()
            .filter(|(key, interval)| {
                interval.role.is_pull_like()
                    && interval.touches(joint)
                    && stack_carries(fabric, push_key, joint, *key)
            })
            .count();
        let Some(highest) = cables.checked_sub(1) else {
            return location;
        };
        let top = self.dimensions.ring_center_offset(highest).f32()
            + self.dimensions.ring_thickness.f32() / 2.0
            + self.dimensions.hinge_clearance.f32();
        location - outward * top
    }

    pub fn ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize) -> Vec3 {
        self.dimensions.ring_center(push_end, push_axis, slot)
    }

    /// `(pivot_pos, elevation_deg)` — see `ConnectorDimensions::pivot_geometry`.
    pub fn pivot_geometry(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        slot: usize,
        pull_other_end: Vec3,
    ) -> (Vec3, f32) {
        self.dimensions
            .pivot_geometry(push_end, push_axis, slot, pull_other_end)
    }

    /// Where one assigned cable's ring sits: its height, lifted by any
    /// extra washers below it.
    pub fn connection_ring_center(&self, push_end: Vec3, push_axis: Vec3, connection: &PullConnection) -> Vec3 {
        self.lifted_ring_center(push_end, push_axis, connection.attachment_index, connection.lift)
    }

    fn lifted_ring_center(&self, push_end: Vec3, push_axis: Vec3, slot: usize, lift: u8) -> Vec3 {
        self.ring_center(push_end, push_axis, slot)
            + push_axis * (self.dimensions.washer_thickness.f32() * lift as f32)
    }

    /// `pivot_geometry` for one assigned cable: its ring's place, and its
    /// boss's reach when it is on a custom disc.
    pub fn connection_pivot(
        &self,
        push_end: Vec3,
        push_axis: Vec3,
        connection: &PullConnection,
        pull_other_end: Vec3,
    ) -> (Vec3, f32) {
        let reach = connection.reach.unwrap_or(self.dimensions.pivot_radius);
        let center = self.connection_ring_center(push_end, push_axis, connection);
        ConnectorDimensions::pivot_from_center(center, push_axis, pull_other_end, reach)
    }

    /// Rebuild slot assignments for every push interval from current geometry.
    pub fn update_all_connections(&mut self, fabric: &Fabric) {
        self.connections.clear();
        self.plates.clear();
        self.links.clear();
        self.frozen_parts.clear();
        if fabric.joints.is_empty() {
            return;
        }
        let push_keys: Vec<IntervalKey> = fabric
            .intervals
            .iter()
            .filter_map(|(key, interval)| interval.has_role(Role::Pushing).then_some(key))
            .collect();
        for push_key in push_keys {
            self.update_push_connections(fabric, push_key);
        }
        // The symmetry passes aim at plate holes; the plates then follow
        // where those passes put the far ends.
        self.build_nodes(fabric);
        if let Some(LabelSymmetry::Mirror { .. }) = &fabric.label_symmetry {
            self.mirror_assignments(fabric);
            self.plane_assignments(fabric);
            self.build_nodes(fabric);
        }
        self.mark(fabric, true);
    }

    /// Rebuild `culprits` and `short_cables`: the O(n²) capsule scan over all assembled cable
    /// ends, run once per assignment rebuild rather than per frame. Public
    /// so symmetry enforcement can re-mark after it rewrites assignments.
    pub fn mark_culprits(&mut self, fabric: &Fabric) {
        self.mark(fabric, false);
    }

    /// As `mark_culprits`, first `choosing` which plate cables are links.
    fn mark(&mut self, fabric: &Fabric, choosing: bool) {
        self.culprits.clear();
        self.collisions.clear();
        self.short_cables.clear();

        struct Entry {
            connection: PullConnection,
            pull: IntervalKey,
            near: JointKey,
            far: JointKey,
            end_pos: Vec3,
            axis: Vec3,
        }
        let mut entries: Vec<Entry> = Vec::new();

        for (push_key, push) in fabric.intervals.iter() {
            if !push.has_role(Role::Pushing) {
                continue;
            }
            let [(alpha_end, alpha_axis), (omega_end, omega_axis)] = self.stack_ends(fabric, push_key);

            for (end, end_pos, axis, near) in [
                (IntervalEnd::Alpha, alpha_end, alpha_axis, push.alpha_key),
                (IntervalEnd::Omega, omega_end, omega_axis, push.omega_key),
            ] {
                let Some(conns) = self.connections(push_key, end) else {
                    continue;
                };
                for conn in conns.iter().flatten() {
                    let Some(pull) = fabric.intervals.get(conn.pull_interval_key) else {
                        continue;
                    };
                    let far = if pull.alpha_key == near {
                        pull.omega_key
                    } else {
                        pull.alpha_key
                    };
                    entries.push(Entry {
                        connection: *conn,
                        pull: conn.pull_interval_key,
                        near,
                        far,
                        end_pos,
                        axis,
                    });
                }
            }
        }

        // First pass: pivots aimed at far joints.
        let mut first_pivot: HashMap<(IntervalKey, JointKey), Vec3> = HashMap::new();
        for e in &entries {
            let (pivot, _) =
                self.connection_pivot(e.end_pos, e.axis, &e.connection, fabric.joints[e.far].location);
            first_pivot.insert((e.pull, e.near), pivot);
        }

        // Second pass: final assembly frames aimed at the far end's pivot —
        // the same aiming the renderers use. A cable to a bridle node aims at
        // its hole in the node plate.
        let plates = std::mem::take(&mut self.plates);
        let node_aim = |cable: IntervalKey, node: JointKey| plates.get(&node).and_then(|plate| plate.hole(cable));
        let ring_radius = fabric.dimensions.push_radius.f32();
        let mut frames: Vec<((IntervalKey, JointKey), Vec3, Assembly)> = entries
            .iter()
            .map(|e| {
                let aim = node_aim(e.pull, e.far)
                    .or_else(|| first_pivot.get(&(e.pull, e.far)).copied())
                    .unwrap_or(fabric.joints[e.far].location);
                let (pivot, _) = self.connection_pivot(e.end_pos, e.axis, &e.connection, aim);
                let ring_center = self.connection_ring_center(e.end_pos, e.axis, &e.connection);
                let body = if is_swivel(fabric, e.pull) {
                    fork::swivel_capsules(pivot, aim)
                } else {
                    let radial = (pivot - ring_center).normalize();
                    fork::capsules(pivot, e.axis.cross(radial).normalize(), (aim - pivot).normalize())
                };
                ((e.pull, e.near), pivot, body.with_arm(ring_center, pivot, ring_radius))
            })
            .collect();

        // Each node plate's terminals, pinned through its holes square to the
        // plate, aimed at their cables' far ends; a swivel pins straight on.
        let stack_pivots: HashMap<(IntervalKey, JointKey), Vec3> =
            frames.iter().map(|(end, pivot, _)| (*end, *pivot)).collect();
        for (&node, plate) in &plates {
            for (cable, hole) in plate.holes {
                if is_swivel(fabric, cable) {
                    continue;
                }
                let far = fabric.intervals[cable].other_joint(node);
                let aim = plates
                    .get(&far)
                    .and_then(|other| other.hole(cable))
                    .or_else(|| stack_pivots.get(&(cable, far)).copied())
                    .unwrap_or(fabric.joints[far].location);
                let cable_dir = (aim - hole).normalize();
                let pin_axis = plate.normal.reject_from(cable_dir).normalize_or(plate.normal);
                frames.push(((cable, node), hole, fork::capsules(hole, pin_axis, cable_dir)));
            }
        }

        // Every cable needs room for a terminal at each end plus free wire;
        // one from a node plate without it is a swivel link instead, if it
        // has room for a swivel.
        let pivots: HashMap<(IntervalKey, JointKey), Vec3> =
            frames.iter().map(|(end, pivot, _)| (*end, *pivot)).collect();
        for (key, cable) in fabric.intervals.iter() {
            if !cable.role.is_pull_like() || is_swivel(fabric, key) {
                continue;
            }
            let ends = (pivots.get(&(key, cable.alpha_key)), pivots.get(&(key, cable.omega_key)));
            if let (Some(alpha), Some(omega)) = ends {
                let span = alpha.distance(*omega);
                let at_plate = [cable.alpha_key, cable.omega_key].iter().any(|joint| plates.contains_key(joint));
                if choosing && at_plate && (fork::SWIVEL_LENGTH.f32()..fork::min_cable_length()).contains(&span) {
                    self.links.insert(key, span);
                }
                if !self.links.contains_key(&key) && span < fork::min_cable_length() {
                    self.short_cables.insert(key);
                }
            }
        }
        for ((key, near), pivot, assembly) in &mut frames {
            if self.links.contains_key(key) {
                let far = pivots[&(*key, fabric.intervals[*key].other_joint(*near))];
                assembly.body = fork::link_capsules(*pivot, far).body;
            }
        }

        for i in 0..frames.len() {
            for j in (i + 1)..frames.len() {
                if frames[i].1.distance(frames[j].1) > fork::OVERLAP_PREFILTER {
                    continue;
                }
                // A cable's own two terminals meet only when it's too short,
                // which `short_cables` already marks.
                if frames[i].0 .0 == frames[j].0 .0 {
                    continue;
                }
                if fork::assemblies_collide(&frames[i].2, &frames[j].2) {
                    self.culprits.insert(frames[i].0);
                    self.culprits.insert(frames[j].0);
                    self.collisions.push([frames[i].0, frames[j].0]);
                }
            }
        }

        // A plate with no equilateral fit to its cables.
        for (&node, plate) in &plates {
            if plate.jammed {
                self.culprits.insert((fabric.bridle_nodes[&node], node));
            }
        }

        // A plate against any terminal or swivel but its own, and against
        // other plates. Its collisions are marked on the node's link.
        let plate_bodies: Vec<((IntervalKey, JointKey), Assembly)> = plates
            .iter()
            .map(|(&node, plate)| {
                let holes = plate.holes.map(|(_, hole)| hole);
                ((fabric.bridle_nodes[&node], node), fork::plate_capsules(holes))
            })
            .collect();
        for (i, (plate_end, plate_body)) in plate_bodies.iter().enumerate() {
            let (link, node) = *plate_end;
            let near = |pivot: Vec3| pivot.distance(fabric.joints[node].location) <= fork::OVERLAP_PREFILTER;
            let others = frames
                .iter()
                .filter(|(end, pivot, _)| end.1 != node && end.0 != link && near(*pivot))
                .map(|(end, _, body)| (*end, body));
            let other_plates = plate_bodies[i + 1..].iter().map(|(end, body)| (*end, body));
            for (end, body) in others.chain(other_plates) {
                if fork::assemblies_collide(plate_body, body) {
                    self.culprits.insert(*plate_end);
                    self.culprits.insert(end);
                    self.collisions.push([*plate_end, end]);
                }
            }
        }
        self.plates = plates;
    }

    /// A bridle node's plate, as last built with the slot assignments.
    pub fn node_plate(&self, node: JointKey) -> Option<&NodePlate> {
        self.plates.get(&node)
    }

    /// The pin-to-pin length of `cable` if it is a swivel link.
    pub fn link_length(&self, cable: IntervalKey) -> Option<f32> {
        self.links.get(&cable).copied()
    }

    /// Rebuild the bridle nodes' plates where the joints are now,
    /// keeping the slot assignments: unlike the forks on the stacks, they
    /// are kept in place, not worked out as they are drawn.
    pub fn refresh_nodes(&mut self, fabric: &Fabric) {
        if !self.connections.is_empty() {
            self.build_nodes(fabric);
        }
    }

    fn build_nodes(&mut self, fabric: &Fabric) {
        self.plates = fabric
            .bridle_nodes
            .keys()
            .map(|&node| (node, self.build_plate(fabric, node)))
            .collect();
    }

    /// A bridle node's plate, each of its three cables aimed at its far end:
    /// the far pivot on a strut's stack, or the far joint itself where that
    /// is another node. A swivel aims at the pivot it hangs from.
    fn build_plate(&self, fabric: &Fabric, node: JointKey) -> NodePlate {
        let cables: Vec<(IntervalKey, Vec3, bool)> = fabric
            .intervals
            .iter()
            .filter(|(_, i)| i.role.is_pull_like() && i.touches(node))
            .map(|(key, i)| {
                let far = i.other_joint(node);
                let at_far = fabric.joints[far].location;
                let aim = if fabric.bridle_nodes.contains_key(&far) {
                    at_far
                } else {
                    self.pull_end_pivot(fabric, key, far).unwrap_or(at_far)
                };
                (key, aim, is_swivel(fabric, key))
            })
            .collect();
        NodePlate::new(fabric.joints[node].location, [cables[0], cables[1], cables[2]])
    }

    /// Make every stack's slot order the mirror image of its twin's, so the
    /// built structure is as symmetric as its labels. Many orders tie on
    /// collisions, and tiny geometric differences between the two sides
    /// would otherwise break the tie differently. One order is chosen for
    /// both: the one with the fewest colliding forks on the two stacks
    /// together, keeping the leading side's own choice (the side with the
    /// smaller label) among equals. Stacks on the mirror plane are their own
    /// image and stay as optimised; a stack whose image can't be matched
    /// cable for cable is left alone.
    fn mirror_assignments(&mut self, fabric: &Fabric) {
        let Some(mirror) = MirrorImage::of(fabric) else {
            return;
        };
        let image = |joint: JointKey| mirror.image(fabric, joint);
        let pushes: Vec<(IntervalKey, JointKey, JointKey)> = fabric
            .intervals
            .iter()
            .filter(|(_, i)| i.role == Role::Pushing)
            .map(|(key, i)| (key, i.alpha_key, i.omega_key))
            .collect();
        for (push_key, alpha, omega) in pushes {
            for (end, joint, far) in [(IntervalEnd::Alpha, alpha, omega), (IntervalEnd::Omega, omega, alpha)] {
                let (Some(label), Some(far_label), Some(mirror_joint), Some(mirror_far)) =
                    (fabric.joints[joint].label, fabric.joints[far].label, image(joint), image(far))
                else {
                    continue;
                };
                // The stack with the smaller labels leads. At a hinge pin on
                // the mirror plane the pin is its own image but its struts
                // are twins; a strut lying in the plane is its own image and
                // is left to `twin_assignments`.
                let own = (label, far_label);
                let mirrored = (mirror.symmetry.map_label(label), mirror.symmetry.map_label(far_label));
                if mirrored <= own {
                    continue;
                }
                let Some(mirror_push) = fabric
                    .joining((mirror_joint, mirror_far))
                    .filter(|key| fabric.intervals[*key].role == Role::Pushing)
                else {
                    continue;
                };
                let mirror_end = if fabric.intervals[mirror_push].alpha_key == mirror_joint {
                    IntervalEnd::Alpha
                } else {
                    IntervalEnd::Omega
                };
                let (Some(leading), Some(following)) =
                    (self.connections(push_key, end), self.connections(mirror_push, mirror_end))
                else {
                    continue;
                };
                let mirrored: Option<Vec<Option<PullConnection>>> = leading
                    .iter()
                    .map(|slot| match slot {
                        None => Some(None),
                        Some(connection) => {
                            let cable = &fabric.intervals[connection.pull_interval_key];
                            let twin = fabric.joining((image(cable.alpha_key)?, image(cable.omega_key)?))?;
                            Some(Some(PullConnection {
                                pull_interval_key: twin,
                                attachment_index: connection.attachment_index,
                                reach: connection.reach,
                                lift: connection.lift,
                            }))
                        }
                    })
                    .collect();
                let Some(mirrored) = mirrored else { continue };
                let cables = |slots: &[Option<PullConnection>]| -> HashSet<IntervalKey> {
                    slots.iter().flatten().map(|c| c.pull_interval_key).collect()
                };
                if cables(&mirrored) != cables(following) {
                    continue;
                }
                // Occupied slots in order, each cable with its twin.
                let pairs: Vec<(IntervalKey, IntervalKey)> = leading
                    .iter()
                    .zip(&mirrored)
                    .filter_map(|(a, b)| Some((a.as_ref()?.pull_interval_key, b.as_ref()?.pull_interval_key)))
                    .collect();
                let lead = (push_key, end, joint);
                let follow = (mirror_push, mirror_end, mirror_joint);
                let lead_aims = self.stack_aims(fabric, joint, pairs.iter().map(|p| p.0));
                let follow_aims = self.stack_aims(fabric, mirror_joint, pairs.iter().map(|p| p.1));
                let mut best: Option<(usize, Vec<usize>)> = None;
                let standard = self.dimensions.pivot_radius;
                let heights = |aims: &[(IntervalKey, Vec3)], order: &[usize]| -> Vec<Placed> {
                    order.iter().enumerate().map(|(slot, &k)| (aims[k].0, aims[k].1, slot, standard, 0)).collect()
                };
                for order in permutations(pairs.len()) {
                    let total = self.stack_collisions(fabric, lead, &heights(&lead_aims, &order))
                        + self.stack_collisions(fabric, follow, &heights(&follow_aims, &order));
                    if best.as_ref().is_none_or(|(fewest, _)| total < *fewest) {
                        best = Some((total, order));
                    }
                }
                let Some((_, order)) = best else { continue };
                let place = |connections: &mut PullConnections, end: IntervalEnd, twin: bool| {
                    for slot in 0..ATTACHMENT_POINTS {
                        connections.set_connection(end, slot, None);
                    }
                    for (slot, &k) in order.iter().enumerate() {
                        let cable = if twin { pairs[k].1 } else { pairs[k].0 };
                        connections.set_connection(
                            end,
                            slot,
                            Some(PullConnection {
                                pull_interval_key: cable,
                                attachment_index: slot,
                                reach: None,
                                lift: 0,
                            }),
                        );
                    }
                };
                place(self.connections.get_mut(push_key).expect("stack assigned"), end, false);
                place(self.connections.get_mut(mirror_push).expect("mirror stack assigned"), mirror_end, true);
            }
        }
    }

    /// Each cable's aim from the stack at `joint`: its far end's pivot.
    fn stack_aims(&self, fabric: &Fabric, joint: JointKey, cables: impl Iterator<Item = IntervalKey>) -> Vec<(IntervalKey, Vec3)> {
        cables
            .map(|cable| {
                let far = fabric.intervals[cable].other_joint(joint);
                let aim = self
                    .pull_end_pivot(fabric, cable, far)
                    .unwrap_or(fabric.joints[far].location);
                (cable, aim)
            })
            .collect()
    }

    /// Colliding fork pairs on one stack with its cables placed as given.
    fn stack_collisions(
        &self,
        fabric: &Fabric,
        (push_key, end, _): (IntervalKey, IntervalEnd, JointKey),
        placed: &[Placed],
    ) -> usize {
        let [alpha, omega] = self.stack_ends(fabric, push_key);
        let (end_pos, axis) = match end {
            IntervalEnd::Alpha => alpha,
            IntervalEnd::Omega => omega,
        };
        let ring_radius = fabric.dimensions.push_radius.f32();
        let capsules: Vec<_> = placed
            .iter()
            .map(|&(cable, aim, slot, reach, lift)| {
                let ring_center = self.lifted_ring_center(end_pos, axis, slot, lift);
                let (pivot, _) = ConnectorDimensions::pivot_from_center(ring_center, axis, aim, reach);
                let body = if is_swivel(fabric, cable) {
                    fork::swivel_capsules(pivot, aim)
                } else {
                    let radial = (pivot - ring_center).normalize();
                    fork::capsules(pivot, axis.cross(radial).normalize(), (aim - pivot).normalize())
                };
                body.with_arm(ring_center, pivot, ring_radius)
            })
            .collect();
        (0..capsules.len())
            .flat_map(|i| (i + 1..capsules.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| fork::assemblies_collide(&capsules[i], &capsules[j]))
            .count()
    }

    /// On a strut lying in the mirror plane each stack is its own mirror
    /// image, so to stay symmetric every pair of mirror-twin cables shares
    /// one height on a single custom disc (`Part::CustomDisc`), laser cut
    /// with a boss for each; cables lying in the plane keep standard rings.
    /// Each twin pair's bosses reach out as far as `DISC_REACHES` needs to
    /// clear their forks, and extra washers (`GAP_WASHERS`) can lift a unit
    /// clear of the one below. Order, gaps and reaches are searched together
    /// for the fewest colliding forks, then the fewest extra washers, then
    /// the shortest bosses.
    fn plane_assignments(&mut self, fabric: &Fabric) {
        let Some(mirror) = MirrorImage::of(fabric) else {
            return;
        };
        let standard = self.dimensions.pivot_radius;
        let pushes: Vec<(IntervalKey, JointKey, JointKey)> = fabric
            .intervals
            .iter()
            .filter(|(_, i)| i.role == Role::Pushing)
            .map(|(key, i)| (key, i.alpha_key, i.omega_key))
            .collect();
        for (push_key, alpha, omega) in pushes {
            if mirror.image(fabric, alpha) != Some(alpha) || mirror.image(fabric, omega) != Some(omega) {
                continue;
            }
            for (end, joint) in [(IntervalEnd::Alpha, alpha), (IntervalEnd::Omega, omega)] {
                let Some(connections) = self.connections(push_key, end) else {
                    continue;
                };
                let cables: Vec<IntervalKey> = connections.iter().flatten().map(|c| c.pull_interval_key).collect();
                let far = |cable: IntervalKey| fabric.intervals[cable].other_joint(joint);
                let mut twins: Vec<[IntervalKey; 2]> = Vec::new();
                let mut singles: Vec<IntervalKey> = Vec::new();
                for &cable in &cables {
                    if twins.iter().flatten().any(|c| *c == cable) {
                        continue;
                    }
                    let twin = mirror
                        .image(fabric, far(cable))
                        .filter(|image| *image != far(cable))
                        .and_then(|image| cables.iter().copied().find(|c| far(*c) == image));
                    match twin {
                        Some(twin) => twins.push([cable, twin]),
                        None => singles.push(cable),
                    }
                }
                if twins.is_empty() {
                    continue;
                }
                let aims: HashMap<IntervalKey, Vec3> =
                    self.stack_aims(fabric, joint, cables.iter().copied()).into_iter().collect();
                // Units by index: the singles, then the disc as the last.
                let disc = singles.len();
                // `gaps[i]` extra washers go below the unit at height i + 1.
                let placed = |order: &[usize], reaches: &[Meters], gaps: &[u8]| -> Vec<Placed> {
                    let mut placed = Vec::new();
                    let mut lift = 0;
                    for (height, &unit) in order.iter().enumerate() {
                        if height > 0 {
                            lift += gaps[height - 1];
                        }
                        if unit == disc {
                            for (pair, &reach) in twins.iter().zip(reaches) {
                                placed.extend(pair.iter().map(|&c| (c, aims[&c], height, reach, lift)));
                            }
                        } else {
                            let cable = singles[unit];
                            placed.push((cable, aims[&cable], height, standard, lift));
                        }
                    }
                    placed
                };
                // Fewest collisions, then fewest extra washers, then the
                // shortest bosses.
                let mut best: Option<((usize, u8, f32), Vec<Placed>)> = None;
                for order in permutations(disc + 1) {
                    for gaps in gap_choices(disc) {
                        for reaches in reach_choices(twins.len()) {
                            let candidate = placed(&order, &reaches, &gaps);
                            let score = (
                                self.stack_collisions(fabric, (push_key, end, joint), &candidate),
                                gaps.iter().sum::<u8>(),
                                reaches.iter().map(|reach| reach.f32()).sum::<f32>(),
                            );
                            if best.as_ref().is_none_or(|(fewest, _)| score < *fewest) {
                                best = Some((score, candidate));
                            }
                        }
                    }
                }
                let Some((_, best)) = best else { continue };
                let target = self.connections.get_mut(push_key).expect("stack assigned");
                for slot in 0..ATTACHMENT_POINTS {
                    target.set_connection(end, slot, None);
                }
                for (index, (cable, _, height, reach, lift)) in best.into_iter().enumerate() {
                    let on_disc = twins.iter().flatten().any(|c| *c == cable);
                    target.set_connection(
                        end,
                        index,
                        Some(PullConnection {
                            pull_interval_key: cable,
                            attachment_index: height,
                            reach: on_disc.then_some(reach),
                            lift,
                        }),
                    );
                }
            }
        }
    }

    /// Assign all pull intervals connected to one push interval to their
    /// optimal attachment slots.
    fn update_push_connections(&mut self, fabric: &Fabric, push_key: IntervalKey) {
        let Some(push_interval) = fabric.intervals.get(push_key) else {
            return;
        };
        if !push_interval.has_role(Role::Pushing) {
            return;
        }
        let push_alpha = push_interval.alpha_key;
        let push_omega = push_interval.omega_key;

        let mut connected_pulls = Vec::new();
        let mut pull_data = Vec::new();
        for (key, interval) in fabric.intervals.iter() {
            if !interval.role.is_pull_like() {
                continue;
            }
            let carried = |end: JointKey| {
                interval.touches(end) && stack_carries(fabric, push_key, end, key)
            };
            if carried(push_alpha) || carried(push_omega) {
                connected_pulls.push((key, interval.alpha_key, interval.omega_key));
                pull_data.push(PullIntervalData {
                    key,
                    alpha_key: interval.alpha_key,
                    omega_key: interval.omega_key,
                    strain: interval.strain,
                    unit: interval.unit,
                });
            }
        }

        let [(alpha_end, _), (omega_end, _)] = self.stack_ends(fabric, push_key);
        let (alpha_points, omega_points) =
            calculate_interval_attachment_points(alpha_end, omega_end, &self.dimensions);

        let mut connections = PullConnections::new();
        connections.reorder_connections(
            &alpha_points,
            &omega_points,
            &fabric.joints,
            &connected_pulls,
            &pull_data,
            push_alpha,
            push_omega,
            (alpha_end, omega_end),
            &self.dimensions,
            fabric.dimensions.push_radius.f32(),
        );
        self.connections.insert(push_key, connections);
    }

    /// Pivot pin position for the given pull's end at joint `near`, aimed at
    /// the pull's far joint. `None` when that end isn't attached to any push.
    /// Aiming at the far pivot instead of the far joint is a second pass over
    /// this: see the renderers, which use it to keep forks collinear with
    /// their cables on short spans.
    pub fn pull_end_pivot(
        &self,
        fabric: &Fabric,
        pull_key: IntervalKey,
        near: JointKey,
    ) -> Option<Vec3> {
        if fabric.bridle_nodes.contains_key(&near) {
            return self.plates.get(&near).and_then(|plate| plate.hole(pull_key));
        }
        for (push_key, push) in fabric.intervals.iter() {
            if !push.has_role(Role::Pushing) {
                continue;
            }
            for end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
                let end_joint = match end {
                    IntervalEnd::Alpha => push.alpha_key,
                    IntervalEnd::Omega => push.omega_key,
                };
                if end_joint != near {
                    continue;
                }
                let Some(conns) = self.connections(push_key, end) else {
                    continue;
                };
                for conn in conns.iter().flatten() {
                    if conn.pull_interval_key != pull_key {
                        continue;
                    }
                    let [alpha_stack, omega_stack] = self.stack_ends(fabric, push_key);
                    let (end_pos, axis_dir) = match end {
                        IntervalEnd::Alpha => alpha_stack,
                        IntervalEnd::Omega => omega_stack,
                    };
                    let pull = &fabric.intervals[pull_key];
                    let other = if pull.alpha_key == near {
                        fabric.joints[pull.omega_key].location
                    } else {
                        fabric.joints[pull.alpha_key].location
                    };
                    let (pivot_pos, _elevation) = self.connection_pivot(end_pos, axis_dir, conn, other);
                    return Some(pivot_pos);
                }
            }
        }
        None
    }

    /// Free pivot elevation angle (degrees) at every cable end.
    pub fn collect_pivot_angles(&self, fabric: &Fabric) -> Vec<f32> {
        let mut angles = Vec::new();
        for (push_key, push_interval) in fabric.intervals.iter() {
            if !push_interval.has_role(Role::Pushing) {
                continue;
            }
            let [(alpha_end, alpha_axis), (omega_end, omega_axis)] =
                self.stack_ends(fabric, push_key);

            for interval_end in [IntervalEnd::Alpha, IntervalEnd::Omega] {
                let (end_pos, axis_dir, end_key) = match interval_end {
                    IntervalEnd::Alpha => (alpha_end, alpha_axis, push_interval.alpha_key),
                    IntervalEnd::Omega => (omega_end, omega_axis, push_interval.omega_key),
                };
                let Some(connections) = self.connections(push_key, interval_end) else {
                    continue;
                };
                for connection in connections.iter().flatten() {
                    let Some(pull_interval) = fabric.intervals.get(connection.pull_interval_key)
                    else {
                        continue;
                    };
                    let pull_other_end = if pull_interval.alpha_key == end_key {
                        fabric.joints[pull_interval.omega_key].location
                    } else {
                        fabric.joints[pull_interval.alpha_key].location
                    };
                    let (_, elevation_deg) =
                        self.connection_pivot(end_pos, axis_dir, connection, pull_other_end);
                    angles.push(elevation_deg);
                }
            }
        }
        angles
    }
}

/// Whether `cable` is a bridle's link that is a swivel, its node plate
/// hanging close to the strut end.
pub fn is_swivel(fabric: &Fabric, cable: IntervalKey) -> bool {
    fabric.bridle_swivels.contains(&cable)
}

/// A hinge pin: the joint where two struts end.
fn is_hinge_pin(fabric: &Fabric, joint: JointKey) -> bool {
    fabric
        .intervals
        .values()
        .filter(|interval| interval.role == Role::Pushing && interval.touches(joint))
        .count()
        > 1
}

/// Whether the stack at `joint` on strut `push_key` carries cable `pull_key`.
/// At an ordinary strut end it carries every cable there. At a hinge pin each
/// cable belongs to the strut recorded by `hinge`, and a cable added later
/// (a guy line) attaches to the pin itself, on neither stack.
fn stack_carries(fabric: &Fabric, push_key: IntervalKey, joint: JointKey, pull_key: IntervalKey) -> bool {
    if !is_hinge_pin(fabric, joint) {
        return true;
    }
    fabric.hinge_strut.get(&(pull_key, joint)) == Some(&push_key)
}

/// A cable placed on a stack: `(cable, aim, height, boss reach, lift)`.
type Placed = (IntervalKey, Vec3, usize, Meters, u8);

/// How far a custom disc's bosses may reach from the strut axis, shortest
/// first: from the standard ring's pivot radius outward. Laser-cut steel
/// takes any of these. ESTIMATE of what stays stiff enough.
const DISC_REACHES: [Meters; 5] = [Meters(0.030), Meters(0.040), Meters(0.050), Meters(0.060), Meters(0.070)];

/// Extra washers that may go into one gap on a mirror-plane stack.
const GAP_WASHERS: [u8; 5] = [0, 2, 4, 6, 8];

/// Every combination of `GAP_WASHERS` for `gaps` gaps.
fn gap_choices(gaps: usize) -> Vec<Vec<u8>> {
    (0..gaps).fold(vec![Vec::new()], |combos, _| {
        combos
            .iter()
            .flat_map(|combo| {
                GAP_WASHERS.iter().map(move |&washers| {
                    let mut next = combo.clone();
                    next.push(washers);
                    next
                })
            })
            .collect()
    })
}

/// Every combination of `DISC_REACHES` for `pairs` twin pairs.
fn reach_choices(pairs: usize) -> Vec<Vec<Meters>> {
    (0..pairs).fold(vec![Vec::new()], |combos, _| {
        combos
            .iter()
            .flat_map(|combo| {
                DISC_REACHES.iter().map(move |&reach| {
                    let mut next = combo.clone();
                    next.push(reach);
                    next
                })
            })
            .collect()
    })
}

/// Every ordering of `0..n`, the identity first.
fn permutations(n: usize) -> Vec<Vec<usize>> {
    fn extend(prefix: &mut Vec<usize>, n: usize, out: &mut Vec<Vec<usize>>) {
        if prefix.len() == n {
            out.push(prefix.clone());
            return;
        }
        for k in 0..n {
            if !prefix.contains(&k) {
                prefix.push(k);
                extend(prefix, n, out);
                prefix.pop();
            }
        }
    }
    let mut out = Vec::new();
    extend(&mut Vec::with_capacity(n), n, &mut out);
    out
}

/// Joints' mirror images by label symmetry.
struct MirrorImage<'a> {
    symmetry: &'a LabelSymmetry,
    by_label: HashMap<JointLabel, JointKey>,
}

impl<'a> MirrorImage<'a> {
    fn of(fabric: &'a Fabric) -> Option<Self> {
        let symmetry = fabric.label_symmetry.as_ref()?;
        let by_label = fabric
            .joints
            .iter()
            .filter_map(|(key, joint)| joint.label.map(|label| (label, key)))
            .collect();
        Some(MirrorImage { symmetry, by_label })
    }

    fn image(&self, fabric: &Fabric, joint: JointKey) -> Option<JointKey> {
        let label = fabric.joints[joint].label?;
        self.by_label.get(&self.symmetry.map_label(label)).copied()
    }
}
