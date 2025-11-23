/// Fabric definitions using the type-safe Rust DSL.
///
/// All supporting types and helpers are in the `fabric_dsl` module.

use crate::build::dsl::fabric_dsl::*;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::fabric::physics::SurfaceCharacter;

/// Build the Triped fabric
pub fn triped() -> FabricPlan {
    fabric("Triped")
        .build(
            branch(BrickName::Omni)
                .seed(1)
                .on_face(Face::BotX, grow(8).scale(0.9).mark("end").prism().build())
                .on_face(Face::BotY, grow(8).scale(0.9).mark("end").prism().build())
                .on_face(Face::BotZ, grow(8).scale(0.9).mark("end").prism().build())
                .on_face(Face::Top, grow(1).build())
                .build(),
        )
        .shape([
            during(Sec(25.0), [space("end", 0.38)]),
            during(Sec(15.0), [vulcanize()]),
        ])
        .pretense(pretense(Sec(15.0)).altitude(Mm(1000.0)).surface(SurfaceCharacter::Slippery))
        .converge(Sec(10.0))
        .scale(Mm(1030.0))
        .build_plan()
}

/// Build the Symmetrical fabric
pub fn symmetrical() -> FabricPlan {
    fabric("Symmetrical")
        .build(branch(BrickName::Omni).build())
        .shape([centralize_at(1.0)])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the Vertebra fabric
pub fn vertebra() -> FabricPlan {
    fabric("Vertebra")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow(1).build())
                .build(),
        )
        .shape([centralize_at(1.0)])
        .pretense(pretense(Sec(10.0)).surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the Flagellum fabric
pub fn flagellum() -> FabricPlan {
    fabric("Flagellum")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow(20).scale(0.95).build())
                .build(),
        )
        .shape([vulcanize()])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Cigar fabric
pub fn cigar() -> FabricPlan {
    fabric("Cigar")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow(1).scale(0.85).build())
                .on_face(Face::Base, grow(1).scale(0.85).build())
                .build(),
        )
        .shape([centralize_at(1.0), during(Sec(40000.0), [vulcanize()])])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the X fabric
pub fn x() -> FabricPlan {
    fabric("X")
        .build(
            branch(BrickName::Single)
                .on_face(Face::NextBase, grow(1).scale(0.8).build())
                .on_face(Face::Base, grow(1).scale(0.8).build())
                .build(),
        )
        .shape([centralize_at(1.0), during(Sec(40000.0), [vulcanize()])])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .scale(Mm(430.0))
        .build_plan()
}

/// Build the Tetrapod fabric
pub fn tetrapod() -> FabricPlan {
    fabric("Tetrapod")
        .build(
            branch(BrickName::Omni)
                .on_face(Face::TopRight, grow(3).scale(0.9).build())
                .on_face(Face::BottomRight, grow(3).scale(0.9).build())
                .on_face(Face::BackLeft, grow(3).scale(0.9).build())
                .on_face(Face::FrontLeft, grow(3).scale(0.9).build())
                .build(),
        )
        .shape([vulcanize()])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Bouncy))
        .scale(Mm(46.0))
        .build_plan()
}

