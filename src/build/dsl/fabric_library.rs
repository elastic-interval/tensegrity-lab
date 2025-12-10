use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceName::*, MarkName::*};
use crate::build::dsl::fabric_dsl::*;
use crate::build::dsl::fabric_plan::FabricPlan;
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
    fabric("Triped")
        .altitude(M(7.5))
        .scale(M(1.03))
        .seed(OmniSymmetrical, Seed(1))
        .on_face(OmniBotX, column(8).shrink_by(Pct(10.0)).mark(End).prism().build())
        .on_face(OmniBotY, column(8).shrink_by(Pct(10.0)).mark(End).prism().build())
        .on_face(OmniBotZ, column(8).shrink_by(Pct(10.0)).mark(End).prism().build())
        .on_face(OmniTop, column(1).build())
        .space(Sec(3.0), End, Pct(38.0))
        .vulcanize(Sec(1.0))
        .pretense(Sec(1.0))
        .surface_frozen()
        .fall(Sec(3.0))
        .settle(Sec(3.0))
        .animate_pulse(
            Sec(0.8266),
            Pct(1.0),
            0.1,
            Pct(10.0),
            vec![
                ActuatorSpec::Alpha.between(151, 48),
                ActuatorSpec::Alpha.between(157, 36),
                ActuatorSpec::Alpha.between(145, 42),
            ],
        )
}

pub fn halo_by_crane() -> FabricPlan {
    fabric("Halo by Crane")
        .altitude(M(2.0))
        .scale(M(1.0))
        .seed(SingleTwistLeft, Seed(1))
        .on_face(
            SingleTop,
            column(4)
                .shrink_by(Pct(8.0))
                .build_node(
                    hub(OmniSymmetrical, OnSpinLeft)
                        .on_face(OmniTopX, column(12).shrink_by(Pct(8.0)).mark(HaloEnd).build())
                        .on_face(OmniTopY, column(11).shrink_by(Pct(8.0)).mark(HaloEnd).build())
                        .build(),
                )
                .build(),
        )
        .join(Sec(10.0), HaloEnd)
        .vulcanize(Sec(5.0))
        .pretense(Sec(10.0))
        .surface_frozen()
}

pub fn vertebra() -> FabricPlan {
    fabric("Vertebra")
        .altitude(M(0.5))
        .scale(M(0.0746))
        .seed(SingleTwistLeft, Seed(1))
        .on_face(SingleTop, column(1).build())
        .centralize_at(Sec(1.0), M(0.075))
        .pretense(Sec(10.0))
        .floating()
}

pub fn flagellum() -> FabricPlan {
    fabric("Flagellum")
        .altitude(M(2.0))
        .scale(M(1.0))
        .seed(SingleTwistLeft, Seed(1))
        .on_face(SingleTop, column(20).shrink_by(Pct(5.0)).build())
        .vulcanize(Sec(1.0))
        .pretense(Sec(15.0))
        .surface_frozen()
}

pub fn cigar() -> FabricPlan {
    fabric("Cigar")
        .altitude(M(0.5))
        .scale(M(0.0746))
        .seed(SingleTwistLeft, Seed(1))
        .on_face(SingleTop, column(1).shrink_by(Pct(15.0)).build())
        .on_face(SingleBot, column(1).shrink_by(Pct(15.0)).build())
        .centralize_at(Sec(1.0), M(0.075))
        .vulcanize(Sec(40000.0))
        .pretense(Sec(15.0))
        .floating()
}

pub fn headless_hug() -> FabricPlan {
    fabric("Headless Hug")
        .altitude(M(2.0))
        .scale(M(1.0))
        .seed(OmniSymmetrical, Seed(4))
        .on_face(
            LeftBackBottom,
            column(4)
                .chiral()
                .shrink_by(Pct(8.0))
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
                .shrink_by(Pct(8.0))
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
                .shrink_by(Pct(10.0))
                .build_node(
                    hub(OmniSymmetrical, OnSpinRight)
                        .on_face(OmniTopZ, mark(Chest1))
                        .on_face(OmniBotX, mark(Chest2))
                        .on_face(
                            OmniBotY,
                            column(3)
                                .chiral()
                                .shrink_by(Pct(10.0))
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
                .shrink_by(Pct(10.0))
                .build_node(
                    hub(OmniSymmetrical, OnSpinLeft)
                        .on_face(OmniTopY, mark(Chest1))
                        .on_face(OmniBotZ, mark(Chest2))
                        .on_face(
                            OmniBotX,
                            column(3)
                                .chiral()
                                .shrink_by(Pct(10.0))
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
        .space(Sec(2.0), Legs, Pct(40.0))
        .space(Sec(2.0), Hands, Pct(20.0))
        .space(Sec(2.0), Chest2, Pct(40.0))
        .vulcanize(Sec(6.0))
        .centralize_at(Sec(1.0), M(1.0))
        .pretense(Sec(15.0))
        .surface_frozen()
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
