//! # Vulcanization: Cross-linking Adjacent Bricks
//!
//! Bow tie cables cross-link adjacent tensegrity bricks, transforming a
//! flexible spine-like structure into a rigid unified whole.

use std::collections::{HashMap, HashSet};

use crate::fabric::interval::Span::Measuring;
use crate::fabric::interval::{Interval, Role, Span};
use crate::fabric::{Fabric, IntervalKey, JointKey};
use crate::units::Seconds;

const VULCANIZE_DURATION: Seconds = Seconds(1.0);

/// How the differential ratio affects contraction.
#[derive(Debug, Clone, Copy)]
pub enum VulcanizeMode {
    /// target = current × contraction
    Constant,
    /// target = current × contraction × ratio
    Linear,
    /// target = current × contraction × ratio²
    Quadratic,
}

/// When two paths from opposite ends of a strut meet at the same interval,
/// we call this a "bridge" meeting - they found a shared cable.
const BRIDGE_MEETING: usize = 6;

/// When two paths from opposite ends of a strut meet at the same joint,
/// we call this an "apex" meeting - they converge at a common point.
const APEX_MEETING: usize = 8;

// ============================================================================
// Main Entry Point
// ============================================================================

const DEFAULT_CONTRACTION: f32 = 0.5;

impl Fabric {
    /// Install bow ties as "measuring tapes" before shaping.
    pub fn prepare_vulcanize(&mut self, contraction: f32, mode: VulcanizeMode) {
        let bow_tie_pairs = BowTieFinder::new(self).find_all_bow_tie_pairs();
        for (alpha, omega) in bow_tie_pairs {
            self.create_measuring_interval(alpha, omega, Role::BowTie, contraction, mode);
        }
    }

    /// Activate bow ties. If prepare_vulcanize was called, activates measuring tapes.
    /// Otherwise creates new bow ties with default Constant mode.
    pub fn vulcanize(&mut self) {
        let has_measuring = self
            .intervals
            .values()
            .any(|i| i.role == Role::BowTie && matches!(i.span, Measuring { .. }));

        let start_age = self.age;
        if has_measuring {
            for interval in self.intervals.values_mut() {
                if interval.role != Role::BowTie {
                    continue;
                }
                if let Measuring {
                    baseline,
                    contraction,
                    mode,
                } = interval.span
                {
                    let current = interval.fast_length(&self.joints);
                    let ratio = current / baseline;
                    let target_length = match mode {
                        VulcanizeMode::Constant => current * contraction,
                        VulcanizeMode::Linear => current * contraction * ratio,
                        VulcanizeMode::Quadratic => current * contraction * ratio * ratio,
                    };
                    interval.span = Span::Approaching {
                        start_length: current,
                        target_length,
                        start_age,
                        duration: VULCANIZE_DURATION,
                    };
                }
            }
        } else {
            let bow_tie_pairs = BowTieFinder::new(self).find_all_bow_tie_pairs();
            for (alpha, omega) in bow_tie_pairs {
                let current = self.distance(alpha, omega);
                let target_length = current * DEFAULT_CONTRACTION;
                self.create_approaching_interval(alpha, omega, target_length, Role::BowTie, VULCANIZE_DURATION);
            }
        }
    }
}

// ============================================================================
// Bow Tie Representation
// ============================================================================

/// A bow tie joint pair (alpha, omega) for deduplication.
#[derive(Debug, Clone)]
struct BowTie {
    alpha: JointKey,
    omega: JointKey,
}

impl BowTie {
    fn new(alpha: JointKey, omega: JointKey) -> Self {
        Self { alpha, omega }
    }

    /// Canonical key for deduplication (smaller key first).
    fn key(&self) -> (JointKey, JointKey) {
        if self.alpha < self.omega {
            (self.alpha, self.omega)
        } else {
            (self.omega, self.alpha)
        }
    }
}

// ============================================================================
// Joint Context: Everything we need to know about a joint
// ============================================================================

