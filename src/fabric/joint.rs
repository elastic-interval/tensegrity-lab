/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

use crate::fabric::physics::{Physics, SurfaceInteraction};
use crate::fabric::{Fabric, Force, JointKey, Location, Velocity};
use crate::units::Grams;
use crate::ITERATION_DURATION;
use cgmath::num_traits::zero;
use cgmath::{InnerSpace, MetricSpace, Point3};

impl Fabric {
    /// Create a joint with a specific path (for structured brick creation)
    pub fn create_joint_with_path(&mut self, point: Point3<f32>, path: JointPath) -> JointKey {
        self.joints.insert(Joint::new(point, path))
    }

    /// Create a joint with default path (for legacy code and non-brick joints)
    pub fn create_joint(&mut self, point: Point3<f32>) -> JointKey {
        self.create_joint_with_path(point, JointPath::default())
    }

    pub fn location(&self, key: JointKey) -> Point3<f32> {
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

    pub fn distance(&self, alpha_key: JointKey, omega_key: JointKey) -> f32 {
        self.location(alpha_key).distance(self.location(omega_key))
    }

    pub fn ideal(&self, alpha_key: JointKey, omega_key: JointKey, strain: f32) -> f32 {
        let distance = self.distance(alpha_key, omega_key);
        distance / (1.0 + strain * distance)
    }

    /// Find a joint by its path (linear search, only for setup)
    pub fn joint_key_by_path(&self, path: &JointPath) -> Option<JointKey> {
        self.joints
            .iter()
            .find(|(_, joint)| &joint.path == path)
            .map(|(key, _)| key)
    }
}

pub const AMBIENT_MASS: Grams = Grams(100.0);

/// Hierarchical path identifying a joint's position in the structure.
/// Used for identifying symmetric groups during pretensing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct JointPath {
    /// Branch path from root (0=A, 1=B, ... for each branching point)
    pub branches: Vec<u8>,
    /// Local joint index within the brick (0-5 for AlphaX, OmegaX, AlphaY, OmegaY, AlphaZ, OmegaZ)
    pub local_index: u8,
}

impl JointPath {
    pub fn new(local_index: u8) -> Self {
        Self {
            branches: Vec::new(),
            local_index,
        }
    }

    pub fn with_branches(branches: Vec<u8>, local_index: u8) -> Self {
        Self {
            branches,
            local_index,
        }
    }

    /// Extend this path with a new branch, keeping the same local index
    pub fn extend(&self, branch: u8) -> Self {
        let mut branches = self.branches.clone();
        branches.push(branch);
        Self {
            branches,
            local_index: self.local_index,
        }
    }

    /// Create a new path with the same branches but a different local index
    pub fn with_local_index(&self, local_index: u8) -> Self {
        Self {
            branches: self.branches.clone(),
            local_index,
        }
    }

    /// Get the depth (number of branches from root)
    pub fn depth(&self) -> usize {
        self.branches.len()
    }

    /// Get the axis (0=X, 1=Y, 2=Z) derived from local_index
    /// For single twist bricks: 0,1=X; 2,3=Y; 4,5=Z
    pub fn axis(&self) -> u8 {
        self.local_index / 2
    }

    /// Key for symmetric grouping: (depth, axis)
    pub fn symmetric_key(&self) -> (usize, u8) {
        (self.depth(), self.axis())
    }

    /// Display without run-length encoding (e.g., "AAAAAAAAA[1]" instead of "A9[1]")
    pub fn expanded(&self) -> String {
        let mut s = String::new();
        for &b in &self.branches {
            s.push((b'A' + b) as char);
        }
        s.push_str(&format!("[{}]", self.local_index));
        s
    }
}

impl std::fmt::Display for JointPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Leg:Depth format for columns: "BAAAAAAAA" -> "B:9", "AAAA" -> "A:4"
        // Prism joints: "BAAAAAAAP" -> "B:9P" (P is special marker, not counted in depth)
        // Complex paths: "AAAABCCC" -> "A:4B:3"
        if self.branches.is_empty() {
            return write!(f, "[{}]", self.local_index);
        }

        // Check for trailing P (prism marker = branch 15)
        let (main_branches, has_prism) = if *self.branches.last().unwrap() == 15 {
            (&self.branches[..self.branches.len() - 1], true)
        } else {
            (&self.branches[..], false)
        };

        if main_branches.is_empty() {
            // Just a prism on seed
            if has_prism {
                write!(f, "P")?;
            }
        } else {
            // Check if this is a simple column (first branch + all A's after)
            let first = main_branches[0];
            let is_simple_column = main_branches[1..].iter().all(|&b| b == 0);

            if is_simple_column {
                // Collapse to leg:total_depth
                let letter = (b'A' + first) as char;
                write!(f, "{}:{}", letter, main_branches.len())?;
            } else {
                // Show each segment
                let mut i = 0;
                while i < main_branches.len() {
                    let b = main_branches[i];
                    let letter = (b'A' + b) as char;
                    let mut count = 1;
                    while i + count < main_branches.len() && main_branches[i + count] == b {
                        count += 1;
                    }
                    write!(f, "{}:{}", letter, count)?;
                    i += count;
                }
            }
            if has_prism {
                write!(f, "P")?;
            }
        }
        write!(f, "[{}]", self.local_index)
    }
}

