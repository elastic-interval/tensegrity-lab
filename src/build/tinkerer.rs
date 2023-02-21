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
    Reverting,
    Settling,
    Finished,
}

#[derive(Clone, Debug)]
pub struct Frozen {
    pub fabric: Fabric,
    pub selected_face: UniqueId,
}

pub struct Tinkerer {
    stage: Stage,
    physics: Physics,
    history: Vec<Frozen>,
}

impl Default for Tinkerer {
    fn default() -> Self {
        Self {
            stage: Start,
            physics: LIQUID,
            history: Vec::default(),
        }
    }
}

impl Tinkerer {
    pub fn iterate(&mut self, fabric: &mut Fabric) -> Option<Action> {
        let mut action = None;
        self.stage = match &mut self.stage {
            Start => Navigating,
            Navigating => {
                fabric.iterate(&self.physics);
                Navigating
            }
            AddingBrick { alias, face_id } => {
                self.history.push(Frozen { fabric: fabric.clone(), selected_face: face_id.clone() });
                fabric.attach_brick(alias, FaceRotation::Zero, 1.0, Some(*face_id));
                action = Some(Action::SelectFace(fabric.newest_face_id()));
                fabric.progress.start(1000);
                Approaching
            }
            Reverting => {
                if let Some(frozen)  = self.history.pop() {
                    action = Some(Action::RevertToFrozen(frozen))
                };
                Navigating
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

    pub fn revert(&mut self) {
        self.stage = Reverting;
    }

    pub fn is_done(&self) -> bool {
        self.stage == Finished
    }
}
