use cgmath::Point3;
use strum::Display;
use crate::build::dsl::brick::{
    Baked, BakedJoint, BakedInterval, BrickDefinition, BrickFace,
    FaceDef, Prototype, PullDef, PushDef
};
use crate::build::dsl::{FaceAlias, FaceTag, Spin};

pub use crate::fabric::material::Material;

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
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum BrickName {
    SingleBrick,
    Omni,
    Torque,
    TorqueRight,
    TorqueLeft,
    Equals,
}

/// Context in which a brick face is being used
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum FaceContext {
    /// No parent face - used as the initial brick (seed)
    SeedA,
    /// Alternative initial orientation
    SeedB,
    /// Placed on top of a parent face with left spin
    OnSpinLeft,
    /// Placed on top of a parent face with right spin
    OnSpinRight,
}

impl FaceContext {
    /// Create a face alias entry with the given faces
    pub const fn calls_it(self, faces: &[FaceName]) -> (Self, &[FaceName]) {
        (self, faces)
    }
}

/// Unified joint names for all bricks
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum JointName {
    // Single brick joints
    AlphaX, AlphaY, AlphaZ,
    OmegaX, OmegaY, OmegaZ,

    // Omni brick joints
    BotAlphaX, BotAlphaY, BotAlphaZ,
    BotOmegaX, BotOmegaY, BotOmegaZ,
    TopAlphaX, TopAlphaY, TopAlphaZ,
    TopOmegaX, TopOmegaY, TopOmegaZ,

    // Torque brick joints
    LeftFront, LeftBack,
    MiddleFront, MiddleBack,
    RightFront, RightBack,
    FrontLeftBottom, FrontLeftTop,
    FrontRightBottom, FrontRightTop,
    BackLeftBottom, BackLeftTop,
    BackRightBottom, BackRightTop,
    TopLeft, TopRight,
    BottomLeft, BottomRight,
}

