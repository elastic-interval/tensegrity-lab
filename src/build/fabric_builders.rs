/// Pure Rust fabric definitions - replacing Tenscript parsing
///
/// This module contains only the fabric definition functions.
/// All supporting types and helpers are in the `fabric_dsl` module.

use crate::build::fabric_dsl::*;
use crate::build::tenscript::fabric_plan::FabricPlan;
use crate::fabric::physics::SurfaceCharacter;

// ============================================================================
// Fabric Definitions
// ============================================================================

/// Build the Triped fabric using Rust DSL
pub fn triped() -> FabricPlan {
    fabric("Triped")
        .build(
            branch(BrickName::Omni)
                .seed(1)
                .on_face(Face::BotX, grow("XXXXXXXX").scale(0.9).mark("end").prism().build())
                .on_face(Face::BotY, grow("XXXXXXXX").scale(0.9).mark("end").prism().build())
                .on_face(Face::BotZ, grow("XXXXXXXX").scale(0.9).mark("end").prism().build())
                .on_face(Face::Top, grow("X").build())
                .build(),
        )
        .shape([
            during(Sec(15.0), [space("end", 0.35)]),
            during(Sec(15.0), [vulcanize()]),
        ])
        .pretense(pretense().altitude(Mm(1000.0)).surface(SurfaceCharacter::Slippery))
        .converge(Sec(10.0))
        .scale(Mm(1030.0))
        .build_plan()
}

// ============================================================================
// Fabric Library
// ============================================================================

/// Build the Symmetrical fabric using Rust DSL
pub fn symmetrical() -> FabricPlan {
    fabric("Symmetrical")
        .build(branch(BrickName::Omni).build())
        .shape([centralize_at(1.0)])
        .pretense(pretense().surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the Vertebra fabric using Rust DSL
pub fn vertebra() -> FabricPlan {
    fabric("Vertebra")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow("X").build())
                .build(),
        )
        .shape([centralize_at(1.0)])
        .pretense(pretense().surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the Flagellum fabric using Rust DSL
pub fn flagellum() -> FabricPlan {
    fabric("Flagellum")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow("XXXXXXXXXXXXXXXXXXXX").scale(0.95).build())
                .build(),
        )
        .shape([vulcanize()])
        .pretense(pretense().surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Cigar fabric using Rust DSL
pub fn cigar() -> FabricPlan {
    fabric("Cigar")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow("X").scale(0.85).build())
                .on_face(Face::Base, grow("X").scale(0.85).build())
                .build(),
        )
        .shape([centralize_at(1.0), during(Sec(40000.0), [vulcanize()])])
        .pretense(pretense().surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the X fabric using Rust DSL
pub fn x() -> FabricPlan {
    fabric("X")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow("X").scale(0.8).build())
                .on_face(Face::Base, grow("X").scale(0.8).build())
                .build(),
        )
        .shape([centralize_at(1.0), during(Sec(40000.0), [vulcanize()])])
        .pretense(pretense().surface(SurfaceCharacter::Frozen))
        .scale(Mm(430.0))
        .build_plan()
}

/// Build the Tetrapod fabric using Rust DSL
pub fn tetrapod() -> FabricPlan {
    fabric("Tetrapod")
        .build(
            branch(BrickName::Omni)
                .on_face(Face::TopRight, grow("XXX").scale(0.9).build())
                .on_face(Face::BottomRight, grow("XXX").scale(0.9).build())
                .on_face(Face::BackLeft, grow("XXX").scale(0.9).build())
                .on_face(Face::FrontLeft, grow("XXX").scale(0.9).build())
                .build(),
        )
        .shape([vulcanize()])
        .pretense(pretense().surface(SurfaceCharacter::Bouncy))
        .scale(Mm(46.0))
        .build_plan()
}

