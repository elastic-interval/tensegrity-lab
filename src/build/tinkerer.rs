use crate::build::tenscript::FaceAlias;
use crate::build::tinkerer::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::FaceRotation;
use crate::fabric::physics::Physics;
use crate::fabric::physics::presets::LIQUID;
use crate::user_interface::Action;

#[derive(Clone, PartialEq)]
enum Stage {
    Start,
    Navigating,
    AddingBrick { alias: FaceAlias, face_id: UniqueId },
    Approaching,
    Settling,
    Finished,
}

pub struct Tinkerer {
    stage: Stage,
    physics: Physics,
}

impl Tinkerer {
    pub fn new() -> Self {
        Self {
            stage: Start,
            physics: LIQUID,
        }
    }

    pub fn iterate(&mut self, fabric: &mut Fabric) -> Option<Action> {
        let mut action = None;
        self.stage = match &mut self.stage {
            Start => Navigating,
            Navigating => {
                fabric.iterate(&self.physics);
                Navigating
            }
            AddingBrick { alias, face_id } => {
                let faces = fabric.attach_brick(alias, FaceRotation::Zero, 1.0, Some(*face_id));
                action = faces.first().map(|&face_id| Action::SelectFace(face_id));
                fabric.progress.start(1000);
                Approaching
            }
            Approaching => {
                fabric.iterate(&self.physics);
                if fabric.progress.is_busy() {
                    Approaching
                } else {
                    fabric.progress.start(1000);
                    Settling
                }

            }
            Settling => {
                fabric.iterate(&self.physics);
                if fabric.progress.is_busy() {
                    Settling
                } else {
                    Navigating
                }
            }
            Finished => Finished
        };
        action
    }

    pub fn add_brick(&mut self, alias: FaceAlias, face_id: UniqueId) {
        self.stage = AddingBrick { alias, face_id };
    }

    pub fn is_done(&self) -> bool {
        self.stage == Finished
    }
}
