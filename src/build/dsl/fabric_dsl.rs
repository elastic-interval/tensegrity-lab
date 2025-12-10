/// Type-safe DSL for defining fabric plans with a fluent API.
use crate::build::dsl::build_phase::{BuildNode, BuildPhase, Chirality, ColumnStyle};
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::{ShapeAction, ShapeStep};
use crate::fabric::physics::SurfaceCharacter;
use crate::units::{Meters, Percent, Seconds};

pub use crate::build::dsl::animate_phase::{Actuator, ActuatorSpec, Waveform};
pub use crate::build::dsl::brick_dsl::{BrickName, BrickOrientation, BrickRole, FaceName, MarkName};
pub use crate::units::Percent as Pct;
pub use crate::build::dsl::build_phase::BuildNode as Node;
pub use crate::units::{Meters as M, Seconds as Sec};

/// Start building a fabric plan
pub fn fabric(name: impl Into<String>) -> FabricStage1 {
    FabricStage1 { name: name.into() }
}

/// Stage 1: Need altitude
pub struct FabricStage1 {
    name: String,
}

impl FabricStage1 {
    pub fn altitude(self, altitude: Meters) -> FabricStage2 {
        FabricStage2 {
            name: self.name,
            altitude,
        }
    }
}

/// Stage 2: Need scale
pub struct FabricStage2 {
    name: String,
    altitude: Meters,
}

impl FabricStage2 {
    pub fn scale(self, scale: Meters) -> FabricBuilder {
        FabricBuilder {
            name: self.name,
            altitude: self.altitude,
            build: None,
            shape: Vec::new(),
            pretense: PretensePhaseBuilder::default(),
            scale,
        }
    }
}

pub struct FabricBuilder {
    name: String,
    altitude: Meters,
    build: Option<BuildNode>,
    shape: Vec<ShapeStep>,
    pretense: PretensePhaseBuilder,
    scale: Meters,
}

impl FabricBuilder {
    pub fn seed(self, brick_name: BrickName, brick_role: BrickRole) -> SeedChain {
        SeedChain {
            fabric: self,
            hub: HubBuilder {
                brick_name,
                brick_role,
                rotation: 0,
                scale: Percent(100.0),
                face_nodes: Vec::new(),
            },
        }
    }

    // Shape operations - each starts or continues the shape chain
    pub fn space(mut self, seconds: Seconds, mark_name: MarkName, distance: Percent) -> Self {
        self.shape.push(ShapeStep { seconds, action: ShapeAction::Spacer { mark_name, distance } });
        self
    }

    pub fn join(mut self, seconds: Seconds, mark_name: MarkName) -> Self {
        self.shape.push(ShapeStep { seconds, action: ShapeAction::Joiner { mark_name } });
        self
    }

    pub fn vulcanize(mut self, seconds: Seconds) -> Self {
        self.shape.push(ShapeStep { seconds, action: ShapeAction::Vulcanize });
        self
    }

    pub fn down(mut self, seconds: Seconds, mark_name: MarkName) -> Self {
        self.shape.push(ShapeStep { seconds, action: ShapeAction::PointDownwards { mark_name } });
        self
    }

    pub fn centralize(mut self, seconds: Seconds) -> Self {
        self.shape.push(ShapeStep { seconds, action: ShapeAction::Centralize });
        self
    }

    pub fn centralize_at(mut self, seconds: Seconds, altitude: Meters) -> Self {
        self.shape.push(ShapeStep { seconds, action: ShapeAction::CentralizeAt { altitude } });
        self
    }

    pub fn pretense(mut self, seconds: Seconds) -> PretenseChain {
        self.pretense.seconds = Some(seconds);
        PretenseChain { fabric: self }
    }

