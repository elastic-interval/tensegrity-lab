use std::collections::HashSet;

use cgmath::Point3;
use pest::iterators::{Pair, Pairs};

use crate::build::tenscript::{Rule, TenscriptError};
use crate::fabric::{Fabric, Link};
use crate::fabric::physics::SurfaceCharacter;

#[derive(Debug, Clone, Default)]
pub struct MuscleMovement {
    pub(crate) amplitude: f32,
    pub(crate) countdown: usize,
}

#[derive(Debug, Clone)]
pub struct Hanger {
    pub location: Point3<f32>,
    pub ring_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FinalPhase {
    pub surface_character: SurfaceCharacter,
    pub muscle_movement: Option<MuscleMovement>,
    pub pretense_factor: Option<f32>,
    pub hangers: Vec<Hanger>,
}

impl FinalPhase {
    pub fn new(surface_character: SurfaceCharacter) -> Self {
        Self {
            surface_character,
            muscle_movement: None,
            pretense_factor: None,
            hangers: Vec::new(),
        }
    }

    pub fn from_pair_option(pair: Option<Pair<Rule>>) -> Result<FinalPhase, TenscriptError> {
        let Some(pair) = pair else {
            return Ok(FinalPhase::default());
        };
        Self::parse_final(pair)
    }

    fn parse_final(pair: Pair<Rule>) -> Result<FinalPhase, TenscriptError> {
        match pair.as_rule() {
            Rule::final_state => {
                Self::parse_features(pair.into_inner())
            }
            _ => {
                unreachable!()
            }
        }
    }

    fn parse_features(pairs: Pairs<Rule>) -> Result<FinalPhase, TenscriptError> {
        let mut final_phase = FinalPhase::default();
        for feature_pair in pairs {
            match feature_pair.as_rule() {
                Rule::final_feature => {
                    for pretense_pair in feature_pair.into_inner() {
                        match pretense_pair.as_rule() {
                            Rule::hanger => {
                                let [x, y, z, ring] = pretense_pair.into_inner().next_chunk().unwrap();
                                let x = TenscriptError::parse_float(x.as_str(), "hanger")?;
                                let y = TenscriptError::parse_float(y.as_str(), "hanger")?;
                                let z = TenscriptError::parse_float(z.as_str(), "hanger")?;
                                let ring_index = TenscriptError::parse_usize_inside(ring, "ring")?;
                                let location = Point3::new(x, y, z);
                                final_phase.hangers.push(Hanger { location, ring_index })
                            }
                            Rule::surface => {
                                final_phase.surface_character = match pretense_pair.into_inner().next().unwrap().as_str() {
                                    ":frozen" => SurfaceCharacter::Frozen,
                                    ":bouncy" => SurfaceCharacter::Bouncy,
                                    ":absent" => SurfaceCharacter::Absent,
                                    _ => unreachable!("surface character")
                                }
                            }
                            Rule::muscle => {
                                let [amplitude, countdown] = pretense_pair.into_inner().next_chunk().unwrap();
                                let amplitude = TenscriptError::parse_float(amplitude.as_str(), "muscle amplitude")?;
                                let countdown = TenscriptError::parse_usize(countdown.as_str(), "muscle countdown")?;
                                final_phase.muscle_movement = Some(MuscleMovement { amplitude, countdown })
                            }
                            Rule::pretense_factor => {
                                let factor = TenscriptError::parse_float_inside(pretense_pair, "pretense-factor")?;
                                final_phase.pretense_factor = Some(factor)
                            }
                            _ => unreachable!()
                        }
                    }
                }
                _ => {
                    unreachable!()
                }
            }
        }
        Ok(final_phase)
    }

    pub fn check_muscles(&self, fabric: &mut Fabric) {
        if let Some(muscle_movement) = &self.muscle_movement {
            fabric.activate_muscles(muscle_movement);
        };
    }

    pub fn create_hangers(&self, fabric: &mut Fabric) {
        for &Hanger { location, ring_index } in &self.hangers {
            let joint = fabric.create_joint(location);
            fabric.joints[joint].location_fixed = true;
            let ring_joints: HashSet<usize> = fabric.rings[ring_index]
                .map(|id|
                    [fabric.interval(id).alpha_index, fabric.interval(id).omega_index]
                )
                .flatten()
                .iter().cloned()
                .collect();
            for ring_joint in ring_joints {
                let length = fabric.distance(joint, ring_joint);
                let link = Link { ideal: length, material_name: ":pull".into() };
                fabric.create_interval(joint, ring_joint, link);
            }
        }
    }
}