/// Unified face names for all bricks
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum SingleFace {
    Top,
    Bot,
    Base,
    NextBase,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum OmniFaceDown {
    Top,
    TopX, TopY, TopZ,
    BotX, BotY, BotZ,
    Bot,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum FourDown {
    TopRight, TopLeft,
    FrontRight, FrontLeft,
    BackRight, BackLeft,
    BottomRight, BottomLeft,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum TorqueFaceOnTop {
    BaseBack, BaseSide, BaseFront,
    FarBack, FarSide, FarFront, FarBase, FarBrother,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum TorqueFaceFourDown {
    LeftFrontBottom, LeftBackBottom, RightBackBottom, RightFrontBottom,
    LeftBackTop, LeftFrontTop, RightFrontTop, RightBackTop,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum TorqueTiedFace {
    OtherA, OtherB, Brother,
    FarOtherA, FarOtherB,
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash)]
pub enum FaceName {
    Single(SingleFace),
    OmiFaceDown(OmniFaceDown),
    OmniFaceUp(OmniFaceDown),
    Four(FourDown),
    TorqueOnTop(TorqueFaceOnTop),
    TorqueFourDown(TorqueFaceFourDown),
    TorqueTied(TorqueTiedFace),
}

/// Create a FaceAlias from a list of FaceTags
pub fn alias(tags: &[FaceTag]) -> FaceAlias {
    FaceAlias(tags.to_vec())
}

/// Trait for converting face context and names into a face alias
pub trait IntoFaceAlias {
    fn into_face_alias(self, brick_name: BrickName) -> FaceAlias;
}

impl IntoFaceAlias for (FaceContext, &[FaceName]) {
    fn into_face_alias(self, brick_name: BrickName) -> FaceAlias {
        face_alias_for(brick_name, self.0, self.1)
    }
}

impl<const M: usize> IntoFaceAlias for (FaceContext, &[FaceName; M]) {
    fn into_face_alias(self, brick_name: BrickName) -> FaceAlias {
        face_alias_for(brick_name, self.0, self.1)
    }
}

impl<const M: usize> IntoFaceAlias for (FaceContext, [FaceName; M]) {
    fn into_face_alias(self, brick_name: BrickName) -> FaceAlias {
        face_alias_for(brick_name, self.0, &self.1)
    }
}

/// Builder for Prototype - provides a fluent API for constructing brick prototypes
pub struct ProtoBuilder {
    brick_name: BrickName,
    alias: FaceAlias,
    joints: Vec<JointName>,
    pushes: Vec<PushDef>,
    pulls: Vec<PullDef>,
    faces: Vec<FaceDef>,
}

impl ProtoBuilder {
    pub fn new(brick_name: BrickName) -> Self {
        Self {
            brick_name,
            alias: alias(&[FaceTag::Brick(brick_name)]),
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

    pub fn pushes<const N: usize>(mut self, ideal: f32, pushes: [(JointName, JointName); N]) -> Self {
        use crate::build::dsl::brick::Axis;
        self.pushes.extend(pushes.into_iter().map(|(alpha, omega)| PushDef {
            axis: Axis::X, // Axis is metadata, not used in our simplified builder
            alpha,
            omega,
            ideal,
        }));
        self
    }

    pub fn pulls<const N: usize>(mut self, ideal: f32, pulls: [(JointName, JointName); N]) -> Self {
        self.pulls.extend(pulls.into_iter().map(|(alpha, omega)| PullDef {
            alpha,
            omega,
            ideal,
            material: "pull".to_string(),
        }));
        self
    }

    /// Add a face with multiple context/face-name pairs - brick name is automatically added to aliases
    pub fn face<T: IntoFaceAlias>(mut self, spin: Spin, joints: [JointName; 3],
                aliases: impl IntoIterator<Item = T>) -> Self {
        let face_aliases = aliases.into_iter()
            .map(|entry| entry.into_face_alias(self.brick_name))
            .collect();
        self.faces.push(FaceDef {
            spin,
            joints,
            aliases: face_aliases,
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
            alias: self.alias,
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
pub fn proto(brick_name: BrickName) -> ProtoBuilder {
    ProtoBuilder::new(brick_name)
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
        self.intervals.extend(pushes.into_iter().map(|(alpha, omega, strain)| {
            interval(alpha, omega, strain, Material::Push)
        }));
        self
    }

    pub fn pulls<const N: usize>(mut self, pulls: [(usize, usize, f32); N]) -> Self {
        self.intervals.extend(pulls.into_iter().map(|(alpha, omega, strain)| {
            interval(alpha, omega, strain, Material::Pull)
        }));
        self
    }

    pub fn faces(mut self, faces: impl Into<Vec<BrickFace>>) -> Self {
        self.faces = faces.into();
        self
    }

    pub fn build(self) -> BrickDefinition {
        let baked = Baked {
            joints: self.joints,
            intervals: self.intervals,
            faces: self.faces,
        };
        BrickDefinition {
            proto: self.proto,
            baked: Some(baked),
        }
    }
}


/// Derive baked faces from prototype faces by mapping joint names to indices
pub fn derive_baked_faces(proto: &Prototype) -> Vec<BrickFace> {
    // Build a map from joint names to indices
    let mut joint_map = std::collections::HashMap::new();

    // Add explicit joints first (they get indices 0, 1, 2, ...)
    for (idx, joint_name) in proto.joints.iter().enumerate() {
        joint_map.insert(*joint_name, idx);
    }

    // Add joints from pushes (starting after explicit joints)
    let offset = proto.joints.len();
    for (idx, push) in proto.pushes.iter().enumerate() {
        let alpha_idx = offset + idx * 2;
        let omega_idx = offset + idx * 2 + 1;
        joint_map.insert(push.alpha, alpha_idx);
        joint_map.insert(push.omega, omega_idx);
    }

    // Convert proto faces to baked faces
    proto.faces.iter().map(|face_def| {
        let joints = [
            *joint_map.get(&face_def.joints[0]).expect("Joint name not found"),
            *joint_map.get(&face_def.joints[1]).expect("Joint name not found"),
            *joint_map.get(&face_def.joints[2]).expect("Joint name not found"),
        ];
        BrickFace {
            spin: face_def.spin,
            joints,
            aliases: face_def.aliases.clone(),
        }
    }).collect()
}

/// Helper to build face aliases for a specific context
/// Combines context, face names, and brick type into a complete alias
pub fn face_alias_for(brick: BrickName, context: FaceContext, faces: &[FaceName]) -> FaceAlias {
    let mut tags = vec![FaceTag::Context(context)];
    tags.extend(faces.iter().map(|&f| FaceTag::Face(f)));
    tags.push(FaceTag::Brick(brick));
    FaceAlias(tags)
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
    BakedJoint { location: Point3::new(x, y, z) }
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
    BrickFace { spin, joints, aliases: aliases.into() }
}
