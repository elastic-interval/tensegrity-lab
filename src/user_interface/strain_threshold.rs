use crate::user_interface::ControlMessage;

#[derive(Debug, Clone)]
pub enum StrainThresholdMessage {
    SetStrainLimits((f32, f32)),
    NuanceChanged(f32),
    Calibrate,
}

impl From<StrainThresholdMessage> for ControlMessage {
    fn from(value: StrainThresholdMessage) -> Self {
        ControlMessage::StrainThreshold(value)
    }
}

#[derive(Clone, Debug)]
pub struct StrainThreshold {
    pub nuance: f32,
    pub strain_limits: (f32, f32),
}

impl StrainThreshold {
    pub fn strain_threshold(&self) -> f32 {
        let (min_strain, max_strain) = self.strain_limits;
        min_strain * (1.0 - self.nuance) + max_strain * self.nuance
    }
}