/// Build the Halo by Crane fabric
pub fn halo_by_crane() -> FabricPlan {
    fabric("Halo by Crane")
        .build(
            branch(BrickName::Single)
                .rotate()
                .rotate()
                .on_face(
                    Face::NextBase,
                    grow(4)
                        .scale(0.92)
                        .build_node(
                            branch(BrickName::Omni)
                                .on_face(Face::TopX, grow(12).scale(0.92).mark("halo-end").build())
                                .on_face(Face::TopY, grow(11).scale(0.92).mark("halo-end").build())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .shape([join("halo-end"), vulcanize()])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Convergence fabric
pub fn convergence() -> FabricPlan {
    fabric("Convergence")
        .build(
            branch(BrickName::Omni)
                .seed(1)
                .on_face(Face::Bot, grow(2).scale(0.9).build())
                .on_face(Face::TopY, grow(10).scale(0.9).mark("end").build())
                .on_face(Face::TopX, grow(10).scale(0.9).mark("end").build())
                .on_face(Face::TopZ, grow(10).scale(0.9).mark("end").build())
                .build(),
        )
        .shape([
            during(Sec(18000.0), [join_seed("end", 1)]),
            during(Sec(20000.0), [vulcanize()]),
            centralize_at(5.0),
        ])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Torque Walker fabric
pub fn torque_walker() -> FabricPlan {
    fabric("Torque Walker")
        .build(
            branch(BrickName::Torque)
                .on_face(
                    Face::LeftFrontBottom,
                    grow(1)
                        .build_node(branch(BrickName::TorqueLeft).build())
                        .build(),
                )
                .on_face(
                    Face::LeftBackBottom,
                    grow(1)
                        .build_node(branch(BrickName::TorqueRight).build())
                        .build(),
                )
                .on_face(
                    Face::RightFrontBottom,
                    grow(1)
                        .build_node(branch(BrickName::TorqueLeft).build())
                        .build(),
                )
                .on_face(
                    Face::RightBackBottom,
                    grow(1)
                        .build_node(branch(BrickName::TorqueRight).build())
                        .build(),
                )
                .build(),
        )
        .shape([])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Bouncy))
        .build_plan()
}

/// Build the Twisted Infinity fabric
pub fn twisted_infinity() -> FabricPlan {
    fabric("Twisted Infinity")
        .build(
            branch(BrickName::Omni)
                .on_face(Face::TopRight, grow(6).scale(0.83).mark("ring-a").build())
                .on_face(Face::BottomRight, grow(5).scale(0.83).mark("ring-a").build())
                .on_face(Face::BackLeft, grow(6).scale(0.83).mark("ring-b").build())
                .on_face(Face::FrontLeft, grow(5).scale(0.83).mark("ring-b").build())
                .build(),
        )
        .shape([
            during(Sec(20000.0), [join("ring-a"), join("ring-b")]),
            during(Sec(5000.0), [vulcanize()]),
            centralize_at(1.0),
        ])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Absent).rigidity(0.1))
        .scale(Mm(60.0))
        .build_plan()
}

/// Build the Propellor fabric
pub fn propellor() -> FabricPlan {
    fabric("Propellor")
        .build(
            branch(BrickName::Omni)
                .seed(1)
                .on_face(Face::TopX, grow(6).scale(0.83).mark("ring-x").build())
                .on_face(Face::BotY, grow(5).scale(0.83).mark("ring-x").build())
                .on_face(Face::TopY, grow(6).scale(0.83).mark("ring-y").build())
                .on_face(Face::BotZ, grow(5).scale(0.83).mark("ring-y").build())
                .on_face(Face::TopZ, grow(6).scale(0.83).mark("ring-z").build())
                .on_face(Face::BotX, grow(5).scale(0.83).mark("ring-z").build())
                .build(),
        )
        .shape([
            during(Sec(15000.0), [join("ring-x"), join("ring-y"), join("ring-z")]),
            during(Sec(40000.0), [vulcanize()]),
            centralize_at(1.0),
        ])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Absent))
        .build_plan()
}

/// Build the Headless Hug fabric
pub fn headless_hug() -> FabricPlan {
    fabric("Headless Hug")
        .build(
            branch(BrickName::Omni)
                .on_face(Face::BottomLeft, grow(4).chiral().scale(0.92).build_node(grow(1).build_node(grow(2).chiral().mark("legs").build()).build()).build())
                .on_face(Face::BottomRight, grow(4).chiral().scale(0.92).build_node(grow(1).build_node(grow(2).chiral().mark("legs").build()).build()).build())
                .on_face(
                    Face::TopLeft,
                    grow(2)
                        .scale(0.9)
                        .build_node(
                            branch(BrickName::Omni)
                                .on_face(Face::TopZ, grow_mark("chest-1"))
                                .on_face(Face::BotX, grow_mark("chest-2"))
                                .on_face(Face::BotY, grow(3).chiral().scale(0.9).build_node(grow(1).build_node(grow(2).chiral().mark("hands").build()).build()).build())
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    Face::TopRight,
                    grow(2)
                        .scale(0.9)
                        .build_node(
                            branch(BrickName::Omni)
                                .on_face(Face::TopY, grow_mark("chest-1"))
                                .on_face(Face::BotZ, grow_mark("chest-2"))
                                .on_face(Face::BotX, grow(3).chiral().scale(0.9).build_node(grow(1).build_node(grow(2).chiral().mark("hands").build()).build()).build())
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
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Torque Ring fabric
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
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Bouncy))
        .build_plan()
}

/// Build all fabric plans
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
