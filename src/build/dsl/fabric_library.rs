use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName::*};
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
    #[strum(serialize = "Open Claw")]
    OpenClaw,
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
            OpenClaw => self
                .build(
                    FabricDimensions::default()
                        .with_scale(M(0.80))
                        .with_locked_bend_magnitudes(vec![12.0, 30.0, 49.0, 68.0]),
                )
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX).column(4).label(LegEndA).prism(Pct(200.0)),
                    on(OmniBotY).column(4).label(LegEndB).prism(Pct(200.0)),
                    on(OmniBotZ).column(4).label(LegEndC).prism(Pct(200.0)),
                    on(OmniTop).prism(Pct(200.0)),
                    on(OmniBot).open(),
                ])
                .omit([
                    ("Z6", "Z9"),
                    ("Z6", "Z3"),
                    ("Z2", "Z11"),
                    ("Z2", "Z5"),
                    ("Z1", "Z10"),
                    ("Z10", "Z7"),
                ])
                .prepare_vulcanize(0.5, VulcanizeMode::Linear)
                .space(Sec(2.8), [LegEndA, LegEndB, LegEndC], Pct(46.0))
                .vulcanize(Sec(1.0))
                .pretense(Sec(3.0), Pct(1.0))
                .surface_frozen()
                .fall(Sec(1.5))
                .settle(Sec(1.5))
                .animate()
                .actuator_frequency(Hz(3.0))
                .amplitude(Pct(3.0))
                .stiffness(Pct(1.0))
                .sine()
                .actuators([
                    phase(Pct(0.0)).between("CX2Z4", "BX4Z5"),
                    phase(Pct(0.0)).between("AX2Z4", "CX4Z5"),
                    phase(Pct(0.0)).between("BX2Z4", "AX4Z5"),
                ]),
            Mockup => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(2.0))
                        .with_scale(M(0.59)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(2).shrink_by(Pct(12.0))])
                .vulcanize(Sec(2.0))
                .pretense(Sec(0.02), Pct(1.0))
                .surface_frozen()
                .fall(Sec(3.0))
                .settle(Sec(4.0)),

            HaloByCrane => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(4).shrink_by(Pct(8.0)).then(
                    hub(OmniSymmetrical, OnSpinLeft).faces([
                        on(OmniTopX).column(12).shrink_by(Pct(8.0)).label(HaloEndA),
                        on(OmniTopY).column(11).shrink_by(Pct(8.0)).label(HaloEndB),
                    ]),
                )])
                .join(Sec(10.0), HaloEndA, HaloEndB)
                .vulcanize(Sec(5.0))
                .pretense(Sec(0.02), Pct(1.0))
                .surface_frozen(),

            Vertebra => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(0.5))
                        .with_scale(M(0.0746)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(1)])
                .centralize_at(Sec(1.0), M(0.075))
                .pretense(Sec(0.02), Pct(1.0))
                .floating(),

            Flagellum => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(20).shrink_by(Pct(5.0))])
                .vulcanize(Sec(1.0))
                .pretense(Sec(0.02), Pct(1.0))
                .surface_frozen(),

            HeadlessHug => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(9.0))
                        .with_scale(M(1.0)),
                )
                .seed(OmniSymmetrical, Seed(2))
                .faces([
                    on(LowerLeft)
                        .column(4)
                        .shrink_by(Pct(8.0))
                        .then(column(1).then(column(2).label(LeftFoot))),
                    on(LowerRight)
                        .column(4)
                        .shrink_by(Pct(8.0))
                        .then(column(1).then(column(2).label(RightFoot))),
                    on(UpperLeft).column(2).shrink_by(Pct(10.0)).then(
                        hub(OmniSymmetrical, OnSpinLeft).faces([
                            on(OmniTopZ).label(LeftChestUpper),
                            on(OmniBotX).label(LeftChestLower),
                            on(OmniBotY)
                                .column(3)
                                .shrink_by(Pct(10.0))
                                .then(column(1).then(column(2).label(LeftHand)))
                                .into(),
                        ]),
                    ),
                    on(UpperRight).column(2).shrink_by(Pct(10.0)).then(
                        hub(OmniSymmetrical, OnSpinRight).faces([
                            on(OmniTopY).label(RightChestUpper),
                            on(OmniBotZ).label(RightChestLower),
                            on(OmniBotX)
                                .column(3)
                                .shrink_by(Pct(10.0))
                                .then(column(1).then(column(2).label(RightHand)))
                                .into(),
                        ]),
                    ),
                ])
                .space(Sec(2.0), [LeftFoot, RightFoot], Pct(30.0))
                .space(Sec(2.0), [LeftHand, RightHand], Pct(20.0))
                .space(Sec(2.0), [LeftChestLower, RightChestLower], Pct(40.0))
                .vulcanize(Sec(2.0))
                .down(Sec(1.0), [LeftFoot, RightFoot])
                .pretense(Sec(1.0), Pct(1.0))
                .surface_frozen()
                .fall(Sec(1.5))
                .settle(Sec(1.5)),
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
