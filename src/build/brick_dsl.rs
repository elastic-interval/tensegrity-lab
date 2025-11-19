use cgmath::Point3;
use crate::build::tenscript::brick::{
    Baked, BakedJoint, BakedInterval, BrickDefinition, BrickFace, 
    FaceDef, Prototype, PullDef, PushDef
};
use crate::build::tenscript::{FaceAlias, Spin};

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
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BrickName {
    Single,
    Omni,
    Torque,
    TorqueRight,
    TorqueLeft,
    Equals,
}

impl BrickName {
    pub fn name(self) -> &'static str {
        match self {
            BrickName::Single => "Single",
            BrickName::Omni => "Omni",
            BrickName::Torque => "Torque",
            BrickName::TorqueRight => "TorqueRight",
            BrickName::TorqueLeft => "TorqueLeft",
            BrickName::Equals => "Equals",
        }
    }
}

/// Context in which a brick face is being used
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FaceContext {
    /// No parent face - used as the initial brick (seed)
    Initial,
    /// Alternative initial orientation
    Initial1,
    /// Placed on top of a parent face with this spin
    OnSpin(crate::build::tenscript::Spin),
}

impl FaceContext {
    pub fn name(self) -> &'static str {
        use crate::build::tenscript::Spin;
        match self {
            FaceContext::Initial => ":seed",
            FaceContext::Initial1 => ":seed-1",
            FaceContext::OnSpin(Spin::Right) => ":right",
            FaceContext::OnSpin(Spin::Left) => ":left",
        }
    }
}

/// Joint names for Single brick (both left and right variants)
#[derive(Copy, Clone, Debug)]
pub enum SingleJoint {
    AlphaX,
    AlphaY,
    AlphaZ,
    OmegaX,
    OmegaY,
    OmegaZ,
}

impl SingleJoint {
    pub fn name(self) -> &'static str {
        match self {
            Self::AlphaX => "alpha_x",
            Self::AlphaY => "alpha_y",
            Self::AlphaZ => "alpha_z",
            Self::OmegaX => "omega_x",
            Self::OmegaY => "omega_y",
            Self::OmegaZ => "omega_z",
        }
    }
}

impl From<SingleJoint> for String {
    fn from(joint: SingleJoint) -> String {
        joint.name().to_string()
    }
}

/// Joint names for Omni brick
#[derive(Copy, Clone, Debug)]
pub enum OmniJoint {
    BotAlphaX, BotAlphaY, BotAlphaZ,
    BotOmegaX, BotOmegaY, BotOmegaZ,
    TopAlphaX, TopAlphaY, TopAlphaZ,
    TopOmegaX, TopOmegaY, TopOmegaZ,
}

impl OmniJoint {
    pub fn name(self) -> &'static str {
        match self {
            Self::BotAlphaX => "bot_alpha_x",
            Self::BotAlphaY => "bot_alpha_y",
            Self::BotAlphaZ => "bot_alpha_z",
            Self::BotOmegaX => "bot_omega_x",
            Self::BotOmegaY => "bot_omega_y",
            Self::BotOmegaZ => "bot_omega_z",
            Self::TopAlphaX => "top_alpha_x",
            Self::TopAlphaY => "top_alpha_y",
            Self::TopAlphaZ => "top_alpha_z",
            Self::TopOmegaX => "top_omega_x",
            Self::TopOmegaY => "top_omega_y",
            Self::TopOmegaZ => "top_omega_z",
        }
    }
}

impl From<OmniJoint> for String {
    fn from(joint: OmniJoint) -> String {
        joint.name().to_string()
    }
}


/// Unified face names for all bricks
#[derive(Copy, Clone, Debug)]
pub enum Face {
    // Base faces
    Base,
    NextBase,  // Special marker for :next-base
    // Single brick faces
    Top,
    // Omni brick faces
    TopRight, TopLeft, TopX, TopY, TopZ,
    BotX, BotY, BotZ,
    Bot,
    FrontRight, FrontLeft,
    BackRight, BackLeft,
    BottomRight, BottomLeft,
    // Torque/Equals brick faces
    BaseBack, BaseSide, BaseFront,
    FarBack, FarSide, FarFront, FarBase, FarBrother,
    LeftFrontBottom, LeftBackBottom, RightBackBottom, RightFrontBottom,
    LeftBackTop, LeftFrontTop, RightFrontTop, RightBackTop,
    // TorqueRight/TorqueLeft specific faces
    OtherA, OtherB, Brother,
    FarOtherA, FarOtherB,
}