/// Information about a joint and all intervals connected to it.
#[derive(Debug, Clone)]
struct JointContext {
    key: JointKey,
    push: Option<IntervalKey>, // The strut connected here (if any)
    pulls: Vec<(IntervalKey, Interval)>, // All pull cables connected here
}

impl JointContext {
    fn new(key: JointKey) -> Self {
        Self {
            key,
            push: None,
            pulls: Vec::new(),
        }
    }

    fn add_interval(&mut self, interval_key: IntervalKey, interval: &Interval) {
        if interval.role == Role::Pushing {
            self.push = Some(interval_key);
        } else if interval.role.is_pull_like() && interval.role != Role::PrismPull {
            self.pulls.push((interval_key, interval.clone()));
        }
    }

    /// Is this joint at the end of a strut?
    fn has_strut(&self) -> bool {
        self.push.is_some()
    }

    /// Get the joint at the other end of this joint's strut.
    fn strut_partner(&self, intervals: &HashMap<IntervalKey, Interval>) -> Option<JointKey> {
        self.push.map(|push_key| {
            let push = &intervals[&push_key];
            push.other_joint(self.key)
        })
    }
}

// ============================================================================
// Cable Path: A chain of pull cables from a starting joint
// ============================================================================

/// A path through the fabric following pull cables.
/// Used to find where paths from opposite ends of a strut meet.
#[derive(Debug, Clone)]
struct CablePath {
    /// Joints visited along this path (first is the starting joint)
    joints: Vec<JointKey>,
    /// Intervals traversed (parallel to joints, offset by 1)
    intervals: Vec<Interval>,
}

impl CablePath {
    /// Start a new path from a joint along a pull cable.
    fn start(from: JointKey, along: Interval) -> Self {
        Self {
            joints: vec![from],
            intervals: vec![along],
        }
    }

    /// Extend this path along another pull cable.
    fn extend(&self, along: Interval) -> Option<Self> {
        let next_joint = self.last_interval().joint_with(&along)?;

        // Don't create cycles
        if self.joints.contains(&next_joint) {
            return None;
        }

        let mut extended = self.clone();
        extended.joints.push(next_joint);
        extended.intervals.push(along);
        Some(extended)
    }

    /// The joint where this path currently ends.
    fn end_joint(&self) -> JointKey {
        let last_interval = self.last_interval();
        last_interval.other_joint(*self.joints.last().unwrap())
    }

    /// The last interval in this path.
    fn last_interval(&self) -> &Interval {
        self.intervals.last().unwrap()
    }

    /// The joint one step before the end (useful for bridge patterns).
    fn penultimate_joint(&self) -> JointKey {
        self.joints[self.joints.len() - 1]
    }
}

// ============================================================================
// Meeting Point: Where paths from opposite strut ends converge
// ============================================================================

/// How two paths from opposite ends of a strut meet.
#[derive(Debug, Clone)]
enum Meeting {
    /// Paths share the same final interval (a "bridge" cable between bricks).
    Bridge {
        alpha_path: CablePath,
        omega_path: CablePath,
        bridge_interval: Interval,
    },
    /// Paths end at the same joint (an "apex" where bricks meet).
    Apex {
        alpha_path: CablePath,
        omega_path: CablePath,
        apex_joint: JointKey,
    },
}

impl Meeting {
    fn priority(&self) -> usize {
        match self {
            Meeting::Bridge { .. } => BRIDGE_MEETING,
            Meeting::Apex { .. } => APEX_MEETING,
        }
    }
}

// ============================================================================
// Bow Tie Finder: The main algorithm
// ============================================================================

struct BowTieFinder {
    joint_contexts: HashMap<JointKey, JointContext>,
    intervals: HashMap<IntervalKey, Interval>,
    existing_intervals: HashSet<(JointKey, JointKey)>,
    found_bow_ties: HashMap<(JointKey, JointKey), BowTie>,
}

