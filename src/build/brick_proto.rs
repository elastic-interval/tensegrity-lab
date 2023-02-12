use cgmath::num_traits::abs;
use cgmath::Point3;

use crate::build::brick::{Baked, BrickFace};
use crate::build::tenscript::FaceAlias;
use crate::build::tenscript::Spin::{Left, Right};
use crate::fabric::Fabric;
use crate::fabric::interval::{Interval, Role};
use crate::fabric::joint::Joint;

impl Baked {
    pub const TARGET_FACE_STRAIN: f32 = 0.1;

    pub fn into_tenscript(self) -> String {
        format!("(baked
    {joints}
    {intervals}
    {faces})
        ",
                joints = self.joints
                    .into_iter()
                    .map(|Point3 { x, y, z }|
                        format!("(joint {x:.4} {y:.4} {z:.4})"))
                    .collect::<Vec<_>>()
                    .join("\n    "),
                intervals = self.intervals
                    .into_iter()
                    .map(|(alpha, omega, role, strain)|
                        format!("({} {alpha} {omega} {strain:.4})", match role {
                            Role::Push => "push",
                            Role::Pull => "pull",
                        }))
                    .collect::<Vec<_>>()
                    .join("\n    "),
                faces = self.faces
                    .into_iter()
                    .map(|BrickFace { joints: [a, b, c], aliases, spin }|
                        format!(
                            "({spin} {a} {b} {c} {aliases})",
                            spin = match spin {
                                Left => "left",
                                Right => "right",
                            },
                            aliases = aliases
                                .into_iter()
                                .map(|FaceAlias(name)|
                                    format!("(alias {})", name.join(" ")))
                                .collect::<Vec<_>>()
                                .join(" "))
                    )
                    .collect::<Vec<_>>()
                    .join("\n    ")
        )
    }
}

impl TryFrom<Fabric> for Baked {
    type Error = String;

    fn try_from(fabric: Fabric) -> Result<Self, String> {
        let joint_incident = fabric.joint_incident();
        let target_face_strain = Baked::TARGET_FACE_STRAIN;
        for face in fabric.faces.values() {
            let strain = face.strain(&fabric);
            if abs(strain - target_face_strain) > 0.0001 {
                return Err(format!("Face interval strain too far from {target_face_strain} {strain:.5}"));
            }
        }
        Ok(Self {
            joints: fabric.joints
                .iter()
                .map(|Joint { location, .. }| *location)
                .collect(),
            intervals: fabric.interval_values()
                .filter_map(|Interval { alpha_index, omega_index, material, strain, .. }|
                    joint_incident[*alpha_index].push
                        .map(|_| (*alpha_index, *omega_index, fabric.materials[*material].role, *strain)))
                .collect(),
            faces: fabric.faces
                .values()
                .map(|face| BrickFace {
                    joints: face.radial_joints(&fabric),
                    aliases: face.aliases.clone(),
                    spin: face.spin,
                })
                .collect(),
        })
    }
}
