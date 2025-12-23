/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

/// Column marker in branches vec (255 = column continuation, not a face choice)
pub const COLUMN_MARKER: u8 = 255;
/// Prism marker in branches vec
pub const PRISM_MARKER: u8 = 254;

/// Hierarchical path identifying a joint's position in the structure.
/// Used for identifying symmetric groups during pretensing.
///
/// # Format
/// The string format uses single characters for different path elements:
/// - `A`-`V`: Face choices (0-21)
/// - `X` + count: Column steps (e.g., `X8` = 8 columns)
/// - `Y`: Prism marker
/// - `Z` + number: Local index delimiter (e.g., `Z1` = local index 1)
///
/// # Examples
/// - `Z0` = seed brick joint with local index 0
/// - `AX8Z1` = face A + 8 columns + local index 1
/// - `AX8YZ1` = face A + 8 columns + prism + local index 1
/// - `X4ABX12Z0` = 4 columns + faces A,B + 12 columns + local index 0
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct JointPath {
    /// Branch path from root (0-21 for face choices A-V, 255 for column, 254 for prism)
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

    /// Display without run-length encoding (e.g., "AXXXXXXXX[1]" instead of "AX8[1]")
    pub fn expanded(&self) -> String {
        let mut s = String::new();
        for &b in &self.branches {
            match b {
                COLUMN_MARKER => s.push('X'),
                PRISM_MARKER => s.push('Y'),
                15 => s.push('Y'), // Legacy prism marker
                _ => s.push((b'A' + b) as char),
            }
        }
        s.push_str(&format!("[{}]", self.local_index));
        s
    }
}

impl std::fmt::Display for JointPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format: face choices as A-V; columns as X+count; prism as Y; local as ZN
        // Examples: "AX8Z1" = face A, 8 column, local 1
        //           "X5BX12Z3" = 5 column, face B, 12 column, local 3
        //           "AX9YZ1" = face A, 9 column, prism, local 1
        if self.branches.is_empty() {
            return write!(f, "Z{}", self.local_index);
        }

        // Check for trailing prism marker (254 new, 15 legacy)
        let (main_branches, has_prism) = match self.branches.last() {
            Some(&PRISM_MARKER) | Some(&15) => {
                (&self.branches[..self.branches.len() - 1], true)
            }
            _ => (&self.branches[..], false),
        };

        // Group branches into segments: column (X) or face choice (A-V)
        let mut i = 0;
        while i < main_branches.len() {
            let b = main_branches[i];
            if b == COLUMN_MARKER {
                // Column segment - count consecutive column markers
                let mut count = 1;
                while i + count < main_branches.len()
                    && main_branches[i + count] == COLUMN_MARKER
                {
                    count += 1;
                }
                write!(f, "X{}", count)?;
                i += count;
            } else {
                // Face choice - single letter A-V (0-21)
                let letter = (b'A' + b) as char;
                write!(f, "{}", letter)?;
                i += 1;
            }
        }

        if has_prism {
            write!(f, "Y")?;
        }
        write!(f, "Z{}", self.local_index)
    }
}

impl std::str::FromStr for JointPath {
    type Err = String;

    /// Parse a JointPath from a string like "AX8YZ1" or "X5BX12Z3" or "Z0"
    /// Format: segments of (A-V for face choice) or (X+count for column), optional Y suffix (prism), then Z+local_index
    /// Examples: "Z0" = local_index 0, "AX8Z1" = face A + 8 column + local 1, "AX8YZ1" = face A + 8 column + prism + local 1
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty JointPath string".to_string());
        }

        // Find the ZN local_index at the end (last Z followed by digits)
        let (branch_part, local_str) = match s.rfind('Z') {
            Some(z_pos) => (&s[..z_pos], &s[z_pos + 1..]),
            None => {
                return Err(format!(
                    "JointPath must end with Z+local_index, got: '{}'",
                    s
                ))
            }
        };

        // Parse local_index
        let local_index = local_str
            .parse::<u8>()
            .map_err(|_| format!("Invalid local index: '{}'", local_str))?;

        // Check for trailing Y (prism marker)
        let (main_part, has_prism) = if branch_part.ends_with('Y') {
            (&branch_part[..branch_part.len() - 1], true)
        } else {
            (branch_part, false)
        };

        // Parse segments: X+count for columns, A-V for face choices
        let mut branches = Vec::new();
        let chars: Vec<char> = main_part.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            if c == 'X' {
                // Column segment - read count
                i += 1;
                let mut count_str = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    count_str.push(chars[i]);
                    i += 1;
                }
                let count: usize = count_str
                    .parse()
                    .map_err(|_| format!("Invalid count after 'X'"))?;
                for _ in 0..count {
                    branches.push(COLUMN_MARKER);
                }
            } else if c.is_ascii_uppercase() && c >= 'A' && c <= 'V' {
                // Face choice - single letter A-V
                let branch = c as u8 - b'A';
                branches.push(branch);
                i += 1;
            } else {
                return Err(format!("Invalid character in branch part: '{}'", c));
            }
        }

        // Add prism marker if present
        if has_prism {
            branches.push(PRISM_MARKER);
        }

        Ok(JointPath::with_branches(branches, local_index))
    }
}

impl From<&str> for JointPath {
    fn from(s: &str) -> Self {
        s.parse().expect("Invalid JointPath string")
    }
}