impl BowTieFinder {
    fn new(fabric: &Fabric) -> Self {
        let mut joint_contexts: HashMap<JointKey, JointContext> = fabric
            .joints
            .iter()
            .map(|(key, _joint)| (key, JointContext::new(key)))
            .collect();

        let mut intervals = HashMap::new();
        for (key, interval) in fabric.intervals.iter() {
            intervals.insert(key, interval.clone());
            if let Some(ctx) = joint_contexts.get_mut(&interval.alpha_key) {
                ctx.add_interval(key, interval);
            }
            if let Some(ctx) = joint_contexts.get_mut(&interval.omega_key) {
                ctx.add_interval(key, interval);
            }
        }

        let existing_intervals: HashSet<_> = intervals
            .values()
            .map(|i| {
                if i.alpha_key < i.omega_key {
                    (i.alpha_key, i.omega_key)
                } else {
                    (i.omega_key, i.alpha_key)
                }
            })
            .collect();

        Self {
            joint_contexts,
            intervals,
            existing_intervals,
            found_bow_ties: HashMap::new(),
        }
    }

    fn find_all_bow_tie_pairs(mut self) -> Vec<(JointKey, JointKey)> {
        self.find_bow_ties_internal();
        self.found_bow_ties.into_keys().collect()
    }

    fn find_bow_ties_internal(&mut self) {
        // Get all struts (push intervals)
        let struts: Vec<_> = self
            .intervals
            .values()
            .filter(|i| i.role == Role::Pushing)
            .cloned()
            .collect();

        // For each strut, find meetings and create appropriate bow ties
        for strut in struts {
            let meetings = self.find_meetings(&strut);
            self.process_meetings(&meetings, &strut);
        }
    }

    /// Find all places where paths from opposite ends of a strut meet.
    fn find_meetings(&self, strut: &Interval) -> Vec<Meeting> {
        let alpha_paths = self.paths_from(strut.alpha_key, 2);
        let omega_paths = self.paths_from(strut.omega_key, 2);

        let mut meetings = Vec::new();

        for alpha_path in &alpha_paths {
            for omega_path in &omega_paths {
                // Check for bridge meeting (same final interval)
                if alpha_path.last_interval().key() == omega_path.last_interval().key() {
                    meetings.push(Meeting::Bridge {
                        alpha_path: alpha_path.clone(),
                        omega_path: omega_path.clone(),
                        bridge_interval: alpha_path.last_interval().clone(),
                    });
                }
                // Check for apex meeting (same final joint)
                else if alpha_path.end_joint() == omega_path.end_joint() {
                    meetings.push(Meeting::Apex {
                        alpha_path: alpha_path.clone(),
                        omega_path: omega_path.clone(),
                        apex_joint: alpha_path.end_joint(),
                    });
                }
            }
        }

        // Sort by priority (bridge meetings first)
        meetings.sort_by_key(|m| m.priority());
        meetings
    }

    /// Generate all paths of a given length starting from a joint.
    fn paths_from(&self, start: JointKey, length: usize) -> Vec<CablePath> {
        let ctx = &self.joint_contexts[&start];

        // Start with paths of length 1
        let mut paths: Vec<_> = ctx
            .pulls
            .iter()
            .map(|(_, interval)| CablePath::start(start, interval.clone()))
            .collect();

        // Extend to desired length
        for _ in 1..length {
            paths = paths
                .iter()
                .flat_map(|path| {
                    let end_ctx = &self.joint_contexts[&path.end_joint()];
                    end_ctx
                        .pulls
                        .iter()
                        .filter_map(|(_, interval)| path.extend(interval.clone()))
                })
                .collect();
        }

        paths
    }

    /// Process meetings to create appropriate bow ties.
    fn process_meetings(&mut self, meetings: &[Meeting], strut: &Interval) {
        // Look for pairs of bridge meetings (the common case for adjacent bricks)
        let bridge_meetings: Vec<_> = meetings
            .iter()
            .filter_map(|m| match m {
                Meeting::Bridge {
                    alpha_path,
                    omega_path,
                    bridge_interval,
                } => Some((alpha_path, omega_path, bridge_interval)),
                _ => None,
            })
            .collect();

        if bridge_meetings.len() >= 2 {
            self.handle_bridge_pair(bridge_meetings[0], bridge_meetings[1]);
            return;
        }

        // Look for pairs of apex meetings
        let apex_meetings: Vec<_> = meetings
            .iter()
            .filter_map(|m| match m {
                Meeting::Apex {
                    alpha_path,
                    omega_path,
                    apex_joint,
                } => Some((alpha_path, omega_path, *apex_joint)),
                _ => None,
            })
            .collect();

        if apex_meetings.len() >= 2 {
            self.handle_apex_pair(apex_meetings[0], apex_meetings[1], strut);
        }
    }

