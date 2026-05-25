/// Type-safe DSL for defining fabric plans with a fluent API.
use crate::build::dsl::build_phase::{BuildNode, BuildPhase};
use crate::build::dsl::fabric_library::FabricName;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::build::dsl::fall_phase::FallPhase;
use crate::build::dsl::pretense_phase::PretensePhase;
use crate::build::dsl::shape_phase::{ShapeAction, ShapeStep};
use crate::fabric::joint_path::JointPath;
use crate::fabric::physics::SurfaceCharacter;
use crate::units::{Meters, Percent, Seconds, Unit};

pub use crate::build::dsl::animate_phase::{phase, Actuator, Waveform};
pub use crate::build::dsl::brick_dsl::{
    BrickName, BrickOrientation, BrickRole, FaceName, MarkName,
};
pub use crate::build::dsl::build_phase::BuildNode as Node;
pub use crate::fabric::vulcanize::VulcanizeMode;
pub use crate::fabric::FabricDimensions;
pub use crate::units::Percent as Pct;
pub use crate::units::{Grams as Gm, GramsPerMeter as GpmM, Hertz as Hz, Meters as M, Seconds as Sec};

impl FabricName {
    pub fn build(self, dimensions: FabricDimensions) -> FabricBuilder {
        FabricBuilder {
            name: self,
            dimensions,
            build: None,
            shape: Vec::new(),
            omit_pairs: Vec::new(),
        }
    }
}

