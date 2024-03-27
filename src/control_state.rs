use crate::fabric::interval::Role;

#[derive(Clone, Debug, Copy)]
pub struct IntervalDetails {
    pub alpha_index: usize,
    pub omega_index: usize,
    pub length: f32,
    pub role: Role,
}

#[derive(Clone, Debug, Default)]
pub enum ControlState {
    #[default]
    Choosing,
    Viewing,
    ShowingInterval(IntervalDetails),
    SettingLength(IntervalDetails),
}
