use crate::build::tenscript::FaceAlias;
use crate::build::tinkerer::Stage::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::FaceRotation;
use crate::fabric::physics::Physics;
use crate::fabric::physics::presets::LIQUID;
use crate::user_interface::Action;

#[derive(Clone, PartialEq)]
enum Stage {
    Navigating,
    ReifyBrick,
    PendingFaceJoin,
    Connect,
    Approaching,
    Reverting,
    Settling,
}

#[derive(Clone, Debug)]
pub struct BrickOnFace {
    pub alias: FaceAlias,
    pub face_id: UniqueId,
    pub face_rotation: FaceRotation,
}

pub struct Tinkerer {
    stage: Stage,
    proposed_brick: Option<BrickOnFace>,
    proposed_connect: Option<(UniqueId, UniqueId)>,
    physics: Physics,
    history: Vec<Fabric>,
}

impl Default for Tinkerer {
    fn default() -> Self {
        Self {
            stage: Navigating,
            proposed_brick: None,
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
            Navigating => {
                fabric.iterate(&self.physics);
                Navigating
            }
            ReifyBrick => {
                if let Some(BrickOnFace { alias, face_id, face_rotation }) = &self.proposed_brick {
                    self.history.push(fabric.clone());
                    let (base_face_id, _) = fabric
                        .create_brick(alias, *face_rotation, 1.0, Some(*face_id));
                    self.proposed_connect = Some((base_face_id, *face_id));
                    PendingFaceJoin
                } else {
                    Navigating
                }
            }
            PendingFaceJoin => PendingFaceJoin,
            Connect => {
                if let Some((alpha, omega)) = self.proposed_connect {
                    fabric.join_faces(alpha, omega);
                    fabric.progress.start(1000);
                    self.proposed_brick = None;
                    action = Some(Action::SelectFace(None));
                }
                self.proposed_connect = None;
                Navigating
            }
            Reverting => {
                if let Some(fabric) = self.history.pop() {
                    let brick_on_face = self.proposed_brick.take();
                    action = Some(Action::RevertToFrozen { fabric, brick_on_face })
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
        };
        action
    }

    pub fn join_faces(&mut self, a: UniqueId, b: UniqueId) {
        self.proposed_connect = Some((a, b));
        self.connect();
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

    pub fn connect(&mut self) {
        self.stage = Connect;
    }

    pub fn revert(&mut self) {
        self.stage = Reverting;
    }
}
