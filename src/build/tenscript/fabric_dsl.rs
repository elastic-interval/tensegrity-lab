/// Type-safe DSL for defining fabric plans with a fluent API.

use crate::build::tenscript::build_phase::{BuildNode, BuildPhase};
use crate::build::tenscript::converge_phase::ConvergePhase;
use crate::build::tenscript::fabric_plan::FabricPlan;
use crate::build::tenscript::pretense_phase::PretensePhase;
use crate::build::tenscript::shape_phase::ShapeOperation;
use crate::build::tenscript::FaceAlias;
use crate::fabric::physics::SurfaceCharacter;
use crate::units::{Millimeters, Seconds};

pub use crate::build::tenscript::brick_dsl::{BrickName, Face};
pub use crate::build::tenscript::build_phase::BuildNode as Node;
pub use crate::units::{Millimeters as Mm, Seconds as Sec};


/// Create a FaceAlias from a Face enum
impl From<Face> for FaceAlias {
    fn from(face: Face) -> Self {
        FaceAlias::single(face.name())
    }
}

/// Create a FaceAlias from a BrickName enum
impl From<BrickName> for FaceAlias {
    fn from(brick: BrickName) -> Self {
        FaceAlias::single(brick.name())
    }
}


/// Start building a fabric plan
pub fn fabric(name: impl Into<String>) -> FabricBuilder {
    FabricBuilder {
        name: name.into(),
        build: None,
        shape: Vec::new(),
        pretense: PretensePhaseBuilder::default(),
        converge: None,
        animate: None,
        scale: Millimeters(1.0),
    }
}

pub struct FabricBuilder {
    name: String,
    build: Option<BuildNode>,
    shape: Vec<ShapeOperation>,
    pretense: PretensePhaseBuilder,
    converge: Option<Seconds>,
    #[allow(dead_code)]
    animate: Option<()>, // TODO: AnimatePhase when needed
    scale: Millimeters,
}

impl FabricBuilder {
    pub fn build(mut self, node: BuildNode) -> Self {
        self.build = Some(node);
        self
    }

    pub fn shape<const N: usize>(mut self, ops: [ShapeOperation; N]) -> Self {
        self.shape = ops.into();
        self
    }

    pub fn pretense(mut self, builder: PretensePhaseBuilder) -> Self {
        self.pretense = builder;
        self
    }

    pub fn converge(mut self, seconds: Seconds) -> Self {
        self.converge = Some(seconds);
        self
    }

    pub fn scale(mut self, scale: Millimeters) -> Self {
        self.scale = scale;
        self
    }

    pub fn build_plan(self) -> FabricPlan {
        FabricPlan {
            name: self.name,
            build_phase: BuildPhase::new(self.build.expect("build phase required")),
            shape_phase: crate::build::tenscript::shape_phase::ShapePhase {
                operations: self.shape,
                marks: Vec::new(),
                spacers: Vec::new(),
                joiners: Vec::new(),
                anchors: Vec::new(),
                shape_operation_index: 0,
            },
            pretense_phase: self.pretense.build(),
            converge_phase: self.converge.map(|seconds| ConvergePhase { seconds }),
            animate_phase: None,
            scale: self.scale.0,
        }
    }
}


/// Create a branch node
pub fn branch(alias: impl Into<FaceAlias>) -> BranchBuilder {
    BranchBuilder {
        alias: alias.into(),
        rotation: 0,
        scale_factor: 1.0,
        seed: None,
        face_nodes: Vec::new(),
    }
}

pub struct BranchBuilder {
    alias: FaceAlias,
    rotation: usize,
    scale_factor: f32,
    seed: Option<usize>,
    face_nodes: Vec<BuildNode>,
}

impl BranchBuilder {
    pub fn seed(mut self, seed: usize) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale_factor = scale;
        self
    }

    pub fn rotate(mut self) -> Self {
        self.rotation += 1;
        self
    }

    pub fn on_face(mut self, alias: impl Into<FaceAlias>, node: BuildNode) -> Self {
        self.face_nodes.push(BuildNode::Face {
            alias: alias.into(),
            node: Box::new(node),
        });
        self
    }

    pub fn build(self) -> BuildNode {
        BuildNode::Branch {
            alias: self.alias,
            rotation: self.rotation,
            scale_factor: self.scale_factor,
            seed: self.seed,
            face_nodes: self.face_nodes,
        }
    }
}

