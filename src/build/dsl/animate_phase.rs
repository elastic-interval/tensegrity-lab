use crate::fabric::UniqueId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuscleDirection {
    Alpha, // First muscle group - contracts while Omega relaxes
    Omega, // Second muscle group - contracts while Alpha relaxes
}

#[derive(Debug, Clone)]
pub struct AnimatePhase {
    pub contraction: Option<f32>,
    pub frequency_hz: f32, // Obligatory - cycles per second (Hertz)
    pub muscle_intervals: Vec<(UniqueId, UniqueId, MuscleDirection)>,
}
