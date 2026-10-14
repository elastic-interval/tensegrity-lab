use crate::units::{Meters, Percent, Seconds};

#[derive(Debug, Clone)]
pub struct SettlePhase {
    pub seconds: Seconds,
    pub guy_lines: Vec<GuyLine>,
}

/// A cable from a labelled joint to a fixed ground point, created on landing.
/// The ground point is offset horizontally (x, z) from the landed joint, and
/// the cable's length is a percentage of the landed joint-to-ground distance.
#[derive(Debug, Clone)]
pub struct GuyLine {
    pub joint: String,
    pub ground_offset: (Meters, Meters),
    pub length: Percent,
}