/// Create a grow node
pub fn grow(forward: impl Into<String>) -> GrowBuilder {
    GrowBuilder {
        forward: forward.into(),
        scale_factor: 1.0,
        post_growth_nodes: Vec::new(),
    }
}

/// Create a BuildNode that just marks a location (no grow)
pub fn grow_mark(mark_name: impl Into<String>) -> BuildNode {
    BuildNode::Mark {
        mark_name: mark_name.into(),
    }
}

pub struct GrowBuilder {
    forward: String,
    scale_factor: f32,
    post_growth_nodes: Vec<BuildNode>,
}

impl GrowBuilder {
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale_factor = scale;
        self
    }

    pub fn mark(mut self, mark_name: impl Into<String>) -> Self {
        self.post_growth_nodes.push(BuildNode::Mark {
            mark_name: mark_name.into(),
        });
        self
    }

    pub fn prism(mut self) -> Self {
        self.post_growth_nodes.push(BuildNode::Prism);
        self
    }

    pub fn build_node(mut self, node: BuildNode) -> Self {
        self.post_growth_nodes.push(node);
        self
    }

    pub fn build(self) -> BuildNode {
        BuildNode::Grow {
            forward: self.forward,
            scale_factor: self.scale_factor,
            post_growth_nodes: self.post_growth_nodes,
        }
    }
}

// Shape Operations

pub fn during<const N: usize>(seconds: Seconds, ops: [ShapeOperation; N]) -> ShapeOperation {
    ShapeOperation::During {
        seconds,
        operations: ops.into(),
    }
}

pub fn space(mark_name: impl Into<String>, distance_factor: f32) -> ShapeOperation {
    ShapeOperation::Spacer {
        mark_name: mark_name.into(),
        distance_factor,
    }
}

pub fn vulcanize() -> ShapeOperation {
    ShapeOperation::Vulcanize
}

pub fn join(mark_name: impl Into<String>) -> ShapeOperation {
    ShapeOperation::Joiner {
        mark_name: mark_name.into(),
        seed: None,
    }
}

pub fn join_seed(mark_name: impl Into<String>, seed: usize) -> ShapeOperation {
    ShapeOperation::Joiner {
        mark_name: mark_name.into(),
        seed: Some(seed),
    }
}

pub fn down(mark_name: impl Into<String>) -> ShapeOperation {
    ShapeOperation::PointDownwards {
        mark_name: mark_name.into(),
    }
}

pub fn centralize() -> ShapeOperation {
    ShapeOperation::Centralize { altitude: None }
}

pub fn centralize_at(altitude: f32) -> ShapeOperation {
    ShapeOperation::Centralize {
        altitude: Some(altitude),
    }
}

pub fn anchor(joint_index: usize, surface: (f32, f32)) -> ShapeOperation {
    ShapeOperation::Anchor {
        joint_index,
        surface,
    }
}

pub fn guy_line(joint_index: usize, length: f32, surface: (f32, f32)) -> ShapeOperation {
    ShapeOperation::GuyLine {
        joint_index,
        length,
        surface,
    }
}

pub fn omit(alpha_index: usize, omega_index: usize) -> ShapeOperation {
    ShapeOperation::Omit((alpha_index, omega_index))
}

pub fn add(alpha_index: usize, omega_index: usize, length_factor: f32) -> ShapeOperation {
    ShapeOperation::Add {
        alpha_index,
        omega_index,
        length_factor,
    }
}

// Pretense Phase

#[derive(Default)]
pub struct PretensePhaseBuilder {
    surface_character: Option<SurfaceCharacter>,
    altitude: Option<Millimeters>,
    pretenst: Option<f32>,
    rigidity: Option<f32>,
}

impl PretensePhaseBuilder {
    pub fn surface(mut self, character: SurfaceCharacter) -> Self {
        self.surface_character = Some(character);
        self
    }

    pub fn altitude(mut self, altitude: Millimeters) -> Self {
        self.altitude = Some(altitude);
        self
    }

    pub fn pretenst(mut self, pretenst: f32) -> Self {
        self.pretenst = Some(pretenst);
        self
    }

    pub fn rigidity(mut self, rigidity: f32) -> Self {
        self.rigidity = Some(rigidity);
        self
    }

    pub(crate) fn build(self) -> PretensePhase {
        PretensePhase {
            surface_character: self.surface_character,
            pretenst: self.pretenst,
            seconds: None,
            rigidity: self.rigidity,
            altitude: self.altitude.map(|mm| mm.0),
        }
    }
}

pub fn pretense() -> PretensePhaseBuilder {
    PretensePhaseBuilder::default()
}
