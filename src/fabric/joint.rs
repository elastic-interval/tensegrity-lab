/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::interval::Role;
use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::{Physics, SurfaceInteraction};
use crate::fabric::{Fabric, JointKey};
use crate::units::{Grams, Meters, Unit};
use glam::Vec3;
use std::fmt::{self, Display, Formatter};

/// Engraver-friendly joint label. `OffAxis` reads as `<letter><brick>.<position>`
/// (limb joints); `Axial` reads as `Z<index>` (apex / on-mirror / unsymmetric).
/// Ordering is derived (OffAxis before Axial, then by letter/brick/position),
/// giving `LabelSymmetry::orbit_key` its canonical minimum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JointLabel {
    OffAxis { letter: char, brick: u8, position: u8 },
    Axial { index: u16 },
}

impl Display for JointLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JointLabel::OffAxis { letter, brick, position } => {
                write!(f, "{letter}{brick:02}.{position}")
            }
            JointLabel::Axial { index } => write!(f, "Z{index}"),
        }
    }
}

impl Fabric {
    /// Create a joint with a specific path (for structured brick creation)
    pub fn create_joint_with_path(&mut self, point: Vec3, path: JointPath) -> JointKey {
        self.joints.insert(Joint::new(point, path))
    }

    /// Create a joint with default path (for legacy code and non-brick joints)
    pub fn create_joint(&mut self, point: Vec3) -> JointKey {
        self.create_joint_with_path(point, JointPath::default())
    }

    pub fn location(&self, key: JointKey) -> Vec3 {
        self.joints[key].location
    }

    pub fn remove_joint(&mut self, key: JointKey) {
        // Remove all intervals that touch this joint
        let to_remove: Vec<_> = self
            .intervals
            .iter()
            .filter_map(|(interval_key, interval)| {
                if interval.alpha_key == key || interval.omega_key == key {
                    Some(interval_key)
                } else {
                    None
                }
            })
            .collect();
        for interval_key in to_remove {
            self.remove_interval(interval_key);
        }
        // Simply remove the joint - no index adjustment needed with SlotMap!
        self.joints.remove(key);
    }

    /// Join the struts ending at `keep` and `merge` where they cross: both are
    /// cut back to the crossing and `merge` folds into `keep`, which becomes the
    /// hinge. Intervals between the two vanish; where both reached the same
    /// joint only one remains (a non-bow-tie wins). Rebuilt intervals start
    /// unstrained at their new length. Merging a symmetric pair (each other's
    /// image) puts the hinge on the symmetry axis, so it takes the next axial
    /// label (`Z<n>`); otherwise it keeps `keep`'s label.
    pub fn hinge(&mut self, keep: JointKey, merge: JointKey) {
        let strut_far_end = |end: JointKey| {
            self.intervals
                .values()
                .find(|i| i.role == Role::Pushing && i.touches(end))
                .expect("hinge joint without a strut")
                .other_joint(end)
        };
        let (keep_far, merge_far) = (strut_far_end(keep), strut_far_end(merge));
        let on_axis = match (&self.label_symmetry, self.joints[keep].label, self.joints[merge].label) {
            (Some(symmetry), Some(keep_label), Some(merge_label)) => {
                symmetry.orbit(keep_label).contains(&merge_label)
            }
            _ => false,
        };
        let crossing = crossing_point(
            (self.location(keep), self.location(keep_far)),
            (self.location(merge), self.location(merge_far)),
        );
        let touching: Vec<_> = self
            .intervals
            .iter()
            .filter(|(_, i)| i.touches(keep) || i.touches(merge))
            .map(|(key, _)| key)
            .collect();
        let mut rebuilt: Vec<(JointKey, Role)> = Vec::new();
        for key in touching {
            let interval = self.remove_interval(key);
            let end = if interval.touches(keep) { keep } else { merge };
            let other = interval.other_joint(end);
            if other == keep || other == merge {
                continue;
            }
            match rebuilt.iter_mut().find(|(joint, _)| *joint == other) {
                Some((_, role)) if *role == Role::BowTie => *role = interval.role,
                Some(_) => {}
                None => rebuilt.push((other, interval.role)),
            }
        }
        self.joints.remove(merge);
        if on_axis {
            self.joints[keep].label = Some(self.next_axial_label());
        }
        self.joints[keep].location = crossing;
        for (other, role) in rebuilt {
            self.create_strained_interval(keep, other, role, 0.0);
        }
    }