/// Build the Halo by Crane fabric using Rust DSL
pub fn halo_by_crane() -> FabricPlan {
    fabric("Halo by Crane")
        .build(
            branch(BrickName::Single)
                .rotate()
                .rotate()
                .on_face(
                    Face::NextBase,
                    grow("XXXX")
                        .scale(0.92)
                        .build_node(
                            branch(BrickName::Omni)
                                .on_face(Face::TopX, grow("XXXXXXXXXXXX").scale(0.92).mark("halo-end").build())
                                .on_face(Face::TopY, grow("XXXXXXXXXXX").scale(0.92).mark("halo-end").build())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .shape([join("halo-end"), vulcanize()])
        .pretense(pretense().surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Convergence fabric using Rust DSL
pub fn convergence() -> FabricPlan {
    fabric("Convergence")
        .build(
            branch(BrickName::Omni)
                .seed(1)
                .on_face(Face::Bot, grow("XX").scale(0.9).build())
                .on_face(Face::TopY, grow("XXXXXXXXXX").scale(0.9).mark("end").build())
                .on_face(Face::TopX, grow("XXXXXXXXXX").scale(0.9).mark("end").build())
                .on_face(Face::TopZ, grow("XXXXXXXXXX").scale(0.9).mark("end").build())
                .build(),
        )
        .shape([
            during(Sec(18000.0), [join_seed("end", 1)]),
            during(Sec(20000.0), [vulcanize()]),
            centralize_at(5.0),
        ])
        .pretense(pretense().surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Torque Walker fabric using Rust DSL
pub fn torque_walker() -> FabricPlan {
    fabric("Torque Walker")
        .build(
            branch(BrickName::Torque)
                .on_face(
                    Face::LeftFrontBottom,
                    grow("X")
                        .build_node(branch(BrickName::TorqueLeft).build())
                        .build(),
                )
                .on_face(
                    Face::LeftBackBottom,
                    grow("X")
                        .build_node(branch(BrickName::TorqueRight).build())
                        .build(),
                )
                .on_face(
                    Face::RightFrontBottom,
                    grow("X")
                        .build_node(branch(BrickName::TorqueLeft).build())
                        .build(),
                )
                .on_face(
                    Face::RightBackBottom,
                    grow("X")
                        .build_node(branch(BrickName::TorqueRight).build())
                        .build(),
                )
                .build(),
        )
        .shape([])
        .pretense(pretense().surface(SurfaceCharacter::Bouncy))
        .build_plan()
}

/// Build the Twisted Infinity fabric using Rust DSL
pub fn twisted_infinity() -> FabricPlan {
    fabric("Twisted Infinity")
        .build(
            branch(BrickName::Omni)
                .on_face(Face::TopRight, grow("XXXXXX").scale(0.83).mark("ring-a").build())
                .on_face(Face::BottomRight, grow("XXXXX").scale(0.83).mark("ring-a").build())
                .on_face(Face::BackLeft, grow("XXXXXX").scale(0.83).mark("ring-b").build())
                .on_face(Face::FrontLeft, grow("XXXXX").scale(0.83).mark("ring-b").build())
                .build(),
        )
        .shape([
            during(Sec(20000.0), [join("ring-a"), join("ring-b")]),
            during(Sec(5000.0), [vulcanize()]),
            centralize_at(1.0),
        ])
        .pretense(pretense().surface(SurfaceCharacter::Absent).rigidity(0.1))
        .scale(Mm(60.0))
        .build_plan()
}

/// Build the Propellor fabric using Rust DSL
pub fn propellor() -> FabricPlan {
    fabric("Propellor")
        .build(
            branch(BrickName::Omni)
                .seed(1)
                .on_face(Face::TopX, grow("XXXXXX").scale(0.83).mark("ring-x").build())
                .on_face(Face::BotY, grow("XXXXX").scale(0.83).mark("ring-x").build())
                .on_face(Face::TopY, grow("XXXXXX").scale(0.83).mark("ring-y").build())
                .on_face(Face::BotZ, grow("XXXXX").scale(0.83).mark("ring-y").build())
                .on_face(Face::TopZ, grow("XXXXXX").scale(0.83).mark("ring-z").build())
                .on_face(Face::BotX, grow("XXXXX").scale(0.83).mark("ring-z").build())
                .build(),
        )
        .shape([
            during(Sec(15000.0), [join("ring-x"), join("ring-y"), join("ring-z")]),
            during(Sec(40000.0), [vulcanize()]),
            centralize_at(1.0),
        ])
        .pretense(pretense().surface(SurfaceCharacter::Absent))
        .build_plan()
}

/// Build the Headless Hug fabric using Rust DSL
pub fn headless_hug() -> FabricPlan {
    fabric("Headless Hug")
        .build(
            branch(BrickName::Omni)
                .on_face(Face::BottomLeft, grow("....X..").scale(0.92).mark("legs").build())
                .on_face(Face::BottomRight, grow("....X..").scale(0.92).mark("legs").build())
                .on_face(
                    Face::TopLeft,
                    grow("XX")
                        .scale(0.9)
                        .build_node(
                            branch(BrickName::Omni)
                                .on_face(Face::TopZ, grow_mark("chest-1"))
                                .on_face(Face::BotX, grow_mark("chest-2"))
                                .on_face(Face::BotY, grow("...X..").scale(0.9).mark("hands").build())
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    Face::TopRight,
                    grow("XX")
                        .scale(0.9)
                        .build_node(
                            branch(BrickName::Omni)
                                .on_face(Face::TopY, grow_mark("chest-1"))
                                .on_face(Face::BotZ, grow_mark("chest-2"))
                                .on_face(Face::BotX, grow("...X..").scale(0.9).mark("hands").build())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .shape([
            during(Sec(6.0), [space("legs", 0.4), space("hands", 0.2), space("chest-2", 0.4)]),
            during(Sec(6.0), [vulcanize()]),
            centralize_at(1.0),
        ])
        .pretense(pretense().surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Torque Ring fabric using Rust DSL
pub fn torque_ring() -> FabricPlan {
    fabric("Torque Ring")
        .build(
            branch(BrickName::Torque)
                .on_face(
                    Face::LeftFrontBottom,
                    Node::Branch {
                        alias: BrickName::Torque.into(),
                        rotation: 2,
                        scale_factor: 1.0,
                        seed: None,
                        face_nodes: vec![Node::Face {
                            alias: Face::FarSide.into(),
                            node: Box::new(Node::Mark {
                                mark_name: ":loose".to_string(),
                            }),
                        }],
                    },
                )
                .on_face(
                    Face::RightFrontBottom,
                    Node::Branch {
                        alias: BrickName::Torque.into(),
                        rotation: 2,
                        scale_factor: 1.0,
                        seed: None,
                        face_nodes: vec![Node::Face {
                            alias: Face::FarSide.into(),
                            node: Box::new(Node::Branch {
                                alias: BrickName::Torque.into(),
                                rotation: 0,
                                scale_factor: 1.0,
                                seed: None,
                                face_nodes: vec![Node::Face {
                                    alias: Face::FarSide.into(),
                                    node: Box::new(Node::Mark {
                                        mark_name: ":loose".to_string(),
                                    }),
                                }],
                            }),
                        }],
                    },
                )
                .build(),
        )
        .shape([join("loose"), centralize()])
        .pretense(pretense().surface(SurfaceCharacter::Bouncy))
        .build_plan()
}

/// Build all fabric plans using Rust DSL
pub fn build_fabric_library() -> Vec<FabricPlan> {
    vec![
        triped(),
        symmetrical(),
        vertebra(),
        flagellum(),
        cigar(),
        x(),
        tetrapod(),
        halo_by_crane(),
        convergence(),
        torque_walker(),
        twisted_infinity(),
        propellor(),
        headless_hug(),
        torque_ring(),
        // De Twips is skipped for now (very complex with many shape operations)
    ]
}
