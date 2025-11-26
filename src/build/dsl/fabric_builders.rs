use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceName::*, MarkName::*};
use crate::build::dsl::fabric_dsl::*;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::fabric::physics::SurfaceCharacter;

/// Build the Triped fabric
pub fn triped() -> FabricPlan {
    fabric("Triped")
        .build(
            branching(OmniBrick, SeedFaceDown)
                .on_face(OmniBotX, growing(8).scale(0.9).mark(End).prism().build())
                .on_face(OmniBotY, growing(8).scale(0.9).mark(End).prism().build())
                .on_face(OmniBotZ, growing(8).scale(0.9).mark(End).prism().build())
                .on_face(OmniTop, growing(1).build())
                .build(),
        )
        .shape([
            during(Sec(25.0), [space(End, 0.38)]),
            during(Sec(15.0), [vulcanize()]),
        ])
        .pretense(
            pretense(Sec(15.0))
                .altitude(Mm(1000.0))
                .surface(SurfaceCharacter::Slippery),
        )
        .converge(Sec(10.0))
        .scale(Mm(1030.0))
        .build_plan()
}

/// Build the Vertebra fabric
pub fn vertebra() -> FabricPlan {
    fabric("Vertebra")
        .build(
            branching(SingleLeftBrick, Seed)
                .on_face(SingleTop, growing(1).build())
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
            branching(SingleLeftBrick, Seed)
                .on_face(SingleTop, growing(20).scale(0.95).build())
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
            branching(SingleLeftBrick, Seed)
                .on_face(SingleTop, growing(1).scale(0.85).build())
                .on_face(SingleBot, growing(1).scale(0.85).build())
                .build(),
        )
        .shape([centralize_at(1.0), during(Sec(40000.0), [vulcanize()])])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Absent))
        .scale(Mm(74.6))
        .build_plan()
}

/// Build the Halo by Crane fabric
pub fn halo_by_crane() -> FabricPlan {
    fabric("Halo by Crane")
        .build(
            branching(SingleLeftBrick, Seed)
                .on_face(
                    SingleTop,
                    growing(4)
                        .scale(0.92)
                        .build_node(
                            branching(OmniBrick, OnSpinRight)
                                .on_face(OmniTopX, growing(12).scale(0.92).mark(HaloEnd).build())
                                .on_face(OmniTopY, growing(11).scale(0.92).mark(HaloEnd).build())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .shape([join(HaloEnd), vulcanize()])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Convergence fabric
pub fn convergence() -> FabricPlan {
    fabric("Convergence")
        .build(
            branching(OmniBrick, SeedFaceDown)
                .on_face(OmniBot, growing(2).scale(0.9).build())
                .on_face(OmniTopY, growing(10).scale(0.9).mark(End).build())
                .on_face(OmniTopX, growing(10).scale(0.9).mark(End).build())
                .on_face(OmniTopZ, growing(10).scale(0.9).mark(End).build())
                .build(),
        )
        .shape([
            during(Sec(18000.0), [join_seed(End, 1)]),
            during(Sec(20000.0), [vulcanize()]),
            centralize_at(5.0),
        ])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build the Headless Hug fabric
pub fn headless_hug() -> FabricPlan {
    fabric("Headless Hug")
        .build(
            branching(OmniBrick, SeedFourDown)
                .on_face(
                    LeftBackBottom,
                    growing(4)
                        .chiral()
                        .scale(0.92)
                        .build_node(
                            growing(1)
                                .build_node(growing(2).chiral().mark(Legs).build())
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    RightBackBottom,
                    growing(4)
                        .chiral()
                        .scale(0.92)
                        .build_node(
                            growing(1)
                                .build_node(growing(2).chiral().mark(Legs).build())
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    LeftFrontTop,
                    growing(2)
                        .scale(0.9)
                        .build_node(
                            branching(OmniBrick, OnSpinRight)
                                .on_face(OmniTopZ, grow_mark(Chest1))
                                .on_face(OmniBotX, grow_mark(Chest2))
                                .on_face(
                                    OmniBotY,
                                    growing(3)
                                        .chiral()
                                        .scale(0.9)
                                        .build_node(
                                            growing(1)
                                                .build_node(growing(2).chiral().mark(Hands).build())
                                                .build(),
                                        )
                                        .build(),
                                )
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    RightFrontTop,
                    growing(2)
                        .scale(0.9)
                        .build_node(
                            branching(OmniBrick, OnSpinLeft)
                                .on_face(OmniTopY, grow_mark(Chest1))
                                .on_face(OmniBotZ, grow_mark(Chest2))
                                .on_face(
                                    OmniBotX,
                                    growing(3)
                                        .chiral()
                                        .scale(0.9)
                                        .build_node(
                                            growing(1)
                                                .build_node(growing(2).chiral().mark(Hands).build())
                                                .build(),
                                        )
                                        .build(),
                                )
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .shape([
            during(
                Sec(6.0),
                [space(Legs, 0.4), space(Hands, 0.2), space(Chest2, 0.4)],
            ),
            during(Sec(6.0), [vulcanize()]),
            centralize_at(1.0),
        ])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

/// Build all fabric plans
pub fn build_fabric_library() -> Vec<FabricPlan> {
    vec![
        triped(),
        vertebra(),
        flagellum(),
        cigar(),
        halo_by_crane(),
        convergence(),
        headless_hug(),
    ]
}