    /// The first unused `Z<n>` label, for joints added on the mirror plane.
    pub fn next_axial_label(&self) -> JointLabel {
        let highest = self
            .joints
            .values()
            .filter_map(|joint| match joint.label {
                Some(JointLabel::Axial { index }) => Some(index),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        JointLabel::Axial { index: highest + 1 }
    }

    pub fn distance(&self, alpha_key: JointKey, omega_key: JointKey) -> Meters {
        Meters(self.location(alpha_key).distance(self.location(omega_key)))
    }

    /// Find a joint by its path (linear search, only for setup)
    pub fn joint_key_by_path(&self, path: &JointPath) -> Option<JointKey> {
        self.joints
            .iter()
            .find(|(_, joint)| &joint.path == path)
            .map(|(key, _)| key)
    }

    /// Find a joint by its engraver label (e.g. `"C03.10"`, `"Z6"`).
    pub fn joint_key_by_label(&self, label: &str) -> Option<JointKey> {
        self.joints
            .iter()
            .find(|(k, _)| self.joint_label(*k) == label)
            .map(|(key, _)| key)
    }
}

/// Midpoint of the closest approach between two (non-parallel) lines.
fn crossing_point((p0, p1): (Vec3, Vec3), (q0, q1): (Vec3, Vec3)) -> Vec3 {
    let (d1, d2, r) = (p1 - p0, q1 - q0, p0 - q0);
    let (a, b, c) = (d1.dot(d1), d1.dot(d2), d2.dot(d2));
    let (d, e) = (d1.dot(r), d2.dot(r));
    let denom = a * c - b * b;
    let s = (b * e - c * d) / denom;
    let t = (a * e - b * d) / denom;
    (p0 + d1 * s + q0 + d2 * t) / 2.0
}

pub const AMBIENT_MASS: Grams = Grams(100.0);

#[derive(Clone, Debug)]
pub struct Joint {
    pub path: JointPath,
    pub label: Option<JointLabel>,
    pub location: Vec3,
    pub force: Vec3,
    pub velocity: Vec3,
    pub accumulated_mass: Grams,
    /// Own base mass when this joint isn't a normal connector hub (e.g. a point
    /// along a bendable cable). `None` means use the fabric's `joint_mass`.
    pub point_mass: Option<Grams>,
}

impl Joint {
    pub fn new(location: Vec3, path: JointPath) -> Joint {
        Joint {
            path,
            label: None,
            location,
            force: Vec3::ZERO,
            velocity: Vec3::ZERO,
            accumulated_mass: AMBIENT_MASS,
            point_mass: None,
        }
    }

    pub fn reset(&mut self) {
        self.force = Vec3::ZERO;
        self.accumulated_mass = self.point_mass.unwrap_or(AMBIENT_MASS);
    }

    pub fn reset_with_mass(&mut self, ambient_mass: Grams) {
        self.force = Vec3::ZERO;
        self.accumulated_mass = self.point_mass.unwrap_or(ambient_mass);
    }

    /// Half-kick: update velocity by half timestep using current force
    /// v += 0.5 * (F/m) * dt
    pub fn half_kick(&mut self, dt: f32) {
        let mass = self.accumulated_mass.f32();
        let acceleration = self.force / mass;
        self.velocity += acceleration * dt * 0.5;
    }

    /// Drift: update position using current velocity
    /// x += v * dt
    pub fn drift(&mut self, dt: f32) {
        self.location += self.velocity * dt;
    }

    /// Apply damping and surface interaction (called after second half-kick in Verlet)
    /// This handles both air damping and surface collision/friction
    /// Note: Gravity is applied as a force in iterate_verlet, so we skip gravity here
    pub fn apply_damping_and_surface(&mut self, physics: &Physics, dt: f32) {
        let drag = physics.drag();
        let viscosity = physics.viscosity();

        match &physics.surface {
            None => {
                // No surface - apply quadratic viscosity and linear drag.
                // Stable form: v' = v / (1 + speed² · visc · dt). See
                // fabric/physics.rs for the rationale.
                let speed_squared = self.velocity.length_squared();
                self.velocity /= 1.0 + speed_squared * viscosity * dt;
                self.velocity *= 1.0 - drag * dt;
            }
            Some(surface) => {
                let surface_tolerance = 0.01 * surface.scale;

                if self.location.y > surface_tolerance {
                    // Above surface - just apply air damping (gravity already in forces)
                    let speed_squared = self.velocity.length_squared();
                    self.velocity /= 1.0 + speed_squared * viscosity * dt;
                    self.velocity *= 1.0 - drag * dt;
                } else {
                    // On or below surface - use surface interaction for collision/friction
                    // Pass force_velocity as ZERO since forces already applied via half_kick
                    let result = surface.interact(SurfaceInteraction {
                        altitude: self.location.y,
                        velocity: self.velocity,
                        force_velocity: Vec3::ZERO,
                        drag,
                        viscosity,
                        mass: self.accumulated_mass.f32(),
                        dt,
                    });
                    self.velocity = result.velocity;
                    if let Some(y) = result.clamp_y {
                        self.location.y = y;
                    }
                }
            }
        }
    }
}
