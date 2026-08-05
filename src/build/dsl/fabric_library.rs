use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName::*, Side};
use crate::build::dsl::fabric_dsl::{on, *};
use crate::build::dsl::fabric_plan::FabricPlan;
use std::sync::OnceLock;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use crate::build::dsl::fabric_dsl::Rotation::OneThird;

static PLANS: [OnceLock<FabricPlan>; 9] = [
    OnceLock::new(),
    OnceLock::new(),
    OnceLock::new(),
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
    Flagellum,
    #[strum(serialize = "Halo by Crane")]
    HaloByCrane,
    #[strum(serialize = "Headless Hug")]
    HeadlessHug,
    #[strum(serialize = "Minimal Man")]
    MinimalMan,
    Diamond,
    Propeller,
    Infinity,
    #[strum(serialize = "Twisted Infinity")]
    TwistedInfinity,
}

impl FabricName {
    pub fn fabric_plan(self) -> FabricPlan {
        use FabricName::*;
        match self {
            OpenClaw => self
                .build(
                    FabricDimensions::default()
                        .with_scale(M(0.80))
                        .with_connector(),
                )
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX).column(4).tip_label().prism(Pct(200.0)),
                    on(OmniBotY).column(4).tip_label().prism(Pct(200.0)),
                    on(OmniBotZ).column(4).tip_label().prism(Pct(200.0)),
                    on(OmniTop).prism(Pct(200.0)),
                    on(OmniBot).open(),
                ])
                .omit([
                    ("B00.2", "C00.3"),
                    ("B00.2", "A00.1"),
                    ("A00.2", "C00.1"),
                    ("A00.2", "B00.3"),
                    ("A00.3", "C00.2"),
                    ("C00.2", "B00.1"),
                ])
                .prepare_vulcanize(0.5, VulcanizeMode::Linear)
                .space(
                    Sec(2.8),
                    [Tip(OmniBotX), Tip(OmniBotY), Tip(OmniBotZ)],
                    Pct(46.0),
                )
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
            HaloByCrane => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([on(SingleTop).column(4).shrink_by(Pct(8.0)).then(
                    hub(OmniSymmetrical).faces([
                        on(OmniTopX).column(12).shrink_by(Pct(8.0)).tip_label(),
                        on(OmniTopY).column(11).shrink_by(Pct(8.0)).tip_label(),
                    ]),
                )])
                .join(Sec(3.0), Tip(OmniTopX), Tip(OmniTopY))
                .vulcanize(Sec(1.0))
                .pretense(Sec(0.02), Pct(1.0))
                .surface_frozen(),

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
                        .with_altitude(M(12.0))
                        .with_scale(M(1.0)),
                )
                .seed(OmniSymmetrical, Seed(2))
                .faces([
                    on(LowerLeft)
                        .column(6)
                        .shrink_by(Pct(12.0))
                        .label(Foot(Side::Left)),
                    on(LowerRight)
                        .column(6)
                        .shrink_by(Pct(12.0))
                        .label(Foot(Side::Right)),
                    on(UpperLeft).column(2).shrink_by(Pct(15.0)).then(
                        hub(OmniSymmetrical).faces([
                            on(OmniTopZ).label(ChestUpper(Side::Left)),
                            on(OmniBotX).label(ChestLower(Side::Left)),
                            on(OmniBotY)
                                .column(6)
                                .shrink_by(Pct(10.0))
                                .label(Hand(Side::Left))
                                .into(),
                        ]),
                    ),
                    on(UpperRight).column(2).shrink_by(Pct(15.0)).then(
                        hub(OmniSymmetrical).faces([
                            on(OmniTopY).label(ChestUpper(Side::Right)),
                            on(OmniBotZ).label(ChestLower(Side::Right)),
                            on(OmniBotX)
                                .column(6)
                                .shrink_by(Pct(10.0))
                                .label(Hand(Side::Right))
                                .into(),
                        ]),
                    ),
                ])
                .prepare_vulcanize(0.5, VulcanizeMode::Linear)
                .space_parallel(
                    Sec(8.0),
                    [
                        spacer([Foot(Side::Left), Hand(Side::Left)], Pct(100.0)),
                        spacer([Foot(Side::Right), Hand(Side::Right)], Pct(100.0)),
                        spacer([Foot(Side::Left), Hand(Side::Right)], Pct(102.0)),
                        spacer([Foot(Side::Right), Hand(Side::Left)], Pct(102.0)),
                        spacer([Foot(Side::Left), Foot(Side::Right)], Pct(30.0)),
                        spacer([Hand(Side::Left), Hand(Side::Right)], Pct(20.0)),
                        spacer([ChestLower(Side::Left), ChestLower(Side::Right)], Pct(50.0)),
                    ],
                )
                .vulcanize(Sec(2.0))
                .add(Sec(1.0), [("C03.10", "D03.10", Pct(90.0))])
                .down(Sec(1.0), [Foot(Side::Left), Foot(Side::Right)])
                .pretense(Sec(1.0), Pct(1.0))
                .surface_frozen()
                .fall(Sec(1.5))
                .settle(Sec(1.5)),

            MinimalMan => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(12.0))
                        .with_scale(M(1.0))
                        .with_connector(),
                )
                .seed(TorqueSymmetrical, Seed(2))
                .faces([
                    on(LowerLeft)
                        .column(4)
                        .shrink_by(Pct(20.0))
                        .prism(Pct(200.0))
                        .label(Foot(Side::Left)),
                    on(LowerRight)
                        .column(4)
                        .shrink_by(Pct(20.0))
                        .prism(Pct(200.0))
                        .label(Foot(Side::Right)),
                    on(UpperLeft).column(1).shrink_by(Pct(40.0)).then(
                        hub(OmniSymmetrical).faces([on(OmniTopZ)
                            .column(3)
                            .label(Hand(Side::Left))]),
                    ),
                    on(UpperRight).rotate(OneThird).column(1).shrink_by(Pct(40.0)).then(
                        hub(OmniSymmetrical).faces([on(OmniTopX)
                            .column(3)
                            .label(Hand(Side::Right))]),
                    ),
                ])
                .prepare_vulcanize(0.5, VulcanizeMode::Linear)
                .space_parallel(
                    Sec(8.0),
                    [
                        spacer([Foot(Side::Left), Hand(Side::Left)], Pct(80.0)),
                        spacer([Foot(Side::Right), Hand(Side::Right)], Pct(80.0)),
                        spacer([Foot(Side::Left), Hand(Side::Right)], Pct(102.0)),
                        spacer([Foot(Side::Right), Hand(Side::Left)], Pct(102.0)),
                        spacer([Foot(Side::Left), Foot(Side::Right)], Pct(40.0)),
                        spacer([Hand(Side::Left), Hand(Side::Right)], Pct(40.0)),
                    ],
                )
                .vulcanize(Sec(2.0))                .down(Sec(1.0), [Foot(Side::Left), Foot(Side::Right)])
                .pretense(Sec(1.0), Pct(1.0))
                .surface_frozen()
                .fall(Sec(1.5))
                .settle(Sec(1.5)),

            Diamond => self
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBot).column(4).then(
                        hub(OmniSymmetrical).faces([
                            on(OmniTopX).column(4).then(
                                hub(OmniSymmetrical).faces([
                                    on(OmniTopZ).column(4).then(
                                        hub(OmniSymmetrical)
                                            .faces([on(OmniTopZ).column(2).label(Bottom(3))]),
                                    ),
                                    on(OmniTopX).column(4).then(
                                        hub(OmniSymmetrical)
                                            .faces([on(OmniTopX).column(2).label(Bottom(4))]),
                                    ),
                                ]),
                            ),
                            on(OmniTopY).column(4).then(
                                hub(OmniSymmetrical).faces([
                                    on(OmniTopZ).column(4).then(
                                        hub(OmniSymmetrical)
                                            .faces([on(OmniTopZ).column(2).label(Bottom(1))]),
                                    ),
                                    on(OmniTopX).column(4).then(
                                        hub(OmniSymmetrical)
                                            .faces([on(OmniTopX).column(2).label(Bottom(5))]),
                                    ),
                                ]),
                            ),
                            on(OmniTopZ).column(4).then(
                                hub(OmniSymmetrical).faces([
                                    on(OmniTopZ).column(4).then(
                                        hub(OmniSymmetrical)
                                            .faces([on(OmniTopZ).column(2).label(Bottom(6))]),
                                    ),
                                    on(OmniTopX).column(4).then(
                                        hub(OmniSymmetrical)
                                            .faces([on(OmniTopX).column(2).label(Bottom(2))]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                    on(OmniTopX).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopZ).column(4).then(
                            hub(OmniSymmetrical).faces([on(OmniTopZ).column(2).label(Top(4))]),
                        ),
                        on(OmniTopX).column(4).then(
                            hub(OmniSymmetrical).faces([on(OmniTopX).column(2).label(Top(6))]),
                        ),
                    ])),
                    on(OmniTopY).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopZ).column(4).then(
                            hub(OmniSymmetrical).faces([on(OmniTopZ).column(2).label(Top(5))]),
                        ),
                        on(OmniTopX).column(4).then(
                            hub(OmniSymmetrical).faces([on(OmniTopX).column(2).label(Top(3))]),
                        ),
                    ])),
                    on(OmniTopZ).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopZ).column(4).then(
                            hub(OmniSymmetrical).faces([on(OmniTopZ).column(2).label(Top(2))]),
                        ),
                        on(OmniTopX).column(4).then(
                            hub(OmniSymmetrical).faces([on(OmniTopX).column(2).label(Top(1))]),
                        ),
                    ])),
                ])
                .join_parallel(Sec(2.0), (1..=6).map(|n| (Bottom(n), Top(n))))
                .vulcanize(Sec(1.0))
                .pretense(Sec(1.0), Pct(1.0))
                .floating(),

            Propeller => self
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX).column(11).shrink_by(Pct(10.0)).tip_label(),
                    on(OmniBotY).column(11).shrink_by(Pct(10.0)).tip_label(),
                    on(OmniBotZ).column(11).shrink_by(Pct(10.0)).tip_label(),
                    on(OmniTopX).column(11).shrink_by(Pct(10.0)).tip_label(),
                    on(OmniTopY).column(11).shrink_by(Pct(10.0)).tip_label(),
                    on(OmniTopZ).column(11).shrink_by(Pct(10.0)).tip_label(),
                ])
                .join_parallel(
                    Sec(2.0),
                    tips([
                        (OmniBotX, OmniTopZ),
                        (OmniBotY, OmniTopX),
                        (OmniBotZ, OmniTopY),
                    ]),
                )
                .vulcanize(Sec(1.0))
                .pretense(Sec(1.0), Pct(1.0))
                .floating(),

            Infinity => self
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(4))
                .faces([
                    on(RightFrontTop)
                        .column(11)
                        .shrink_by(Pct(12.0))
                        .tip_label(),
                    on(RightFrontBottom)
                        .column(11)
                        .shrink_by(Pct(12.0))
                        .tip_label(),
                    on(LeftBackTop).column(11).shrink_by(Pct(12.0)).tip_label(),
                    on(LeftBackBottom)
                        .column(11)
                        .shrink_by(Pct(12.0))
                        .tip_label(),
                ])
                .join_parallel(
                    Sec(2.0),
                    tips([
                        (RightFrontTop, RightFrontBottom),
                        (LeftBackTop, LeftBackBottom),
                    ]),
                )
                .vulcanize(Sec(1.0))
                .pretense(Sec(1.0), Pct(1.0))
                .floating(),

            TwistedInfinity => self
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(4))
                .faces([
                    on(RightFrontTop).column(6).shrink_by(Pct(17.0)).tip_label(),
                    on(RightBackBottom)
                        .column(5)
                        .shrink_by(Pct(17.0))
                        .tip_label(),
                    on(LeftBackTop).column(6).shrink_by(Pct(17.0)).tip_label(),
                    on(LeftFrontBottom)
                        .column(5)
                        .shrink_by(Pct(17.0))
                        .tip_label(),
                ])
                .join_parallel(
                    Sec(2.0),
                    tips([
                        (RightFrontTop, RightBackBottom),
                        (LeftBackTop, LeftFrontBottom),
                    ]),
                )
                .vulcanize(Sec(1.0))
                .pretense(Sec(1.0), Pct(1.0))
                .floating(),
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
