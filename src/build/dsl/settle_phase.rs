use crate::build::dsl::joint_names::Erased;
use crate::units::{Meters, Percent, Seconds};

#[derive(Debug, Clone)]
pub struct SettlePhase {
    pub seconds: Seconds,
    pub guy_lines: Vec<GuyLine>,
}

/// A cable from named joints to fixed ground points, created on landing, one
/// for each twin. Each ground point is offset horizontally from where its
/// joint lands, (outward from the mirror plane, forward along z), so twins'
/// lines mirror each other; and the cable's length is a percentage of the
/// landed joint-to-ground distance.
#[derive(Debug, Clone)]
pub struct GuyLine {
    pub joint: Erased,
    pub ground_offset: (Meters, Meters),
    pub length: Percent,
}
