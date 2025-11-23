/*
 * Copyright (c) 2020. Beautiful Code BV, Rotterdam, Netherlands
 * Licensed under GNU GENERAL PUBLIC LICENSE Version 3.
 */

//! On-demand fabric analysis for detecting oscillations and measuring properties
//!
//! FabricSampler records joint positions over time and analyzes movement patterns
//! and fabric properties like mass. Unlike the old position history (which tracked
//! every joint always), this sampler only activates when requested, making it
//! efficient and targeted.

use cgmath::{MetricSpace, Point3};
use crate::fabric::Fabric;
use crate::Age;

/// Number of samples to collect before analysis
const DEFAULT_SAMPLE_COUNT: usize = 240;

/// Multi-resolution sampling strategy: dense initially, progressively sparser
/// This allows detection of both fast and slow oscillations within the same analysis
const SAMPLING_PHASES: &[(usize, usize)] = &[
    (60, 1),   // Phase 1: 60 samples, every 1 frame  (1.0s total)
    (60, 2),   // Phase 2: 60 samples, every 2 frames (2.0s total)
    (60, 4),   // Phase 3: 60 samples, every 4 frames (4.0s total)
    (40, 8),   // Phase 4: 40 samples, every 8 frames (5.3s total)
];
// Total: 220 samples over ~12.3 seconds

/// A single snapshot of all joint positions at one point in time
#[derive(Clone, Debug)]
struct Sample {
    positions: Vec<Point3<f32>>,
    age: Age,  // When this sample was taken
}

/// Movement classification for a joint (6 tiers)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementType {
    Frozen,  // < 0.01mm
    Stable,  // 0.01-0.1mm
    Micro,   // 0.1-0.5mm
    Small,   // 0.5-2mm
    Medium,  // 2-5mm
    Large,   // > 5mm
}

/// Oscillation frequency classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrequencyClass {
    Drift,   // < 0.5 Hz (very slow or no oscillation)
    Slow,    // 0.5-2 Hz
    Medium,  // 2-5 Hz
    Fast,    // > 5 Hz
}

/// Oscillation pattern over time
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OscillationPattern {
    Growing,  // Amplitude increasing (unstable!)
    Stable,   // Roughly constant amplitude
    Damped,   // Amplitude decreasing
}

/// Analysis results for a single joint
#[derive(Debug, Clone)]
pub struct JointAnalysis {
    pub joint_index: usize,
    pub movement_type: MovementType,
    pub amplitude_mm: f32,        // Peak-to-peak amplitude
    pub avg_speed_mm_per_s: f32,  // Average speed
    pub frequency_hz: f32,        // Estimated frequency
    pub frequency_class: FrequencyClass,
    pub pattern: OscillationPattern,
}

/// Complete fabric analysis results
#[derive(Debug, Clone)]
pub struct FabricAnalysis {
    pub joint_analyses: Vec<JointAnalysis>,
    pub frozen_count: usize,
    pub stable_count: usize,
    pub micro_count: usize,
    pub small_count: usize,
    pub medium_count: usize,
    pub large_count: usize,
    pub max_amplitude_mm: f32,
    pub max_amplitude_joint: usize,
    pub growing_pattern_count: usize,
    pub total_mass_kg: f32,
}

/// On-demand sampler for fabric analysis with multi-resolution sampling
pub struct FabricSampler {
    samples: Vec<Sample>,
    joint_count: usize,
    is_complete: bool,
    current_phase: usize,
    phase_sample_count: usize,
    frames_since_last_sample: usize,
}

impl FabricSampler {
    /// Create a new sampler
    pub fn new(joint_count: usize) -> Self {
        Self {
            samples: Vec::with_capacity(DEFAULT_SAMPLE_COUNT),
            joint_count,
            is_complete: false,
            current_phase: 0,
            phase_sample_count: 0,
            frames_since_last_sample: 0,
        }
    }

