use crate::build::dsl::brick::{Axis, BrickPrototype, FaceDef, PullDef, PushDef};
use crate::build::dsl::brick_dsl::FaceName::Downwards;
use crate::build::dsl::{FaceAlias, ScaleMode, Spin};
pub use crate::fabric::material::Material;
use cgmath::Vector3;
use strum::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct OmniParams {
    pub push_lengths: Vector3<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SingleParams {
    pub push_lengths: Vector3<f32>,
    pub pull_length: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TorqueParams {
    pub push_lengths: Vector3<f32>,
    pub pull_length: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BrickParams {
    Omni(OmniParams),
    SingleLeft(SingleParams),
    SingleRight(SingleParams),
    Torque(TorqueParams),
}

pub fn material_name(material: Material) -> &'static str {
    match material {
        Material::Pull => "pull",
        Material::Push => "push",
        Material::Spring => "spring",
    }
}

/// Brick type names
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash, clap::ValueEnum, strum::EnumIter)]
pub enum BrickName {
    SingleTwistLeft,
    SingleTwistRight,
    OmniSymmetrical,
    OmniTetrahedral,
    TorqueSymmetrical,
}

impl BrickName {
    pub fn face_scaling(&self) -> ScaleMode {
        match self {
            BrickName::OmniTetrahedral => ScaleMode::Tetrahedral,
            _ => ScaleMode::None,
        }
    }
}

/// Context in which a brick face is being used
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum BrickRole {
    Seed(usize), // how many faces down
    OnSpinLeft,
    OnSpinRight,
}

impl BrickRole {
    pub fn calls_it(self, face_name: FaceName) -> FaceAlias {
        FaceAlias {
            brick_role: self,
            face_name,
        }
    }

    pub fn downwards(self) -> FaceAlias {
        let downwards_count = match self {
            BrickRole::Seed(downward_count) => downward_count,
            _ => panic!("Downwards requires a seed variant"),
        };
        FaceAlias {
            brick_role: self,
            face_name: Downwards(downwards_count),
        }
    }

    /// Mirror this role (swap OnSpinLeft ↔ OnSpinRight, Seed stays same)
    pub fn mirror(self) -> BrickRole {
        match self {
            BrickRole::OnSpinLeft => BrickRole::OnSpinRight,
            BrickRole::OnSpinRight => BrickRole::OnSpinLeft,
            BrickRole::Seed(n) => BrickRole::Seed(n),
        }
    }
}

/// Mark names used in fabric definitions
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum MarkName {
    End,
    HaloEnd,
    RingA,
    RingB,
    RingX,
    RingY,
    RingZ,
    Legs,
    Chest1,
    Chest2,
    Hands,
    Loose,
}

