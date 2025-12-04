use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceName::*, MarkName::*};
use crate::build::dsl::fabric_dsl::*;
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::fabric::physics::SurfaceCharacter;
use crate::units::Amplitude;
use std::sync::OnceLock;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

static TRIPED: OnceLock<FabricPlan> = OnceLock::new();
static VERTEBRA: OnceLock<FabricPlan> = OnceLock::new();
static FLAGELLUM: OnceLock<FabricPlan> = OnceLock::new();
static CIGAR: OnceLock<FabricPlan> = OnceLock::new();
static HALO_BY_CRANE: OnceLock<FabricPlan> = OnceLock::new();
static HEADLESS_HUG: OnceLock<FabricPlan> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, EnumIter)]
pub enum FabricName {
    Triped,
    Vertebra,
    Flagellum,
    Cigar,
    HaloByCrane,
    HeadlessHug,
}

pub fn triped() -> FabricPlan {
    fabric("Triped", Mm(7500.0))
        .build(
            branching(OmniSymmetrical, Seed(1))
                .on_face(OmniBotX, growing(8).scale(0.9).mark(End).prism().build())
                .on_face(OmniBotY, growing(8).scale(0.9).mark(End).prism().build())
                .on_face(OmniBotZ, growing(8).scale(0.9).mark(End).prism().build())
                .on_face(OmniTop, growing(1).build())
                .build(),
        )
        .shape([
            during(Sec(5.0), [space(End, 0.38)]),
            during(Sec(1.0), [vulcanize()]),
        ])
        .pretense(
            pretense(Sec(1.0))
                .surface(SurfaceCharacter::Frozen),
        )
        .fall(Sec(8.0))
        .settle(Sec(3.0))
        .animate(
            Sec(0.8266),
            Amplitude::new(0.01),
            vec![
                MuscleSpec::Alpha.between(151, 48),
                MuscleSpec::Alpha.between(157, 36),
                MuscleSpec::Alpha.between(145, 42),
            ],
        )
        .scale(Mm(1030.0))
        .build_plan()
}

pub fn halo_by_crane() -> FabricPlan {
    fabric("Halo by Crane", Mm(2000.0))
        .build(
            branching(SingleTwistLeft, Seed(1))
                .on_face(
                    SingleTop,
                    growing(4)
                        .scale(0.92)
                        .build_node(
                            branching(OmniSymmetrical, OnSpinLeft)
                                .on_face(OmniTopX, growing(12).scale(0.92).mark(HaloEnd).build())
                                .on_face(OmniTopY, growing(11).scale(0.92).mark(HaloEnd).build())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .shape([
            during(Sec(10.0), [join(HaloEnd)]),
            during(Sec(5.0), [vulcanize()]),
        ])
        .pretense(pretense(Sec(10.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

pub fn vertebra() -> FabricPlan {
    fabric("Vertebra", Mm(500.0))
        .build(
            branching(SingleTwistLeft, Seed(1))
                .on_face(SingleTop, growing(1).build())
                .build(),
        )
        .shape([centralize_at(1.0)])
        .pretense(pretense(Sec(10.0)))
        .scale(Mm(74.6))
        .build_plan()
}

pub fn flagellum() -> FabricPlan {
    fabric("Flagellum", Mm(2000.0))
        .build(
            branching(SingleTwistLeft, Seed(1))
                .on_face(SingleTop, growing(20).scale(0.95).build())
                .build(),
        )
        .shape([vulcanize()])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

pub fn cigar() -> FabricPlan {
    fabric("Cigar", Mm(500.0))
        .build(
            branching(SingleTwistLeft, Seed(1))
                .on_face(SingleTop, growing(1).scale(0.85).build())
                .on_face(SingleBot, growing(1).scale(0.85).build())
                .build(),
        )
        .shape([centralize_at(1.0), during(Sec(40000.0), [vulcanize()])])
        .pretense(pretense(Sec(15.0)))
        .scale(Mm(74.6))
        .build_plan()
}

pub fn headless_hug() -> FabricPlan {
    fabric("Headless Hug", Mm(2000.0))
        .build(
            branching(OmniSymmetrical, Seed(4))
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
                            branching(OmniSymmetrical, OnSpinRight)
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
                            branching(OmniSymmetrical, OnSpinLeft)
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

pub fn get_fabric_plan(fabric_name: FabricName) -> FabricPlan {
    match fabric_name {
        FabricName::Triped => TRIPED.get_or_init(triped),
        FabricName::Vertebra => VERTEBRA.get_or_init(vertebra),
        FabricName::Flagellum => FLAGELLUM.get_or_init(flagellum),
        FabricName::Cigar => CIGAR.get_or_init(cigar),
        FabricName::HaloByCrane => HALO_BY_CRANE.get_or_init(halo_by_crane),
        FabricName::HeadlessHug => HEADLESS_HUG.get_or_init(headless_hug),
    }
    .clone()
}

pub fn all_fabric_plans() -> impl Iterator<Item = FabricPlan> {
    FabricName::iter().map(get_fabric_plan)
}