    pub(crate) fn build_plan(self) -> FabricPlan {
        let scale_mm = self.scale.to_millimeters();
        FabricPlan {
            name: self.name,
            build_phase: BuildPhase::new(
                self.build.expect("build phase required"),
                self.altitude.0 / self.scale.0,
            ),
            shape_phase: crate::build::dsl::shape_phase::ShapePhase {
                steps: self.shape,
                marks: Vec::new(),
                spacers: Vec::new(),
                joiners: Vec::new(),
                anchors: Vec::new(),
                step_index: 0,
                scale: scale_mm,
            },
            pretense_phase: self.pretense.build(),
            fall_phase: FallPhase { seconds: Seconds(5.0) },
            settle_phase: None,
            animate_phase: None,
            scale: scale_mm.0,
            altitude: self.altitude.to_millimeters(),
        }
    }
}

/// Create a hub node (places a brick with multiple faces)
pub fn hub(brick_name: BrickName, brick_role: BrickRole) -> HubBuilder {
    HubBuilder {
        brick_name,
        brick_role,
        rotation: 0,
        scale: Percent(100.0),
        face_nodes: Vec::new(),
    }
}

pub struct HubBuilder {
    brick_name: BrickName,
    brick_role: BrickRole,
    rotation: usize,
    scale: Percent,
    face_nodes: Vec<BuildNode>,
}

impl HubBuilder {
    /// Shrink this brick by the given percentage (e.g., Pct(10.0) means 90% scale)
    pub fn shrink_by(mut self, percent: Percent) -> Self {
        self.scale = Percent(100.0 - percent.0);
        self
    }

    /// Grow this brick by the given percentage (e.g., Pct(10.0) means 110% scale)
    pub fn grow_by(mut self, percent: Percent) -> Self {
        self.scale = Percent(100.0 + percent.0);
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
        BuildNode::Hub {
            brick_name: self.brick_name,
            brick_role: self.brick_role,
            rotation: self.rotation,
            scale: self.scale,
            face_nodes: self.face_nodes,
        }
    }
}

/// Chain for seed configuration that transitions to FabricBuilder
pub struct SeedChain {
    fabric: FabricBuilder,
    hub: HubBuilder,
}

impl SeedChain {
    pub fn shrink_by(mut self, percent: Percent) -> Self {
        self.hub = self.hub.shrink_by(percent);
        self
    }

    pub fn grow_by(mut self, percent: Percent) -> Self {
        self.hub = self.hub.grow_by(percent);
        self
    }

    pub fn rotate(mut self) -> Self {
        self.hub = self.hub.rotate();
        self
    }

    pub fn on_face(mut self, face_name: FaceName, node: BuildNode) -> Self {
        self.hub = self.hub.on_face(face_name, node);
        self
    }

    fn finalize_build(mut self) -> FabricBuilder {
        self.fabric.build = Some(self.hub.build());
        self.fabric
    }

    // Shape operations
    pub fn space(self, seconds: Seconds, mark_name: MarkName, distance: Percent) -> FabricBuilder {
        self.finalize_build().space(seconds, mark_name, distance)
    }

    pub fn join(self, seconds: Seconds, mark_name: MarkName) -> FabricBuilder {
        self.finalize_build().join(seconds, mark_name)
    }

    pub fn vulcanize(self, seconds: Seconds) -> FabricBuilder {
        self.finalize_build().vulcanize(seconds)
    }

    pub fn down(self, seconds: Seconds, mark_name: MarkName) -> FabricBuilder {
        self.finalize_build().down(seconds, mark_name)
    }

    pub fn centralize(self, seconds: Seconds) -> FabricBuilder {
        self.finalize_build().centralize(seconds)
    }

    pub fn centralize_at(self, seconds: Seconds, altitude: Meters) -> FabricBuilder {
        self.finalize_build().centralize_at(seconds, altitude)
    }

    // Terminal operation (no shape phase)
    pub fn pretense(self, seconds: Seconds) -> PretenseChain {
        self.finalize_build().pretense(seconds)
    }
}

