use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceName::*, MarkName::*};
use crate::build::dsl::fabric_dsl::{on, *};
use crate::build::dsl::fabric_plan::FabricPlan;
use std::sync::OnceLock;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

static PLANS: [OnceLock<FabricPlan>; 6] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, EnumIter)]
pub enum FabricName {
    Triped,
    #[strum(serialize = "Triped Model")]
    TripedModel,
    Mockup,
    Vertebra,
    Flagellum,
    #[strum(serialize = "Halo by Crane")]
    HaloByCrane,
    #[strum(serialize = "Headless Hug")]
    HeadlessHug,
}

impl FabricName {
    pub fn fabric_plan(self) -> FabricPlan {
        use FabricName::*;
        match self {
            Triped | TripedModel => self
                .build(if self == Triped {
                    FabricDimensions::full_size()
                } else {
                    FabricDimensions::model_size()
                })
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX)
                        .column(8)
                        .shrink_by(Pct(10.0))
                        .mark(End)
                        .prism(),
                    on(OmniBotY)
                        .column(8)
                        .shrink_by(Pct(10.0))
                        .mark(End)
                        .prism(),
                    on(OmniBotZ)
                        .column(8)
                        .shrink_by(Pct(10.0))
                        .mark(End)
                        .prism(),
                    on(OmniTop).prism(),
                    on(OmniBot).radial(),
                ])
                .omit([(6, 9), (6, 3), (2, 11), (2, 5), (1, 10), (10, 7)])
                .space(Sec(3.0), End, Pct(38.0))
                .vulcanize(Sec(1.0))
                .pretense(Sec(1.0))
                .surface_frozen()
                .fall(Sec(2.0))
                .settle(Sec(3.0))
                .animate()
                .period(Sec(0.847))
                .amplitude(Pct(1.0))
                .stiffness(Pct(20.0))
                .sine()
                .actuators([
                    phase(Pct(0.0)).between(217, 66),
                    phase(Pct(0.0)).between(219, 50),
                    phase(Pct(0.0)).between(215, 58),
                ]),

            Mockup => self
                .build(
                    FabricDimensions::full_size()
                        .with_altitude(M(2.0))
                        .with_scale(M(0.583)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(1), on(SingleBot).column(1)])
                .vulcanize(Sec(2.0))
                .pretense(Sec(2.0))
                .surface_frozen()
                .fall(Sec(3.0))
                .settle(Sec(4.0)),

            HaloByCrane => self
                .build(
                    FabricDimensions::full_size()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(4).shrink_by(Pct(8.0)).then(
                    hub(OmniSymmetrical, OnSpinLeft).faces([
                        on(OmniTopX).column(12).shrink_by(Pct(8.0)).mark(HaloEnd),
                        on(OmniTopY).column(11).shrink_by(Pct(8.0)).mark(HaloEnd),
                    ]),
                )])
                .join(Sec(10.0), HaloEnd)
                .vulcanize(Sec(5.0))
                .pretense(Sec(10.0))
                .surface_frozen(),

            Vertebra => self
                .build(
                    FabricDimensions::full_size()
                        .with_altitude(M(0.5))
                        .with_scale(M(0.0746)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(1)])
                .centralize_at(Sec(1.0), M(0.075))
                .pretense(Sec(10.0))
                .floating(),

            Flagellum => self
                .build(
                    FabricDimensions::full_size()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(20).shrink_by(Pct(5.0))])
                .vulcanize(Sec(1.0))
                .pretense(Sec(15.0))
                .surface_frozen(),

            HeadlessHug => self
                .build(
                    FabricDimensions::full_size()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(OmniSymmetrical, Seed(4))
                .faces([
                    on(LeftBackBottom)
                        .column(4)
                        .chiral()
                        .shrink_by(Pct(8.0))
                        .then(column(1).then(column(2).chiral().mark(Legs))),
                    on(RightBackBottom)
                        .column(4)
                        .chiral()
                        .shrink_by(Pct(8.0))
                        .then(column(1).then(column(2).chiral().mark(Legs))),
                    on(LeftFrontTop).column(2).shrink_by(Pct(10.0)).then(
                        hub(OmniSymmetrical, OnSpinRight).faces([
                            on(OmniTopZ).mark(Chest1),
                            on(OmniBotX).mark(Chest2),
                            on(OmniBotY)
                                .column(3)
                                .chiral()
                                .shrink_by(Pct(10.0))
                                .then(column(1).then(column(2).chiral().mark(Hands)))
                                .into(),
                        ]),
                    ),
                    on(RightFrontTop).column(2).shrink_by(Pct(10.0)).then(
                        hub(OmniSymmetrical, OnSpinLeft).faces([
                            on(OmniTopY).mark(Chest1),
                            on(OmniBotZ).mark(Chest2),
                            on(OmniBotX)
                                .column(3)
                                .chiral()
                                .shrink_by(Pct(10.0))
                                .then(column(1).then(column(2).chiral().mark(Hands)))
                                .into(),
                        ]),
                    ),
                ])
                .space(Sec(2.0), Legs, Pct(40.0))
                .space(Sec(2.0), Hands, Pct(20.0))
                .space(Sec(2.0), Chest2, Pct(40.0))
                .vulcanize(Sec(6.0))
                .centralize_at(Sec(1.0), M(1.0))
                .pretense(Sec(15.0))
                .surface_frozen(),
        }
    }
}

pub fn get_fabric_plan(fabric_name: FabricName) -> FabricPlan {
    PLANS[fabric_name as usize]
        .get_or_init(|| fabric_name.fabric_plan())
        .clone()
}

pub fn all_fabric_plans() -> impl Iterator<Item = FabricPlan> {
    FabricName::iter().map(get_fabric_plan)
}
