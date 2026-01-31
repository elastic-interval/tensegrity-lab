use crate::units::Seconds;

/// Default target compression for push intervals during gravitational pretensing (2%)
/// Higher than zero-G because gravity shifts the equilibrium point
pub const DEFAULT_GRAV_MIN_PUSH_STRAIN: f32 = 0.02;
/// Default maximum compression per extension round during gravitational pretensing (5%)
/// Higher than zero-G to allow rebalancing at the new equilibrium
pub const DEFAULT_GRAV_MAX_PUSH_STRAIN: f32 = 0.05;

#[derive(Debug, Clone, Default)]
pub struct GravPretensePhase {
    /// Duration per extension step
    pub seconds: Option<Seconds>,
    /// Target compression for push intervals (default 1%)
    pub min_push_strain: Option<f32>,
    /// Maximum compression per extension round (default 3%)
    pub max_push_strain: Option<f32>,
}