/// Unified joint names for all bricks
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum JointName {
    // Single brick joints
    AlphaX,
    AlphaY,
    AlphaZ,
    OmegaX,
    OmegaY,
    OmegaZ,

    // Omni brick joints
    BotAlphaX,
    BotAlphaY,
    BotAlphaZ,
    BotOmegaX,
    BotOmegaY,
    BotOmegaZ,
    TopAlphaX,
    TopAlphaY,
    TopAlphaZ,
    TopOmegaX,
    TopOmegaY,
    TopOmegaZ,

    // Torque brick joints
    LeftFront,
    LeftBack,
    MiddleFront,
    MiddleBack,
    RightFront,
    RightBack,
    FrontLeftBottom,
    FrontLeftTop,
    FrontRightBottom,
    FrontRightTop,
    BackLeftBottom,
    BackLeftTop,
    BackRightBottom,
    BackRightTop,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Simple brick orientation types
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum BrickOrientation {
    SingleLeft,
    SingleRight,
    OmniFaceDown,
    OmniFourDown,
    TorqueAttach,
    TorqueFourDown,
}

/// Unified face names for all brick orientations
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum FaceName {
    // Building faces
    Attach(Spin),
    AttachNext,

    // Orientation faces
    Downwards(usize),

    // Single brick faces
    SingleTop,
    SingleBot,

    // Omni face-down faces
    OmniTop,
    OmniTopX,
    OmniTopY,
    OmniTopZ,
    OmniBotX,
    OmniBotY,
    OmniBotZ,
    OmniBot,

    // Four-down faces
    RightFrontTop,
    LeftFrontTop,
    RightFrontBottom,
    LeftFrontBottom,
    RightBackTop,
    LeftBackTop,
    RightBackBottom,
    LeftBackBottom,

    // Torque attach faces (first is Attach())
    NearA,
    NearB,
    NearC,
    FarA,
    FarB,
    FarC,
    Far,
}

impl FaceName {
    /// Mirror this face name (swap Spin in Attach, others stay same)
    pub fn mirror(self) -> FaceName {
        match self {
            FaceName::Attach(spin) => FaceName::Attach(spin.mirror()),
            _ => self,
        }
    }
}

pub struct ProtoBuilder {
    brick_name: BrickName,
    brick_roles: Vec<BrickRole>,
    scale_modes: Vec<ScaleMode>,
    joints: Vec<JointName>,
    pushes: Vec<PushDef>,
    pulls: Vec<PullDef>,
    faces: Vec<FaceDef>,
}

impl ProtoBuilder {
    pub fn new(
        brick_name: BrickName,
        brick_roles: Vec<BrickRole>,
        scale_modes: Vec<ScaleMode>,
    ) -> Self {
        Self {
            brick_name,
            brick_roles,
            scale_modes,
            joints: vec![],
            pushes: vec![],
            pulls: vec![],
            faces: vec![],
        }
    }

    /// Add explicit joint declarations (joints that aren't created by pushes)
    pub fn joints<const N: usize>(mut self, joints: [JointName; N]) -> Self {
        self.joints = joints.into_iter().collect();
        self
    }

    pub fn pushes_x<const N: usize>(
        mut self,
        ideal: f32,
        pushes: [(JointName, JointName); N],
    ) -> Self {
        self.pushes
            .extend(pushes.into_iter().map(|(alpha, omega)| PushDef {
                axis: Axis::X,
                alpha,
                omega,
                ideal,
            }));
        self
    }

    pub fn pushes_y<const N: usize>(
        mut self,
        ideal: f32,
        pushes: [(JointName, JointName); N],
    ) -> Self {
        self.pushes
            .extend(pushes.into_iter().map(|(alpha, omega)| PushDef {
                axis: Axis::Y,
                alpha,
                omega,
                ideal,
            }));
        self
    }

    pub fn pushes_z<const N: usize>(
        mut self,
        ideal: f32,
        pushes: [(JointName, JointName); N],
    ) -> Self {
        self.pushes
            .extend(pushes.into_iter().map(|(alpha, omega)| PushDef {
                axis: Axis::Z,
                alpha,
                omega,
                ideal,
            }));
        self
    }

    pub fn pulls<const N: usize>(mut self, ideal: f32, pulls: [(JointName, JointName); N]) -> Self {
        self.pulls
            .extend(pulls.into_iter().map(|(alpha, omega)| PullDef {
                alpha,
                omega,
                ideal,
                material: "pull".to_string(),
            }));
        self
    }

    pub fn face<const N: usize, const M: usize>(
        mut self,
        spin: Spin,
        joints: [JointName; 3],
        aliases: [FaceAlias; N],
        scale_overrides: [(ScaleMode, f32); M],
    ) -> Self {
        for alias in &aliases {
            if !self.brick_roles.contains(&alias.brick_role) {
                panic!(
                    "Face alias uses role {:?} which is not declared in brick_roles {:?}",
                    alias.brick_role, self.brick_roles
                );
            }
        }
        for (mode, _) in &scale_overrides {
            if !self.scale_modes.contains(mode) {
                panic!(
                    "Face uses scaling {:?} which is not declared in scale_modes {:?}",
                    mode, self.scale_modes
                );
            }
        }
        self.faces.push(FaceDef {
            spin,
            joints,
            aliases: aliases.into(),
            scale_overrides: scale_overrides.into(),
        });
        self
    }

    pub fn faces(mut self, faces: impl Into<Vec<FaceDef>>) -> Self {
        self.faces = faces.into();
        self
    }

    pub fn build(self) -> BrickPrototype {
        BrickPrototype {
            brick_name: self.brick_name,
            brick_roles: self.brick_roles,
            scale_modes: self.scale_modes,
            joints: self.joints,
            pushes: self.pushes,
            pulls: self.pulls,
            faces: self.faces,
        }
    }
}

/// Start building a prototype with no scaling
pub fn proto<const N: usize>(brick_name: BrickName, brick_roles: [BrickRole; N]) -> ProtoBuilder {
    ProtoBuilder::new(brick_name, brick_roles.into(), vec![])
}

/// Start building a prototype with face scalings
pub fn proto_scaled<const N: usize, const M: usize>(
    brick_name: BrickName,
    brick_roles: [BrickRole; N],
    scale_modes: [ScaleMode; M],
) -> ProtoBuilder {
    ProtoBuilder::new(brick_name, brick_roles.into(), scale_modes.into())
}

/// Create a pull interval (cable) definition
pub fn pull(alpha: JointName, omega: JointName, ideal: f32, material: Material) -> PullDef {
    PullDef {
        alpha,
        omega,
        ideal,
        material: material_name(material).to_string(),
    }
}

/// Create a face definition with spin and joint names
pub fn face(spin: Spin, joints: [JointName; 3], aliases: impl Into<Vec<FaceAlias>>) -> FaceDef {
    FaceDef {
        spin,
        joints,
        aliases: aliases.into(),
        scale_overrides: vec![],
    }
}