impl Face {
    pub fn name(self) -> &'static str {
        match self {
            Self::Base => ":base",
            Self::NextBase => ":next-base",
            Self::Top => "Top",
            Self::TopRight => "TopRight",
            Self::TopLeft => "TopLeft",
            Self::TopX => "TopX",
            Self::TopY => "TopY",
            Self::TopZ => "TopZ",
            Self::BotX => "BotX",
            Self::BotY => "BotY",
            Self::BotZ => "BotZ",
            Self::Bot => "Bot",
            Self::FrontRight => "FrontRight",
            Self::FrontLeft => "FrontLeft",
            Self::BackRight => "BackRight",
            Self::BackLeft => "BackLeft",
            Self::BottomRight => "BottomRight",
            Self::BottomLeft => "BottomLeft",
            Self::BaseBack => "BaseBack",
            Self::BaseSide => "BaseSide",
            Self::BaseFront => "BaseFront",
            Self::FarBack => "FarBack",
            Self::FarSide => "FarSide",
            Self::FarFront => "FarFront",
            Self::FarBase => "FarBase",
            Self::FarBrother => "FarBrother",
            Self::LeftFrontBottom => "LeftFrontBottom",
            Self::LeftBackBottom => "LeftBackBottom",
            Self::RightBackBottom => "RightBackBottom",
            Self::RightFrontBottom => "RightFrontBottom",
            Self::LeftBackTop => "LeftBackTop",
            Self::LeftFrontTop => "LeftFrontTop",
            Self::RightFrontTop => "RightFrontTop",
            Self::RightBackTop => "RightBackTop",
            Self::OtherA => "OtherA",
            Self::OtherB => "OtherB",
            Self::Brother => "Brother",
            Self::FarOtherA => "FarOtherA",
            Self::FarOtherB => "FarOtherB",
        }
    }
}


/// Create a FaceAlias from a list of strings
pub fn alias(names: &[&str]) -> FaceAlias {
    FaceAlias(names.iter().map(|s| s.to_string()).collect())
}

/// Builder for Prototype - provides a fluent API for constructing brick prototypes
pub struct ProtoBuilder {
    brick_name: BrickName,
    alias: FaceAlias,
    joints: Vec<String>,
    pushes: Vec<PushDef>,
    pulls: Vec<PullDef>,
    faces: Vec<FaceDef>,
}

impl ProtoBuilder {
    pub fn new(brick_name: BrickName) -> Self {
        Self {
            brick_name,
            alias: alias(&[brick_name.name()]),
            joints: vec![],
            pushes: vec![],
            pulls: vec![],
            faces: vec![],
        }
    }

    /// Add explicit joint declarations (joints that aren't created by pushes)
    pub fn joints<const N: usize>(mut self, joints: [impl Into<String>; N]) -> Self {
        self.joints = joints.into_iter().map(|j| j.into()).collect();
        self
    }

    pub fn pushes<const N: usize>(mut self, ideal: f32, pushes: [(impl Into<String>, impl Into<String>); N]) -> Self {
        use crate::build::tenscript::brick::Axis;
        self.pushes.extend(pushes.into_iter().map(|(alpha, omega)| PushDef {
            axis: Axis::X, // Axis is metadata, not used in our simplified builder
            alpha_name: alpha.into(),
            omega_name: omega.into(),
            ideal,
        }));
        self
    }
    
    pub fn pulls<const N: usize>(mut self, ideal: f32, pulls: [(impl Into<String>, impl Into<String>); N]) -> Self {
        self.pulls.extend(pulls.into_iter().map(|(alpha, omega)| PullDef {
            alpha_name: alpha.into(),
            omega_name: omega.into(),
            ideal,
            material: "pull".to_string(),
        }));
        self
    }
    
    /// Add a face with multiple context/face-name pairs - brick name is automatically added to aliases
    pub fn face<const N: usize>(mut self, spin: Spin, joints: [impl Into<String>; 3], 
                aliases: [(FaceContext, &[Face]); N]) -> Self {
        let face_aliases = aliases.into_iter()
            .map(|(context, faces)| {
                let names: Vec<&str> = faces.iter().map(|f| f.name()).collect();
                face_alias_for(self.brick_name, context, &names)
            })
            .collect();
        self.faces.push(FaceDef {
            spin,
            joint_names: joints.map(|j| j.into()),
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
        joint_map.insert(joint_name.clone(), idx);
    }

    // Add joints from pushes (starting after explicit joints)
    let offset = proto.joints.len();
    for (idx, push) in proto.pushes.iter().enumerate() {
        let alpha_idx = offset + idx * 2;
        let omega_idx = offset + idx * 2 + 1;
        joint_map.insert(push.alpha_name.clone(), alpha_idx);
        joint_map.insert(push.omega_name.clone(), omega_idx);
    }

    // Convert proto faces to baked faces
    proto.faces.iter().map(|face_def| {
        let joints = [
            *joint_map.get(&face_def.joint_names[0]).expect("Joint name not found"),
            *joint_map.get(&face_def.joint_names[1]).expect("Joint name not found"),
            *joint_map.get(&face_def.joint_names[2]).expect("Joint name not found"),
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
pub fn face_alias_for(brick: BrickName, context: FaceContext, names: &[&str]) -> FaceAlias {
    let mut parts = vec![context.name()];
    parts.extend_from_slice(names);
    parts.push(brick.name());
    alias(&parts)
}

/// Create a pull interval (cable) definition
pub fn pull(alpha: impl Into<String>, omega: impl Into<String>, ideal: f32, material: Material) -> PullDef {
    PullDef {
        alpha_name: alpha.into(),
        omega_name: omega.into(),
        ideal,
        material: material_name(material).to_string(),
    }
}

/// Create a face definition with spin and joint names
pub fn face<J: Into<String>>(spin: Spin, joints: [J; 3], aliases: impl Into<Vec<FaceAlias>>) -> FaceDef {
    FaceDef {
        spin,
        joint_names: joints.map(|j| j.into()),
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