    /// Handle two bridge meetings: add cross-diagonal bow ties.
    fn handle_bridge_pair(
        &mut self,
        meeting1: (&CablePath, &CablePath, &Interval),
        meeting2: (&CablePath, &CablePath, &Interval),
    ) {
        let (alpha1, omega1, _) = meeting1;
        let (alpha2, omega2, _) = meeting2;

        // The four corners of the "rectangle" formed by the two bridges
        let corners = [
            (alpha1.end_joint(), omega2.end_joint()),
            (alpha2.end_joint(), omega1.end_joint()),
        ];

        // Try cross-twist diagonals first
        // These connect joints whose strut partners don't already have a connection
        if let Some(bow_tie) = self.try_cross_twist_diagonal(&corners) {
            self.add_bow_tie(bow_tie);
            return;
        }

        // Fall back to triangle completion
        // Connect a strut joint to the end of a path through a non-strut joint
        let candidates = [
            (alpha1, alpha2),
            (alpha2, alpha1),
            (omega1, omega2),
            (omega2, omega1),
        ];

        for (path, other_path) in candidates {
            let middle_joint = other_path.penultimate_joint();
            if !self.joint_contexts[&middle_joint].has_strut() {
                let bow_tie = BowTie::new(path.joints[0], path.end_joint());
                self.add_bow_tie(bow_tie);
                return;
            }
        }
    }

    /// Try to create a cross-twist diagonal bow tie.
    fn try_cross_twist_diagonal(&self, corners: &[(JointKey, JointKey); 2]) -> Option<BowTie> {
        let valid_diagonals: Vec<_> = corners
            .iter()
            .filter(|&&(a, b)| {
                let a_partner = self.joint_contexts[&a].strut_partner(&self.intervals);
                let b_partner = self.joint_contexts[&b].strut_partner(&self.intervals);
                match (a_partner, b_partner) {
                    (Some(ap), Some(bp)) => !self.interval_exists(ap, bp),
                    _ => false,
                }
            })
            .collect();

        if valid_diagonals.len() == 1 {
            let &(alpha, omega) = valid_diagonals[0];
            Some(BowTie::new(alpha, omega))
        } else {
            None
        }
    }

    /// Handle two apex meetings: add bow ties from apex to opposite brick.
    fn handle_apex_pair(
        &mut self,
        meeting1: (&CablePath, &CablePath, JointKey),
        meeting2: (&CablePath, &CablePath, JointKey),
        _strut: &Interval,
    ) {
        let (alpha1, omega1, _apex1) = meeting1;
        let (alpha2, omega2, apex2) = meeting2;

        let candidates = [
            (alpha1, apex2),
            (alpha2, meeting1.2),
            (omega1, apex2),
            (omega2, meeting1.2),
        ];

        for (path, target_apex) in candidates {
            let through_joint = path.penultimate_joint();
            if self.joint_contexts[&through_joint].has_strut() {
                self.add_bow_tie(BowTie::new(through_joint, target_apex));
            }
        }
    }

    /// Check if an interval already exists between two joints.
    fn interval_exists(&self, a: JointKey, b: JointKey) -> bool {
        let key = if a < b { (a, b) } else { (b, a) };
        self.existing_intervals.contains(&key)
    }

    /// Add a bow tie, avoiding duplicates.
    fn add_bow_tie(&mut self, bow_tie: BowTie) {
        let key = bow_tie.key();
        if !self.existing_intervals.contains(&key) {
            self.found_bow_ties.insert(key, bow_tie);
        }
    }
}
