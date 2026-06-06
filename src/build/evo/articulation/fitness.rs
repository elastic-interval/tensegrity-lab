//! Fitness for an articulating brick: a compliant mechanism with a single
//! stable equilibrium. Three properties, combined *multiplicatively* (a
//! weighted average would let a structure win on the cheap dimensions
//! while scoring nothing on the one that matters):
//!
//! - **rest gate** — settled to a finite, properly-tensioned equilibrium.
//!   Zero here zeroes the whole score: no rest state, no articulation.
//! - **gain** — large deformation for little actuator effort. A *ratio*,
//!   so the limp trap (low effort, no motion) can't win.
//! - **reversibility** — returns to rest after release, measured relative
//!   to how far it deformed, so a structure that barely moves can't claim
//!   reversibility for free.

use crate::build::evo::articulation::trial::ArticulationOutcome;

/// Half-saturation point for gain: the raw deformation-per-effort at
/// which the gain term reaches 0.5. Smooth (raw/(raw+K)) rather than a
/// hard cap, so there is always a gradient toward higher gain and the
/// population never plateaus at a ceiling. The Omni seed sits near
/// raw-gain 0.86; evolved joints reach several times that.
const GAIN_HALF: f32 = 2.0;
/// Avoids divide-by-zero when a compliant actuator meets no resistance.
const EFFORT_EPS: f32 = 0.02;
/// Below this deformation (metres) the brick isn't articulating; gain and
/// reversibility are meaningless, so the score is zero.
const DEFORM_EPS: f32 = 0.01;

#[derive(Clone, Copy, Debug, Default)]
pub struct FitnessBreakdown {
    pub gate: bool,
    pub gain: f32,
    pub reversibility: f32,
    pub score: f32,
}

pub fn breakdown(outcome: &ArticulationOutcome) -> FitnessBreakdown {
    let gate = outcome.settled && outcome.tensioned && outcome.finite;
    if !gate || outcome.deformation < DEFORM_EPS {
        return FitnessBreakdown {
            gate,
            ..Default::default()
        };
    }
    let raw_gain = outcome.deformation / (outcome.effort + EFFORT_EPS);
    let gain = raw_gain / (raw_gain + GAIN_HALF);
    let reversibility = (1.0 - outcome.return_error / outcome.deformation).clamp(0.0, 1.0);
    FitnessBreakdown {
        gate,
        gain,
        reversibility,
        score: gain * reversibility,
    }
}

pub fn score(outcome: &ArticulationOutcome) -> f32 {
    breakdown(outcome).score
}