pub struct FabricBuilder {
    name: FabricName,
    dimensions: FabricDimensions,
    build: Option<BuildNode>,
    shape: Vec<ShapeStep>,
    omit_pairs: Vec<(JointPath, JointPath)>,
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
        self.shape.push(ShapeStep {
            seconds,
            action: ShapeAction::Spacer {
                mark_name,
                distance,
            },
        });
        self
    }

    pub fn join(mut self, seconds: Seconds, mark_name: MarkName) -> Self {
        self.shape.push(ShapeStep {
            seconds,
            action: ShapeAction::Joiner { mark_name },
        });
        self
    }

    pub fn prepare_vulcanize(mut self, contraction: f32, mode: VulcanizeMode) -> Self {
        self.shape.push(ShapeStep {
            seconds: Seconds(0.0),
            action: ShapeAction::PrepareVulcanize { contraction, mode },
        });
        self
    }

    pub fn vulcanize(mut self, seconds: Seconds) -> Self {
        self.shape.push(ShapeStep {
            seconds,
            action: ShapeAction::Vulcanize,
        });
        self
    }

    pub fn down(mut self, seconds: Seconds, mark_name: MarkName) -> Self {
        self.shape.push(ShapeStep {
            seconds,
            action: ShapeAction::PointDownwards { mark_name },
        });
        self
    }

    pub fn centralize(mut self, seconds: Seconds) -> Self {
        self.shape.push(ShapeStep {
            seconds,
            action: ShapeAction::Centralize,
        });
        self
    }

    pub fn centralize_at(mut self, seconds: Seconds, altitude: Meters) -> Self {
        self.shape.push(ShapeStep {
            seconds,
            action: ShapeAction::CentralizeAt { altitude },
        });
        self
    }

    pub fn omit<const N: usize>(mut self, pairs: [(&str, &str); N]) -> Self {
        self.omit_pairs
            .extend(pairs.iter().map(|(a, b)| ((*a).into(), (*b).into())));
        self
    }

    pub fn pretense(
        self,
        seconds: Seconds,
        pretenst: Percent,
    ) -> PretenseChain {
        PretenseChain {
            fabric: self,
            pretenst,
            surface: None,
            rigidity: None,
            seconds: Some(seconds),
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

    /// Add faces to this hub
    pub fn faces<const N: usize>(mut self, faces: [impl Into<Face>; N]) -> Self {
        for face in faces {
            let face = face.into();
            self.face_nodes.push(BuildNode::Face {
                alias: self.brick_role.calls_it(face.face_name),
                node: Box::new(face.node),
            });
        }
        self
    }
}

/// A face definition for use in .faces([...])
pub struct Face {
    face_name: FaceName,
    node: BuildNode,
}

/// Start defining a face (for use in .faces([...]))
pub fn on(face_name: FaceName) -> FaceBuilder {
    FaceBuilder { face_name }
}

/// Builder for face content
pub struct FaceBuilder {
    face_name: FaceName,
}

impl FaceBuilder {
    /// Start with a column on this face
    pub fn column(self, count: usize) -> FaceColumnBuilder {
        FaceColumnBuilder {
            face_name: self.face_name,
            column: column(count),
        }
    }

    /// Just mark this face
    pub fn mark(self, mark_name: MarkName) -> Face {
        Face {
            face_name: self.face_name,
            node: mark(mark_name),
        }
    }

    pub fn radials_only(self) -> FaceColumnBuilder {
        FaceColumnBuilder {
            face_name: self.face_name,
            column: ColumnBuilder {
                count: 0,
                scale: Percent(100.0),
                post_column_nodes: vec![BuildNode::RadialsOnly],
            },
        }
    }

    pub fn open(self) -> FaceColumnBuilder {
        FaceColumnBuilder {
            face_name: self.face_name,
            column: ColumnBuilder {
                count: 0,
                scale: Percent(100.0),
                post_column_nodes: vec![BuildNode::Open],
            },
        }
    }

    pub fn prism(self, outer_percent: Percent) -> FaceColumnBuilder {
        FaceColumnBuilder {
            face_name: self.face_name,
            column: ColumnBuilder {
                count: 0,
                scale: Percent(100.0),
                post_column_nodes: vec![BuildNode::Prism { outer_percent }],
            },
        }
    }
}

/// Builder for column content on a face
pub struct FaceColumnBuilder {
    face_name: FaceName,
    column: ColumnBuilder,
}

impl FaceColumnBuilder {
    pub fn shrink_by(mut self, percent: Percent) -> Self {
        self.column = self.column.shrink_by(percent);
        self
    }

    pub fn grow_by(mut self, percent: Percent) -> Self {
        self.column = self.column.grow_by(percent);
        self
    }

    pub fn mark(mut self, mark_name: MarkName) -> Self {
        self.column = self.column.mark(mark_name);
        self
    }

    pub fn prism(mut self, outer_percent: Percent) -> Self {
        self.column = self.column.prism(outer_percent);
        self
    }

    pub fn radials_only(mut self) -> Self {
        self.column = self.column.radials_only();
        self
    }

    pub fn open(mut self) -> Self {
        self.column = self.column.open();
        self
    }

    pub fn then(mut self, node: impl Into<BuildNode>) -> Self {
        self.column = self.column.then(node);
        self
    }
}

impl From<FaceColumnBuilder> for Face {
    fn from(builder: FaceColumnBuilder) -> Face {
        Face {
            face_name: builder.face_name,
            node: builder.column.into(),
        }
    }
}

impl From<HubBuilder> for BuildNode {
    fn from(builder: HubBuilder) -> BuildNode {
        BuildNode::Hub {
            brick_name: builder.brick_name,
            brick_role: builder.brick_role,
            rotation: builder.rotation,
            scale: builder.scale,
            face_nodes: builder.face_nodes,
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

    /// Add faces to the seed brick
    pub fn faces<const N: usize>(mut self, faces: [impl Into<Face>; N]) -> Self {
        for face in faces {
            let face = face.into();
            self.hub.face_nodes.push(BuildNode::Face {
                alias: self.hub.brick_role.calls_it(face.face_name),
                node: Box::new(face.node),
            });
        }
        self
    }

    fn finalize_build(mut self) -> FabricBuilder {
        self.fabric.build = Some(self.hub.into());
        self.fabric
    }

    // Shape operations
    pub fn space(self, seconds: Seconds, mark_name: MarkName, distance: Percent) -> FabricBuilder {
        self.finalize_build().space(seconds, mark_name, distance)
    }

    pub fn join(self, seconds: Seconds, mark_name: MarkName) -> FabricBuilder {
        self.finalize_build().join(seconds, mark_name)
    }

    pub fn prepare_vulcanize(self, contraction: f32, mode: VulcanizeMode) -> FabricBuilder {
        self.finalize_build().prepare_vulcanize(contraction, mode)
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

    pub fn omit<const N: usize>(self, pairs: [(&str, &str); N]) -> FabricBuilder {
        self.finalize_build().omit(pairs)
    }

    pub fn pretense(
        self,
        seconds: Seconds,
        pretenst: Percent,
    ) -> PretenseChain {
        self.finalize_build().pretense(seconds, pretenst)
    }
}

/// Create a column node (extends a column of bricks).
pub fn column(count: usize) -> ColumnBuilder {
    ColumnBuilder {
        count,
        scale: Percent(100.0),
        post_column_nodes: Vec::new(),
    }
}

/// Create a BuildNode that just marks a location (no column)
pub fn mark(mark_name: MarkName) -> BuildNode {
    BuildNode::Mark { mark_name }
}

pub struct ColumnBuilder {
    count: usize,
    scale: Percent,
    post_column_nodes: Vec<BuildNode>,
}

impl ColumnBuilder {
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

    pub fn prism(mut self, outer_percent: Percent) -> Self {
        self.post_column_nodes
            .push(BuildNode::Prism { outer_percent });
        self
    }

    pub fn radials_only(mut self) -> Self {
        self.post_column_nodes.push(BuildNode::RadialsOnly);
        self
    }

    pub fn open(mut self) -> Self {
        self.post_column_nodes.push(BuildNode::Open);
        self
    }

    pub fn build_node(mut self, node: impl Into<BuildNode>) -> Self {
        self.post_column_nodes.push(node.into());
        self
    }

    /// Continue with a nested structure at the end of this column
    pub fn then(mut self, node: impl Into<BuildNode>) -> Self {
        self.post_column_nodes.push(node.into());
        self
    }
}

impl From<ColumnBuilder> for BuildNode {
    fn from(builder: ColumnBuilder) -> BuildNode {
        BuildNode::Column {
            count: builder.count,
            scale: builder.scale,
            post_column_nodes: builder.post_column_nodes,
        }
    }
}

pub struct PretenseChain {
    fabric: FabricBuilder,
    pretenst: Percent,
    surface: Option<SurfaceCharacter>,
    rigidity: Option<Percent>,
    seconds: Option<Seconds>,
}

impl PretenseChain {
    pub fn rigidity(mut self, rigidity: Percent) -> Self {
        self.rigidity = Some(rigidity);
        self
    }

    fn build_plan(self) -> FabricPlan {
        let dims = self.fabric.dimensions;
        let pretense_phase = PretensePhase {
            surface: self.surface,
            seconds: self.seconds,
            rigidity: self.rigidity,
            omit_pairs: self.fabric.omit_pairs,
            pretenst: self.pretenst,
        };
        FabricPlan {
            name: self.fabric.name,
            build_phase: BuildPhase::new(
                self.fabric.build.expect("build phase required"),
                dims.altitude.f32() / dims.scale.f32(),
            ),
            shape_phase: crate::build::dsl::shape_phase::ShapePhase {
                steps: self.fabric.shape,
                marks: Vec::new(),
                spacers: Vec::new(),
                joiners: Vec::new(),
                anchors: Vec::new(),
                step_index: 0,
                scale: dims.scale,
            },
            pretense_phase,
            fall_phase: FallPhase {
                seconds: Seconds(5.0),
            },
            settle_phase: None,
            animate_phase: None,
            dimensions: dims,
        }
    }

    pub fn surface_frozen(mut self) -> FabricPlan {
        self.surface = Some(SurfaceCharacter::Frozen);
        self.build_plan()
    }

    pub fn surface_bouncy(mut self) -> FabricPlan {
        self.surface = Some(SurfaceCharacter::Bouncy);
        self.build_plan()
    }

    pub fn surface_slippery(mut self) -> FabricPlan {
        self.surface = Some(SurfaceCharacter::Slippery);
        self.build_plan()
    }

    pub fn floating(self) -> FabricPlan {
        self.build_plan()
    }
}
