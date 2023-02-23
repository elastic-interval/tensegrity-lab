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
    holding_face: Option<UniqueId>,
    proposed_connect: Option<(UniqueId, UniqueId)>,
    physics: Physics,
    history: Vec<Frozen>,
}

impl Default for Tinkerer {
    fn default() -> Self {
        Self {
            stage: Start,
            proposed_brick: None,
            holding_face: None,
            proposed_connect: None,
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
                    self.proposed_connect = Some((base_face_id, *face_id));
                    PendingFaceJoin
                } else {
                    Navigating
                }
            }
            PendingFaceJoin => PendingFaceJoin,
            JoinFaces => {
                if let Some(pair) = self.proposed_connect {
                    fabric.join_faces(pair.0, pair.1);
                    fabric.progress.start(1000);
                    self.proposed_brick = None;
                    action = Some(Action::SelectFace(fabric.newest_face_id()));
                }
                self.proposed_connect = None;
                Navigating
            }
            Reverting => {
                if let Some(frozen) = self.history.pop() {
                    let brick_on_face = self.proposed_brick.take();
                    action = Some(Action::RevertToFrozen { frozen, brick_on_face })
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

    pub fn hold_face(&mut self, face_id: UniqueId) {
        if let Some(holding_face) = self.holding_face {
            if holding_face != face_id {
                self.proposed_connect = Some((holding_face, face_id));
            }
            self.holding_face = None;
        } else {
            self.holding_face = Some(face_id);
        }
    }

    pub fn propose_brick(&mut self, brick_on_face: BrickOnFace) {
        let proposal_was_active = self.proposed_brick.is_some();
        self.proposed_brick = Some(brick_on_face);
        self.stage = if proposal_was_active {
            Reverting
        } else {
            ReifyBrick
        };
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
