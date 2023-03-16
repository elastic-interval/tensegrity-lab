use crate::fabric::Fabric;
use crate::fabric::interval::Span::Muscle;

pub trait FabricHook {
    fn prepare_fabric(&mut self, fabric: &mut Fabric);
}

pub enum Hook {
    HangerRotation
}

pub fn fabric_hook(hook: Hook) -> Box<dyn FabricHook> {
    Box::new(match hook {
        Hook::HangerRotation => HangerRotationHook::default()
    })
}

#[derive(Debug, Clone, Default)]
struct HangerRotationHook;

const ROTATIONS: [f32; 12] = [
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0,
    2.0, 1.0, 0.0, 5.0, 4.0, 3.0,
];

impl FabricHook for HangerRotationHook {
    fn prepare_fabric(&mut self, fabric: &mut Fabric) {
        for (index, interval) in fabric.intervals
            .values_mut()
            .filter(|interval| fabric.joints[interval.alpha_index].location_fixed)
            .enumerate()
        {
            let ideal = interval.ideal();
            interval.span = Muscle { average: ideal, amplitude: 0.3, angle: ROTATIONS[index] / 6.0 }
        }
    }
}