    /// Record a sample from the current fabric state using multi-resolution sampling
    pub fn record_sample(&mut self, fabric: &Fabric) {
        if self.is_complete {
            return;
        }

        // Check if we've completed all phases
        if self.current_phase >= SAMPLING_PHASES.len() {
            self.is_complete = true;
            return;
        }

        let (phase_samples, frame_interval) = SAMPLING_PHASES[self.current_phase];

        // Time to sample?
        if self.frames_since_last_sample >= frame_interval {
            let positions: Vec<Point3<f32>> = fabric.joints.iter()
                .map(|j| j.location)
                .collect();

            self.samples.push(Sample {
                positions,
                age: fabric.age,
            });

            self.phase_sample_count += 1;
            self.frames_since_last_sample = 0;

            // Move to next phase?
            if self.phase_sample_count >= phase_samples {
                self.current_phase += 1;
                self.phase_sample_count = 0;
            }
        } else {
            self.frames_since_last_sample += 1;
        }
    }

    /// Check if sampling is complete
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Get current sample count
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Format current sampling progress
    pub fn format_progress(&self) -> String {
        let total_samples: usize = SAMPLING_PHASES.iter().map(|(count, _)| count).sum();
        let current_count = self.samples.len();
        let phase = self.current_phase + 1;
        let total_phases = SAMPLING_PHASES.len();

        format!("Sampling... {}/{} (Phase {}/{})",
            current_count,
            total_samples,
            phase,
            total_phases)
    }

    /// Analyze collected samples to detect oscillations and movement patterns
    pub fn analyze(&self, fabric: &Fabric, physics: &crate::fabric::physics::Physics) -> Option<FabricAnalysis> {
        if !self.is_complete || self.samples.is_empty() {
            return None;
        }

        let scale = fabric.scale;
        let mut joint_analyses = Vec::with_capacity(self.joint_count);
        let mut frozen_count = 0;
        let mut stable_count = 0;
        let mut micro_count = 0;
        let mut small_count = 0;
        let mut medium_count = 0;
        let mut large_count = 0;
        let mut growing_pattern_count = 0;
        let mut max_amplitude_mm = 0.0;
        let mut max_amplitude_joint = 0;

        // Calculate total mass (convert from grams to kg)
        let total_mass_kg = fabric.calculate_total_mass(physics).0 / 1000.0;

        // Analyze each joint
        for joint_idx in 0..self.joint_count {
            let analysis = self.analyze_joint(joint_idx, scale);

            match analysis.movement_type {
                MovementType::Frozen => frozen_count += 1,
                MovementType::Stable => stable_count += 1,
                MovementType::Micro => micro_count += 1,
                MovementType::Small => small_count += 1,
                MovementType::Medium => medium_count += 1,
                MovementType::Large => large_count += 1,
            }

            if matches!(analysis.pattern, OscillationPattern::Growing) {
                growing_pattern_count += 1;
            }

            if analysis.amplitude_mm > max_amplitude_mm {
                max_amplitude_mm = analysis.amplitude_mm;
                max_amplitude_joint = joint_idx;
            }

            joint_analyses.push(analysis);
        }

        Some(FabricAnalysis {
            joint_analyses,
            frozen_count,
            stable_count,
            micro_count,
            small_count,
            medium_count,
            large_count,
            max_amplitude_mm,
            max_amplitude_joint,
            growing_pattern_count,
            total_mass_kg,
        })
    }

