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
    ReifyBrick,
    PendingFaceJoin,
    JoinFaces,
    Approaching,
    Reverting,
    Settling,
    Finished,
}

#[derive(Clone, Debug)]
pub struct BrickOnFace {
    pub alias: FaceAlias,
    pub face_id: UniqueId,
    pub face_rotation: FaceRotation,
}

#[derive(Clone, Debug)]
pub struct Frozen {
    pub fabric: Fabric,
    pub selected_face: UniqueId,
}

pub struct Tinkerer {
    stage: Stage,
    proposed_brick: Option<BrickOnFace>,
    pending_join: Option<(UniqueId, UniqueId)>,
    physics: Physics,
    history: Vec<Frozen>,
}

impl Default for Tinkerer {
    fn default() -> Self {
        Self {
            stage: Start,
            proposed_brick: None,
            pending_join: None,
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
            ReifyBrick => {
                if let Some(BrickOnFace { alias, face_id, face_rotation }) = &self.proposed_brick {
                    self.history.push(Frozen { fabric: fabric.clone(), selected_face: *face_id });
                    let (base_face_id, _) = fabric
                        .create_brick(alias, *face_rotation, 1.0, Some(*face_id));
                    self.pending_join = Some((base_face_id, *face_id));
                    PendingFaceJoin
                } else {
                    Navigating
                }
            }
            PendingFaceJoin => PendingFaceJoin,
            JoinFaces => {
                if let Some(pair) = self.pending_join {
                    fabric.join_faces(pair.0, pair.1);
                    fabric.progress.start(1000);
                    self.proposed_brick = None;
                    action = Some(Action::SelectFace(fabric.newest_face_id()));
                }
                self.pending_join = None;
                Navigating
            }
            Reverting => {
                self.proposed_brick = None;
                if let Some(frozen) = self.history.pop() {
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

    pub fn propose_brick(&mut self, brick_on_face: BrickOnFace) {
        self.stage = if self.proposed_brick.is_some() {
            Reverting
        } else {
            ReifyBrick
        };
        self.proposed_brick = Some(brick_on_face);
    }

    pub fn join_faces(&mut self) {
        self.stage = JoinFaces;
    }

    pub fn revert(&mut self) {
        self.stage = Reverting;
    }

    pub fn is_done(&self) -> bool {
        self.stage == Finished
    }
}