impl std::str::FromStr for JointPath {
    type Err = String;

    /// Parse a JointPath from a string like "A:9[1]" or "B:9P[1]" or "[5]"
    /// Format: segments of letter:count, optional P suffix, then [local_index]
    /// Examples: "[0]" = local_index 0, "B:9[1]" = leg B depth 9, "B:9P[1]" = leg B depth 9 + prism
    /// For simple columns: "B:9[1]" expands to B + 8×A (first is leg, rest fill with A)
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty JointPath string".to_string());
        }

        // Find the bracketed local_index at the end
        let (branch_part, local_str) = match (s.rfind('['), s.rfind(']')) {
            (Some(open), Some(close)) if close == s.len() - 1 && open < close => {
                (&s[..open], &s[open + 1..close])
            }
            _ => {
                return Err(format!(
                    "JointPath must end with [local_index], got: '{}'",
                    s
                ))
            }
        };

        // Parse local_index
        let local_index = local_str
            .parse::<u8>()
            .map_err(|_| format!("Invalid local index: '{}'", local_str))?;

        // Check for trailing P (prism marker)
        let (main_part, has_prism) = if branch_part.ends_with('P') {
            (&branch_part[..branch_part.len() - 1], true)
        } else {
            (branch_part, false)
        };

        // Parse segments in letter:count format
        let mut branches = Vec::new();
        let chars: Vec<char> = main_part.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_uppercase() {
                let branch = c as u8 - b'A';
                i += 1;

                // Expect colon
                if i >= chars.len() || chars[i] != ':' {
                    return Err(format!(
                        "Expected ':' after letter '{}' at position {}",
                        c, i
                    ));
                }
                i += 1; // skip colon

                // Read count digits
                let mut count_str = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    count_str.push(chars[i]);
                    i += 1;
                }
                let count: usize = count_str
                    .parse()
                    .map_err(|_| format!("Invalid count after '{}:'", c))?;

                // For simple column format (single segment without prism following),
                // first branch is the leg, remaining depth fills with A
                if branches.is_empty() && i >= chars.len() {
                    // Single segment - this is leg:total_depth format
                    branches.push(branch);
                    for _ in 1..count {
                        branches.push(0); // fill with A
                    }
                } else {
                    // Multiple segments - each segment specifies exact branches
                    for _ in 0..count {
                        branches.push(branch);
                    }
                }
            } else {
                return Err(format!("Invalid character in branch part: '{}'", c));
            }
        }

        // Add prism marker if present
        if has_prism {
            branches.push(15); // P = branch 15
        }

        Ok(JointPath::with_branches(branches, local_index))
    }
}

impl From<&str> for JointPath {
    fn from(s: &str) -> Self {
        s.parse().expect("Invalid JointPath string")
    }
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub path: JointPath,
    pub location: Location,
    pub force: Force,
    pub velocity: Velocity,
    pub accumulated_mass: Grams,
}

impl Joint {
    pub fn new(location: Point3<f32>, path: JointPath) -> Joint {
        Joint {
            path,
            location,
            force: zero(),
            velocity: zero(),
            accumulated_mass: AMBIENT_MASS,
        }
    }

    pub fn reset(&mut self) {
        self.force = zero();
        self.accumulated_mass = AMBIENT_MASS;
    }

    pub fn reset_with_mass(&mut self, ambient_mass: Grams) {
        self.force = zero();
        self.accumulated_mass = ambient_mass;
    }

    pub fn iterate(&mut self, physics: &Physics) {
        let drag = physics.drag();
        let viscosity = physics.viscosity();
        let mass = *self.accumulated_mass;
        let dt = ITERATION_DURATION.secs;

        // Force is in Newtons, mass in grams (converted to kg)
        // a = F/m gives m/s², multiply by dt gives velocity change in m/s
        let force_velocity = (self.force / mass) * dt;

        match &physics.surface {
            None => {
                // No surface, no gravity - free floating
                let speed_squared = self.velocity.magnitude2();
                self.velocity += force_velocity - self.velocity * speed_squared * viscosity * dt;
                self.velocity *= 1.0 - drag * dt;
            }
            Some(surface) => {
                let result = surface.interact(SurfaceInteraction {
                    altitude: self.location.y,
                    velocity: self.velocity,
                    force_velocity,
                    drag,
                    viscosity,
                    mass,
                    dt,
                });
                self.velocity = result.velocity;
                if let Some(y) = result.clamp_y {
                    self.location.y = y;
                }
            }
        }
        self.location = &self.location + self.velocity * dt;
    }
}
