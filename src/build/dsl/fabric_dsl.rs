/// Type-safe DSL for defining fabric plans with a fluent API.
use crate::build::dsl::animate_phase::AnimatePhase;
use crate::build::dsl::build_phase::{BuildNode, BuildPhase, Chirality, GrowStyle};
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::settle_phase::SettlePhase;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::ShapeOperation;
use crate::fabric::physics::SurfaceCharacter;
use crate::units::{Millimeters, Seconds};

pub use crate::build::dsl::animate_phase::{Muscle, MuscleSpec};
pub use crate::build::dsl::brick_dsl::{BrickName, BrickOrientation, MarkName};
pub use crate::units::Amplitude;
use crate::build::dsl::brick_dsl::{BrickRole, FaceName};
pub use crate::build::dsl::build_phase::BuildNode as Node;
pub use crate::units::{Millimeters as Mm, Seconds as Sec};

/// Start building a fabric plan with initial altitude
pub fn fabric(name: impl Into<String>, altitude: Millimeters) -> FabricBuilder {
    FabricBuilder {
        name: name.into(),
        altitude,
        build: None,
        shape: Vec::new(),
        pretense: PretensePhaseBuilder::default(),
        fall: Seconds(5.0),
        settle: Seconds(10.0),
        animate: None,
        scale: Millimeters(1000.0),
    }
}

pub struct FabricBuilder {
    name: String,
    altitude: Millimeters,
    build: Option<BuildNode>,
    shape: Vec<ShapeOperation>,
    pretense: PretensePhaseBuilder,
    fall: Seconds,
    settle: Seconds,
    animate: Option<AnimatePhase>,
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

    pub fn fall(mut self, seconds: Seconds) -> Self {
        self.fall = seconds;
        self
    }

    pub fn settle(mut self, seconds: Seconds) -> Self {
        self.settle = seconds;
        self
    }

    pub fn animate(
        mut self,
        period: Seconds,
        amplitude: Amplitude,
        muscles: Vec<Muscle>,
    ) -> Self {
        self.animate = Some(AnimatePhase {
            period,
            amplitude,
            muscles,
        });
        self
    }

    pub fn scale(mut self, scale: Millimeters) -> Self {
        self.scale = scale;
        self
    }

    pub fn build_plan(self) -> FabricPlan {
        FabricPlan {
            name: self.name,
            build_phase: BuildPhase::new(
                self.build.expect("build phase required"),
                self.altitude.0 / self.scale.0,
            ),
            shape_phase: crate::build::dsl::shape_phase::ShapePhase {
                operations: self.shape,
                marks: Vec::new(),
                spacers: Vec::new(),
                joiners: Vec::new(),
                anchors: Vec::new(),
                shape_operation_index: 0,
            },
            pretense_phase: self.pretense.build(),
            fall_phase: FallPhase { seconds: self.fall },
            settle_phase: SettlePhase { seconds: self.settle },
            animate_phase: self.animate,
            scale: self.scale.0,
            altitude: self.altitude,
        }
    }
}

/// Create a branch node
pub fn branching(brick_name: BrickName, brick_role: BrickRole) -> BranchBuilder {
    BranchBuilder {
        brick_name,
        brick_role,
        rotation: 0,
        scale_factor: 1.0,
        face_nodes: Vec::new(),
    }
}

pub struct BranchBuilder {
    brick_name: BrickName,
    brick_role: BrickRole,
    rotation: usize,
    scale_factor: f32,
    face_nodes: Vec<BuildNode>,
}

impl BranchBuilder {
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale_factor = scale;
        self
    }

    pub fn rotate(mut self) -> Self {
        self.rotation += 1;
        self
    }

    pub fn on_face(mut self, face_name: FaceName, node: BuildNode) -> Self {
        self.face_nodes.push(BuildNode::Face {
            alias: self.brick_role.calls_it(face_name),
            node: Box::new(node),
        });
        self
    }

    pub fn build(self) -> BuildNode {
        BuildNode::Branch {
            brick_name: self.brick_name,
            brick_role: self.brick_role,
            rotation: self.rotation,
            scale_factor: self.scale_factor,
            face_nodes: self.face_nodes,
        }
    }
}

/// Create a grow node (defaults to alternating chirality)
pub fn growing(count: usize) -> GrowBuilder {
    GrowBuilder {
        style: GrowStyle::alternating(count),
        scale_factor: 1.0,
        post_growth_nodes: Vec::new(),
    }
}

/// Create a BuildNode that just marks a location (no grow)
pub fn grow_mark(mark_name: MarkName) -> BuildNode {
    BuildNode::Mark { mark_name }
}

pub struct GrowBuilder {
    style: GrowStyle,
    scale_factor: f32,
    post_growth_nodes: Vec<BuildNode>,
}

impl GrowBuilder {
    pub fn chiral(mut self) -> Self {
        self.style.chirality = Chirality::Chiral;
        self
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale_factor = scale;
        self
    }

    pub fn mark(mut self, mark_name: MarkName) -> Self {
        self.post_growth_nodes.push(BuildNode::Mark { mark_name });
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
            style: self.style,
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

pub fn space(mark_name: MarkName, distance_factor: f32) -> ShapeOperation {
    ShapeOperation::Spacer {
        mark_name,
        distance_factor,
    }
}

pub fn vulcanize() -> ShapeOperation {
    ShapeOperation::Vulcanize
}

pub fn join(mark_name: MarkName) -> ShapeOperation {
    ShapeOperation::Joiner { mark_name }
}
pub fn down(mark_name: MarkName) -> ShapeOperation {
    ShapeOperation::PointDownwards { mark_name }
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
    seconds: Option<Seconds>,
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
            seconds: self.seconds,
            rigidity: self.rigidity,
            altitude: self.altitude.map(|mm| mm.0),
        }
    }
}

pub fn pretense(seconds: Seconds) -> PretensePhaseBuilder {
    PretensePhaseBuilder {
        seconds: Some(seconds),
        ..Default::default()
    }
}
