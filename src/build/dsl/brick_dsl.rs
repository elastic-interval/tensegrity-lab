use crate::build::dsl::brick::{
    BakedInterval, BakedJoint, Brick, BrickFace, FaceDef, Prototype, PullDef, PushDef,
};
use crate::build::dsl::{FaceAlias, Spin};
pub use crate::fabric::material::Material;
use cgmath::Point3;
use strum::Display;

pub fn material_name(material: Material) -> &'static str {
    match material {
        Material::Pull => "pull",
        Material::Push => "push",
        Material::Spring => "spring",
    }
}

pub trait MaterialIntervalExt {
    fn interval(self, alpha: usize, omega: usize, strain: f32) -> BakedInterval;
}

impl MaterialIntervalExt for Material {
    fn interval(self, alpha: usize, omega: usize, strain: f32) -> BakedInterval {
        interval(alpha, omega, strain, self)
    }
}

/// Brick type names
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum BrickName {
    SingleLeftBrick,
    SingleRightBrick,
    OmniBrick,
    TorqueBrick,
}

/// Context in which a brick face is being used
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum BrickRole {
    Seed,
    SeedFourDown,
    SeedFaceDown,
    OnSpinLeft,
    OnSpinRight,
}

impl BrickRole {
    pub const fn calls_it(self, face_name: FaceName) -> FaceAlias {
        FaceAlias {
            brick_role: self,
            face_name,
        }
    }

    pub fn is_seed(&self) -> bool {
        matches!(
            self,
            BrickRole::Seed | BrickRole::SeedFourDown | BrickRole::SeedFaceDown
        )
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
    Downwards,

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

pub struct ProtoBuilder {
    brick_name: BrickName,
    brick_roles: Vec<BrickRole>,
    joints: Vec<JointName>,
    pushes: Vec<PushDef>,
    pulls: Vec<PullDef>,
    faces: Vec<FaceDef>,
}

impl ProtoBuilder {
    pub fn new(brick_name: BrickName, brick_roles: Vec<BrickRole>) -> Self {
        Self {
            brick_name,
            brick_roles,
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

    pub fn pushes<const N: usize>(
        mut self,
        ideal: f32,
        pushes: [(JointName, JointName); N],
    ) -> Self {
        use crate::build::dsl::brick::Axis;
        self.pushes
            .extend(pushes.into_iter().map(|(alpha, omega)| PushDef {
                axis: Axis::X, // Axis is metadata, not used in our simplified builder
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

    pub fn face<const N: usize>(
        mut self,
        spin: Spin,
        joints: [JointName; 3],
        aliases: [FaceAlias; N],
    ) -> Self {
        // Check that all aliases only use roles declared in brick_roles
        for alias in &aliases {
            if !self.brick_roles.contains(&alias.brick_role) {
                panic!(
                    "Face alias uses role {:?} which is not declared in brick_roles {:?}",
                    alias.brick_role, self.brick_roles
                );
            }
        }

        self.faces.push(FaceDef {
            spin,
            joints,
            aliases: aliases.into(),
        });
        self
    }

    pub fn faces(mut self, faces: impl Into<Vec<FaceDef>>) -> Self {
        self.faces = faces.into();
        self
    }

    /// Build just the prototype
    pub fn build_proto(self) -> Prototype {
        Prototype {
            brick_name: self.brick_name,
            joints: self.joints,
            pushes: self.pushes,
            pulls: self.pulls,
            faces: self.faces,
        }
    }

    /// Continue to build the baked data
    pub fn baked(self) -> BakedBuilder {
        let proto = self.build_proto();
        BakedBuilder::new(proto)
    }
}

/// Start building a prototype
pub fn proto<const N: usize>(brick_name: BrickName, brick_roles: [BrickRole; N]) -> ProtoBuilder {
    ProtoBuilder::new(brick_name, brick_roles.into())
}

/// Builder for Baked brick data
pub struct BakedBuilder {
    proto: Prototype,
    joints: Vec<BakedJoint>,
    intervals: Vec<BakedInterval>,
    faces: Vec<BrickFace>,
}

impl BakedBuilder {
    pub fn new(proto: Prototype) -> Self {
        Self {
            proto,
            joints: vec![],
            intervals: vec![],
            faces: vec![],
        }
    }

    pub fn joints<const N: usize>(mut self, joints: [(f32, f32, f32); N]) -> Self {
        self.joints = joints.into_iter().map(|(x, y, z)| joint(x, y, z)).collect();
        self
    }

    pub fn intervals(mut self, intervals: impl Into<Vec<BakedInterval>>) -> Self {
        self.intervals = intervals.into();
        self
    }

    pub fn pushes<const N: usize>(mut self, pushes: [(usize, usize, f32); N]) -> Self {
        self.intervals.extend(
            pushes
                .into_iter()
                .map(|(alpha, omega, strain)| interval(alpha, omega, strain, Material::Push)),
        );
        self
    }

    pub fn pulls<const N: usize>(mut self, pulls: [(usize, usize, f32); N]) -> Self {
        self.intervals.extend(
            pulls
                .into_iter()
                .map(|(alpha, omega, strain)| interval(alpha, omega, strain, Material::Pull)),
        );
        self
    }

    pub fn faces(mut self, faces: impl Into<Vec<BrickFace>>) -> Self {
        self.faces = faces.into();
        self
    }

    pub fn build(self) -> Brick {
        Brick::new(self.proto, self.joints, self.intervals)
    }
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
    }
}

/// Create a baked joint at (x, y, z)
pub fn joint(x: f32, y: f32, z: f32) -> BakedJoint {
    BakedJoint {
        location: Point3::new(x, y, z),
    }
}

/// Create a baked interval with strain
pub fn interval(alpha: usize, omega: usize, strain: f32, material: Material) -> BakedInterval {
    BakedInterval {
        alpha_index: alpha,
        omega_index: omega,
        strain,
        material_name: material_name(material).to_string(),
    }
}

/// Create a baked face with joint indices
pub fn baked_face(spin: Spin, joints: [usize; 3], aliases: impl Into<Vec<FaceAlias>>) -> BrickFace {
    BrickFace {
        spin,
        joints,
        aliases: aliases.into(),
    }
}
