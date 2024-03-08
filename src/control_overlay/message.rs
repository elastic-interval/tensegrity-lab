use crate::fabric::interval::Interval;

#[derive(Clone, Debug)]
pub enum Message {
    Init,
    PickedInterval(Interval),
}
