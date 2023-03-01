use cgmath::InnerSpace;
use crate::build::tenscript::FaceAlias;
use crate::build::tinkerer::Stage::{*};
use crate::camera::Pick;
use crate::crucible::TinkererAction;
use crate::crucible::TinkererAction::{*};
use crate::fabric::{Fabric, UniqueId};
use crate::fabric::face::FaceRotation;
use crate::fabric::physics::Physics;
use crate::fabric::physics::presets::LIQUID;
use crate::user_interface::Action;

#[derive(Clone, Debug)]
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

pub struct ConnectBrick {
    faces: [UniqueId; 2],
    face_to_select: Option<UniqueId>,
}

#[derive(Clone, Debug)]
pub struct Frozen {
    pub fabric: Fabric,
    pub face_id: Option<UniqueId>,
}

pub struct Tinkerer {
    stage: Stage,
    proposed_brick: Option<BrickOnFace>,
    proposed_connect: Option<ConnectBrick>,
    physics: Physics,
    history: Vec<Frozen>,
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
                    let base_normal = fabric.face(*face_id).normal(fabric);
                    self.history.push(Frozen { fabric: fabric.clone(), face_id: Some(face_id.clone()) });
                    let (base_face_id, faces) = fabric
                        .create_brick(alias, *face_rotation, 1.0, Some(*face_id));
                    let face_to_select = faces
                        .iter()
                        .map(|&id| (id, fabric.face(id).normal(fabric).dot(base_normal)))
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                        .map(|(face_id, _)| face_id);
                    self.proposed_connect = Some(ConnectBrick { faces: [base_face_id, *face_id], face_to_select });
                    PendingFaceJoin
                } else {
                    Navigating
                }
            }
            PendingFaceJoin => PendingFaceJoin,
            Connect => {
                if let Some(ConnectBrick { faces: [alpha, omega], face_to_select }) = self.proposed_connect {
                    fabric.join_faces(alpha, omega);
                    fabric.progress.start(1000);
                    self.proposed_brick = None;
                    action = Some(Action::SelectFace(face_to_select.map(Pick::just)));
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
        };
        action
    }

    pub fn action(&mut self, tinkerer_action: TinkererAction) {
        match tinkerer_action {
            Propose(brick_on_face) => {
                let proposal_was_active = self.proposed_brick.is_some();
                self.proposed_brick = Some(brick_on_face);
                self.stage = if proposal_was_active {
                    Reverting
                } else {
                    ReifyBrick
                };
            }
            Clear => {
                self.proposed_brick = None;
                self.stage = Reverting;
            }
            Commit => {
                self.stage = Connect;
            }
            JoinIfPair(face_set) => {
                if let Ok([a, b]) = face_set.into_iter().next_chunk() {
                    self.proposed_connect = Some(ConnectBrick { faces: [a, b], face_to_select: None });
                }
                self.stage = Connect;
            }
            InitiateRevert => {
                self.stage = Reverting;
            }
        }
    }

    pub fn is_brick_proposed(&self) -> bool {
        self.proposed_brick.is_some()
    }

    pub fn is_history_available(&self) -> bool {
        !self.history.is_empty()
    }
}