    /// Analyze movement of a single joint (with non-uniform sampling)
    fn analyze_joint(&self, joint_idx: usize, scale: f32) -> JointAnalysis {
        // Extract positions and ages for this joint across all samples
        let positions: Vec<Point3<f32>> = self.samples.iter()
            .map(|sample| sample.positions[joint_idx])
            .collect();
        let ages: Vec<f32> = self.samples.iter()
            .map(|sample| sample.age.0.as_secs_f32())
            .collect();

        // Calculate bounding box to find amplitude
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for pos in &positions {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
            min_z = min_z.min(pos.z);
            max_z = max_z.max(pos.z);
        }

        // Calculate amplitude (maximum extent in any direction, in mm)
        let extent_x = (max_x - min_x) * scale;
        let extent_y = (max_y - min_y) * scale;
        let extent_z = (max_z - min_z) * scale;
        let amplitude_mm = extent_x.max(extent_y).max(extent_z);

        // Calculate average speed (accounting for non-uniform sampling)
        let mut total_distance = 0.0;
        for i in 1..positions.len() {
            total_distance += positions[i].distance(positions[i - 1]);
        }

        // Use actual time duration, not sample count
        let total_time = if ages.len() >= 2 {
            ages[ages.len() - 1] - ages[0]
        } else {
            1.0
        };

        // Average speed in mm/s
        let avg_speed_mm_per_s = if total_time > 0.0 {
            (total_distance * scale) / total_time
        } else {
            0.0
        };

        // Classify movement type (6 tiers based on amplitude)
        let movement_type = if amplitude_mm < 0.01 {
            MovementType::Frozen
        } else if amplitude_mm < 0.1 {
            MovementType::Stable
        } else if amplitude_mm < 0.5 {
            MovementType::Micro
        } else if amplitude_mm < 2.0 {
            MovementType::Small
        } else if amplitude_mm < 5.0 {
            MovementType::Medium
        } else {
            MovementType::Large
        };

        // Frequency analysis via zero-crossing detection
        // We analyze the dominant axis (the one with largest extent)
        let dominant_positions: Vec<f32> = if extent_x >= extent_y && extent_x >= extent_z {
            positions.iter().map(|p| p.x).collect()
        } else if extent_y >= extent_z {
            positions.iter().map(|p| p.y).collect()
        } else {
            positions.iter().map(|p| p.z).collect()
        };

        // Calculate center point
        let center: f32 = dominant_positions.iter().sum::<f32>() / dominant_positions.len() as f32;

        // Count zero crossings (when position crosses the center line)
        let mut crossings = 0;
        for i in 1..dominant_positions.len() {
            let prev_above = dominant_positions[i - 1] >= center;
            let curr_above = dominant_positions[i] >= center;
            if prev_above != curr_above {
                crossings += 1;
            }
        }

        // Each full cycle has 2 crossings (up and down)
        let cycles = crossings as f32 / 2.0;

        // Calculate actual duration from timestamps (non-uniform sampling)
        let duration_seconds = if ages.len() >= 2 {
            ages[ages.len() - 1] - ages[0]
        } else {
            1.0 // Fallback
        };
        let frequency_hz = if duration_seconds > 0.0 {
            cycles / duration_seconds
        } else {
            0.0
        };

        // Classify frequency
        let frequency_class = if frequency_hz < 0.5 {
            FrequencyClass::Drift
        } else if frequency_hz < 2.0 {
            FrequencyClass::Slow
        } else if frequency_hz < 5.0 {
            FrequencyClass::Medium
        } else {
            FrequencyClass::Fast
        };

        // Pattern detection: compare first half vs second half amplitude
        let mid_point = positions.len() / 2;

        // First half amplitude
        let mut first_min_x = f32::MAX;
        let mut first_max_x = f32::MIN;
        let mut first_min_y = f32::MAX;
        let mut first_max_y = f32::MIN;
        let mut first_min_z = f32::MAX;
        let mut first_max_z = f32::MIN;

        for pos in &positions[..mid_point] {
            first_min_x = first_min_x.min(pos.x);
            first_max_x = first_max_x.max(pos.x);
            first_min_y = first_min_y.min(pos.y);
            first_max_y = first_max_y.max(pos.y);
            first_min_z = first_min_z.min(pos.z);
            first_max_z = first_max_z.max(pos.z);
        }
        let first_extent_x = first_max_x - first_min_x;
        let first_extent_y = first_max_y - first_min_y;
        let first_extent_z = first_max_z - first_min_z;
        let first_amplitude = first_extent_x.max(first_extent_y).max(first_extent_z);

        // Second half amplitude
        let mut second_min_x = f32::MAX;
        let mut second_max_x = f32::MIN;
        let mut second_min_y = f32::MAX;
        let mut second_max_y = f32::MIN;
        let mut second_min_z = f32::MAX;
        let mut second_max_z = f32::MIN;

        for pos in &positions[mid_point..] {
            second_min_x = second_min_x.min(pos.x);
            second_max_x = second_max_x.max(pos.x);
            second_min_y = second_min_y.min(pos.y);
            second_max_y = second_max_y.max(pos.y);
            second_min_z = second_min_z.min(pos.z);
            second_max_z = second_max_z.max(pos.z);
        }
        let second_extent_x = second_max_x - second_min_x;
        let second_extent_y = second_max_y - second_min_y;
        let second_extent_z = second_max_z - second_min_z;
        let second_amplitude = second_extent_x.max(second_extent_y).max(second_extent_z);

        // Compare amplitudes (with 20% threshold to avoid noise)
        let pattern = if second_amplitude > first_amplitude * 1.2 {
            OscillationPattern::Growing
        } else if second_amplitude < first_amplitude * 0.8 {
            OscillationPattern::Damped
        } else {
            OscillationPattern::Stable
        };

        JointAnalysis {
            joint_index: joint_idx,
            movement_type,
            amplitude_mm,
            avg_speed_mm_per_s,
            frequency_hz,
            frequency_class,
            pattern,
        }
    }
}