/// Create a column node (extends a column of bricks, defaults to alternating chirality)
pub fn column(count: usize) -> ColumnBuilder {
    ColumnBuilder {
        style: ColumnStyle::alternating(count),
        scale: Percent(100.0),
        post_column_nodes: Vec::new(),
    }
}

/// Create a BuildNode that just marks a location (no column)
pub fn mark(mark_name: MarkName) -> BuildNode {
    BuildNode::Mark { mark_name }
}

pub struct ColumnBuilder {
    style: ColumnStyle,
    scale: Percent,
    post_column_nodes: Vec<BuildNode>,
}

impl ColumnBuilder {
    pub fn chiral(mut self) -> Self {
        self.style.chirality = Chirality::Chiral;
        self
    }

    /// Shrink each successive brick by the given percentage (e.g., Pct(10.0) means 90% scale per brick)
    pub fn shrink_by(mut self, percent: Percent) -> Self {
        self.scale = Percent(100.0 - percent.0);
        self
    }

    /// Grow each successive brick by the given percentage (e.g., Pct(10.0) means 110% scale per brick)
    pub fn grow_by(mut self, percent: Percent) -> Self {
        self.scale = Percent(100.0 + percent.0);
        self
    }

    pub fn mark(mut self, mark_name: MarkName) -> Self {
        self.post_column_nodes.push(BuildNode::Mark { mark_name });
        self
    }

    pub fn prism(mut self) -> Self {
        self.post_column_nodes.push(BuildNode::Prism);
        self
    }

    pub fn build_node(mut self, node: BuildNode) -> Self {
        self.post_column_nodes.push(node);
        self
    }

    pub fn build(self) -> BuildNode {
        BuildNode::Column {
            style: self.style,
            scale: self.scale,
            post_column_nodes: self.post_column_nodes,
        }
    }
}

// Pretense Phase

#[derive(Default)]
pub struct PretensePhaseBuilder {
    surface: Option<SurfaceCharacter>,
    altitude: Option<Meters>,
    pretenst: Option<Percent>,
    rigidity: Option<Percent>,
    seconds: Option<Seconds>,
}

impl PretensePhaseBuilder {
    pub fn surface(mut self, surface: SurfaceCharacter) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn altitude(mut self, altitude: Meters) -> Self {
        self.altitude = Some(altitude);
        self
    }

    pub fn pretenst(mut self, pretenst: Percent) -> Self {
        self.pretenst = Some(pretenst);
        self
    }

    pub fn rigidity(mut self, rigidity: Percent) -> Self {
        self.rigidity = Some(rigidity);
        self
    }

    pub(crate) fn build(self) -> PretensePhase {
        PretensePhase {
            surface: self.surface,
            pretenst: self.pretenst,
            seconds: self.seconds,
            rigidity: self.rigidity,
            altitude: self.altitude.map(|m| m.to_millimeters().0),
        }
    }
}

/// Chained pretense configuration - must specify surface to complete the plan
pub struct PretenseChain {
    fabric: FabricBuilder,
}

impl PretenseChain {
    pub fn altitude(mut self, altitude: Meters) -> Self {
        self.fabric.pretense.altitude = Some(altitude);
        self
    }

    pub fn pretenst(mut self, pretenst: Percent) -> Self {
        self.fabric.pretense.pretenst = Some(pretenst);
        self
    }

    pub fn rigidity(mut self, rigidity: Percent) -> Self {
        self.fabric.pretense.rigidity = Some(rigidity);
        self
    }

    pub fn surface_frozen(mut self) -> FabricPlan {
        self.fabric.pretense.surface = Some(SurfaceCharacter::Frozen);
        self.fabric.build_plan()
    }

    pub fn surface_bouncy(mut self) -> FabricPlan {
        self.fabric.pretense.surface = Some(SurfaceCharacter::Bouncy);
        self.fabric.build_plan()
    }

    pub fn floating(self) -> FabricPlan {
        // No surface interaction - fabric floats in space
        self.fabric.build_plan()
    }
}
