use cgmath::Point3;
use itertools::Itertools;

use crate::fabric::interval::Interval;
use crate::fabric::joint::Joint;
use crate::fabric::material::interval_material;
use crate::fabric::Fabric;

impl Fabric {
    pub fn csv(&self) -> String {
        let joints = self
            .joints
            .iter()
            .filter(|Joint { fixed, .. }| !fixed)
            .enumerate()
            .map(|(index, Joint { location, .. })| {
                let Point3 { x, y, z } = location * self.scale;
                // ["index", "x", "y", "z"]
                format!("{};{x:.4};{y:.4};{z:.4}", index + 1)
            })
            .join("\n");
        let intervals = self
            .interval_values()
            .filter(|Interval { material, .. }| !interval_material(*material).support)
            .map(|interval| {
                let ideal = interval.length(&self.joints) * self.scale;
                let role = interval_material(interval.material).role;
                let Interval {
                    alpha_index,
                    omega_index,
                    ..
                } = interval;
                // ["joints", "role", "ideal length"]
                format!(
                    "=\"{},{}\";{role:?};{ideal:.4}",
                    alpha_index + 1,
                    omega_index + 1
                )
            })
            .join("\n");
        let submerged = self
            .joints
            .iter()
            .filter(|Joint { fixed, .. }| !fixed)
            .enumerate()
            .filter(
                |(
                    _,
                    Joint {
                        location: Point3 { y, .. },
                        ..
                    },
                )| *y <= 0.0,
            )
            .map(|(index, _)| index + 1)
            .join(",");
        let height = self
            .joints
            .iter()
            .fold(0.0f32, |h, joint| h.max(joint.location.y))
            * self.scale;
        let scale = self.scale;
        format!("Height: {height:.1} Scale: {scale:.1}\n\nIndex;X;Y;Z\n{joints}\n\nJoints;Role;Length\n{intervals}\n\nSubmerged\n=\"{submerged}\"")
    }
}
