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
            seed(OmniSymmetrical, Seed(1))
                .on_face(OmniBotX, column(8).scale(Pct(90.0)).mark(End).prism().build())
                .on_face(OmniBotY, column(8).scale(Pct(90.0)).mark(End).prism().build())
                .on_face(OmniBotZ, column(8).scale(Pct(90.0)).mark(End).prism().build())
                .on_face(OmniTop, column(1).build())
                .build(),
        )
        .shape([
            during(Sec(3.0), [space(End, Pct(38.0))]),
            during(Sec(1.0), [vulcanize()]),
        ])
        .pretense(
            pretense(Sec(1.0))
                .surface(SurfaceCharacter::Frozen),
        )
        .fall(Sec(3.0))
        .settle(Sec(3.0))
        .animate_pulse(
            Sec(0.8266),
            Amplitude::new(0.01),
            0.1,
            Pct(10.0),
            vec![
                ActuatorSpec::Alpha.between(151, 48),
                ActuatorSpec::Alpha.between(157, 36),
                ActuatorSpec::Alpha.between(145, 42),
            ],
        )
        .scale(Mm(1030.0))
        .build_plan()
}

pub fn halo_by_crane() -> FabricPlan {
    fabric("Halo by Crane", Mm(2000.0))
        .build(
            seed(SingleTwistLeft, Seed(1))
                .on_face(
                    SingleTop,
                    column(4)
                        .scale(Pct(92.0))
                        .build_node(
                            hub(OmniSymmetrical, OnSpinLeft)
                                .on_face(OmniTopX, column(12).scale(Pct(92.0)).mark(HaloEnd).build())
                                .on_face(OmniTopY, column(11).scale(Pct(92.0)).mark(HaloEnd).build())
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
            seed(SingleTwistLeft, Seed(1))
                .on_face(SingleTop, column(1).build())
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
            seed(SingleTwistLeft, Seed(1))
                .on_face(SingleTop, column(20).scale(Pct(95.0)).build())
                .build(),
        )
        .shape([vulcanize()])
        .pretense(pretense(Sec(15.0)).surface(SurfaceCharacter::Frozen))
        .build_plan()
}

pub fn cigar() -> FabricPlan {
    fabric("Cigar", Mm(500.0))
        .build(
            seed(SingleTwistLeft, Seed(1))
                .on_face(SingleTop, column(1).scale(Pct(85.0)).build())
                .on_face(SingleBot, column(1).scale(Pct(85.0)).build())
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
            seed(OmniSymmetrical, Seed(4))
                .on_face(
                    LeftBackBottom,
                    column(4)
                        .chiral()
                        .scale(Pct(92.0))
                        .build_node(
                            column(1)
                                .build_node(column(2).chiral().mark(Legs).build())
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    RightBackBottom,
                    column(4)
                        .chiral()
                        .scale(Pct(92.0))
                        .build_node(
                            column(1)
                                .build_node(column(2).chiral().mark(Legs).build())
                                .build(),
                        )
                        .build(),
                )
                .on_face(
                    LeftFrontTop,
                    column(2)
                        .scale(Pct(90.0))
                        .build_node(
                            hub(OmniSymmetrical, OnSpinRight)
                                .on_face(OmniTopZ, mark(Chest1))
                                .on_face(OmniBotX, mark(Chest2))
                                .on_face(
                                    OmniBotY,
                                    column(3)
                                        .chiral()
                                        .scale(Pct(90.0))
                                        .build_node(
                                            column(1)
                                                .build_node(column(2).chiral().mark(Hands).build())
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
                    column(2)
                        .scale(Pct(90.0))
                        .build_node(
                            hub(OmniSymmetrical, OnSpinLeft)
                                .on_face(OmniTopY, mark(Chest1))
                                .on_face(OmniBotZ, mark(Chest2))
                                .on_face(
                                    OmniBotX,
                                    column(3)
                                        .chiral()
                                        .scale(Pct(90.0))
                                        .build_node(
                                            column(1)
                                                .build_node(column(2).chiral().mark(Hands).build())
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
                [space(Legs, Pct(40.0)), space(Hands, Pct(20.0)), space(Chest2, Pct(40.0))],
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
