use crate::fabric::interval::Interval;

#[derive(Clone, Debug, Default)]
pub enum ControlState {
    #[default]
    Choosing,
    Viewing,
    ShowingInterval(Interval),
}
