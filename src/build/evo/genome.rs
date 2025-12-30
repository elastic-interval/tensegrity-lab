/// A genome encodes which random numbers to skip in a deterministic sequence.
///
/// Each `u8` in the `skips` vector represents the number of steps to the next skip.
/// For example: `[5, 12, 3]` means skip at positions 5, 17 (5+12), and 20 (17+3).
///
/// Run-length encoding: A value of 0 means "advance 256 positions without a skip".
/// This allows encoding arbitrarily long gaps. Example: `[0, 0, 10]` means skip at
/// position 256 + 256 + 10 = 522.
///
/// This compact representation allows infinite branching while storing minimal data.
/// Inserting a skip at any position changes all subsequent decisions (slippage).
#[derive(Clone, Debug, Default)]
pub struct Genome {
    /// Each byte is the offset from the previous position to the next skip.
    /// Zero means "advance 256 positions" (run-length encoding for long gaps).
    skips: Vec<u8>,
}

impl Genome {
    /// Create an empty genome (no skips)
    pub fn new() -> Self {
        Self { skips: Vec::new() }
    }

    /// Create a genome from raw skip offsets
    pub fn from_offsets(offsets: Vec<u8>) -> Self {
        Self { skips: offsets }
    }

    /// Create a mutated copy with a new skip inserted at the given absolute position.
    /// Uses run-length encoding (0 = advance 256) for positions > 255.
    pub fn with_skip_at(&self, absolute_position: usize) -> Self {
        let mut new_skips = Vec::with_capacity(self.skips.len() + 3);
        let mut current_pos = 0usize;
        let mut inserted = false;
        let mut i = 0;

        while i < self.skips.len() {
            let offset = self.skips[i];

            // Handle run-length encoding (0 = advance 256)
            let advance = if offset == 0 { 256 } else { offset as usize };
            let next_pos = current_pos + advance;

            if !inserted && absolute_position < next_pos {
                // Insert new skip before this position
                Self::encode_offset(&mut new_skips, absolute_position - current_pos);
                inserted = true;
                current_pos = absolute_position;

                // Now encode the remaining distance to the next skip (if not RLE)
                if offset != 0 {
                    Self::encode_offset(&mut new_skips, next_pos - current_pos);
                    current_pos = next_pos;
                }
                // If offset was 0 (RLE), we still need to continue advancing
                else {
                    // Remaining 256-block distance after our new skip
                    let remaining = next_pos - current_pos;
                    if remaining > 0 {
                        Self::encode_offset(&mut new_skips, remaining);
                        current_pos = next_pos;
                    }
                }
            } else if !inserted && absolute_position == next_pos && offset != 0 {
                // Skip at exactly the same position as existing - just keep existing
                new_skips.push(offset);
                current_pos = next_pos;
                inserted = true; // Already have a skip here
            } else {
                new_skips.push(offset);
                if offset != 0 {
                    current_pos = next_pos;
                } else {
                    current_pos += 256;
                }
            }
            i += 1;
        }

        // If we haven't inserted yet, add at the end
        if !inserted {
            Self::encode_offset(&mut new_skips, absolute_position - current_pos);
        }

        Self { skips: new_skips }
    }

    /// Encode an offset into the skip vector, using 0 for 256-byte runs
    fn encode_offset(skips: &mut Vec<u8>, mut offset: usize) {
        while offset >= 256 {
            skips.push(0); // 0 means advance 256
            offset -= 256;
        }
        if offset > 0 || skips.is_empty() {
            skips.push(offset as u8);
        }
    }

    /// Check if position `n` in the virtual sequence should be skipped.
    pub fn should_skip(&self, position: usize) -> bool {
        let mut current_pos = 0usize;
        for &offset in &self.skips {
            // 0 means advance 256 without a skip (run-length encoding)
            if offset == 0 {
                current_pos += 256;
                continue;
            }
            current_pos += offset as usize;
            if current_pos == position {
                return true;
            }
            if current_pos > position {
                return false;
            }
        }
        false
    }

    /// Get all skip positions as absolute values (for debugging)
    pub fn skip_positions(&self) -> Vec<usize> {
        let mut positions = Vec::new();
        let mut current = 0usize;
        for &offset in &self.skips {
            // 0 means advance 256 without a skip
            if offset == 0 {
                current += 256;
                continue;
            }
            current += offset as usize;
            positions.push(current);
        }
        positions
    }

    /// Number of skips in this genome
    pub fn len(&self) -> usize {
        self.skips.len()
    }

    /// Check if genome has no skips
    pub fn is_empty(&self) -> bool {
        self.skips.is_empty()
    }

    /// Get the raw skip offsets (for serialization)
    pub fn offsets(&self) -> &[u8] {
        &self.skips
    }
}