impl FabricAnalysis {
    /// Format analysis as human-readable text with adaptive histogram
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        // Display total mass first
        lines.push(format!("Mass: {:.3} kg", self.total_mass_kg));

        // Sort amplitudes to create adaptive histogram
        let mut amplitudes: Vec<f32> = self.joint_analyses.iter()
            .map(|a| a.amplitude_mm)
            .collect();
        amplitudes.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let total = amplitudes.len();
        if total == 0 {
            return lines.join("\n");
        }

        let min_amp = amplitudes[0];
        let max_amp = amplitudes[total - 1];

        // Create adaptive bins based on quantiles (10 bins for detailed distribution)
        let bin_count = 10.min(total); // Use fewer bins if we have few joints
        let mut bins: Vec<(f32, f32, usize)> = Vec::new();

        for i in 0..bin_count {
            let start_idx = (i * total) / bin_count;
            let end_idx = ((i + 1) * total) / bin_count;
            let range_min = if i == 0 { min_amp } else { amplitudes[start_idx] };
            let range_max = if i == bin_count - 1 { max_amp } else { amplitudes[end_idx - 1] };
            let count = end_idx - start_idx;
            bins.push((range_min, range_max, count));
        }

        // Display adaptive histogram
        for (range_min, range_max, count) in bins.iter() {
            if *count > 0 {
                let pct = 100.0 * *count as f32 / total as f32;
                if range_min == range_max || range_max - range_min < 0.001 {
                    lines.push(format!("{:.3}mm: {} ({:.1}%)",
                        range_min, count, pct));
                } else {
                    lines.push(format!("{:.3}-{:.3}mm: {} ({:.1}%)",
                        range_min, range_max, count, pct));
                }
            }
        }

        lines.push(format!("Max: {:.3}mm @ J{}",
            self.max_amplitude_mm,
            self.max_amplitude_joint));

        // Top active joints with detailed info
        let mut sorted = self.joint_analyses.clone();
        sorted.sort_by(|a, b| b.amplitude_mm.partial_cmp(&a.amplitude_mm).unwrap());

        // Show top joints if there's significant variation
        let top_count = 5.min(total);
        if max_amp > min_amp * 2.0 && top_count > 0 {
            for analysis in sorted.iter().take(top_count) {
                if analysis.amplitude_mm > min_amp * 1.5 {
                    lines.push(format!(
                        "J{}: {:.3}mm {:.1}Hz {:?}",
                        analysis.joint_index,
                        analysis.amplitude_mm,
                        analysis.frequency_hz,
                        analysis.pattern
                    ));
                }
            }
        }

        lines.join("\n")
    }
}
