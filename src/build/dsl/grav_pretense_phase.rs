use crate::units::Seconds;

#[derive(Debug, Clone)]
pub struct GravPretensePhase {
    pub seconds: Option<Seconds>,
    pub min_push_strain: f32,
}
