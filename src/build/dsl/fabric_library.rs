use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName::*, Side};
use crate::build::dsl::fabric_dsl::{on, *};
use crate::build::dsl::fabric_plan::FabricPlan;
use crate::connector::strut::HEADLESS_HUG_TUBES;
use crate::fabric::dimensions::Hardware;
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
    #[strum(serialize = "Headless Hug")]
    HeadlessHug,
    #[strum(serialize = "Open Claw")]
    OpenClaw,
    #[strum(serialize = "Halo by Crane")]
    HaloByCrane,
    #[strum(serialize = "Twisted Infinity")]
    TwistedInfinity,
    Propeller,
    Diamond,
}

impl FabricName {
    pub fn fabric_plan(self) -> FabricPlan {
        use FabricName::*;
        match self {
            OpenClaw => self
                .build(
                    FabricDimensions::default()
                        .with_scale(M(0.80))
                        .with_connector()
                        .with_connectors_shown(),
                )
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX).column(4).tip_label().prism(Pct(200.0)),
                    on(OmniBotY).column(4).tip_label().prism(Pct(200.0)),
                    on(OmniBotZ).column(4).tip_label().prism(Pct(200.0)),
                    on(OmniTop).prism(Pct(200.0)),
                    on(OmniBot).open(),
                ])
                .omit([ABC(2) << ABC(3), ABC(2) >> ABC(1)])
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
                .actuators([phase(Pct(0.0)).between(ABC(15) >> ABC(28))]),
            HaloByCrane => self
                .build(
                    FabricDimensions::default()
                        .with_altitude(M(2.0))
                        .with_scale(M(1.0)),
                )
                .seed(SingleTwistLeft, Seed(1))
                .faces([
                    on(SingleTop)
                        .column(3)
                        .shrink_by(Pct(8.0))
                        .then(hub(OmniSymmetrical).faces([
                            on(OmniTopX).column(9).shrink_by(Pct(8.0)).tip_label(),
                            on(OmniTopY).column(8).shrink_by(Pct(8.0)).tip_label(),
                        ]))
                        .into(),
                    on(SingleBot).tip_label(),
                ])
                .join(Sec(3.0), Tip(OmniTopX), Tip(OmniTopY))
                .add(
                    Sec(3.0),
                    [
                        ("A01.1", "A02.1", Pct(60.0)),
                        ("A02.1", "A03.1", Pct(60.0)),
                        ("A01.4", "A02.6", Pct(66.0)),
                        ("A02.6", "A03.4", Pct(66.0)),
                    ],
                )
                .vulcanize(Sec(1.0))
                .down(Sec(1.0), [Tip(SingleBot)])
                .down(Sec(0.1), [Tip(SingleBot)])
                .pretense(Sec(0.02), Pct(1.0))
                .surface_frozen(),

            HeadlessHug => headless_hug_plan(HeadlessHugShape::default()),

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

/// Headless Hug's shape: the numbers its plan is built from, the same left
/// and right. `Default` is the sculpture as designed.
#[derive(Clone, Copy, Debug)]
pub struct HeadlessHugShape {
    pub leg_shrink: Pct,
    pub leg_prism: Pct,
    pub shoulder_shrink: Pct,
    pub arm_shrink: Pct,
    pub arm_prism: Pct,
    /// Spacers: each foot to its own hand, each foot to the other hand,
    /// foot to foot, hand to hand, and across the lower chest.
    pub same_side: Pct,
    pub crossed: Pct,
    pub feet: Pct,
    pub hands: Pct,
    pub chest: Pct,
}

impl Default for HeadlessHugShape {
    fn default() -> Self {
        HeadlessHugShape {
            leg_shrink: Pct(13.0),
            leg_prism: Pct(400.0),
            shoulder_shrink: Pct(30.0),
            arm_shrink: Pct(5.0),
            arm_prism: Pct(300.0),
            same_side: Pct(100.0),
            crossed: Pct(102.0),
            feet: Pct(30.0),
            hands: Pct(50.0),
            chest: Pct(80.0),
        }
    }
}

/// Headless Hug's plan, built to `shape`.
pub fn headless_hug_plan(shape: HeadlessHugShape) -> FabricPlan {
    FabricName::HeadlessHug
        .build(
            FabricDimensions::default()
                .with_altitude(M(12.0))
                .with_scale(M(0.75))
                .with_connector()
                .with_tube_sets(HEADLESS_HUG_TUBES)
                .with_connectors_shown()
                .with_hardware(Hardware::OPEN_CLAW),
        )
        .seed(OmniSymmetrical, Seed(2))
        .faces([
            on(LowerLeft)
                .column(3)
                .shrink_by(shape.leg_shrink)
                .prism(shape.leg_prism)
                .label(Foot(Side::Left)),
            on(LowerRight)
                .column(3)
                .shrink_by(shape.leg_shrink)
                .prism(shape.leg_prism)
                .label(Foot(Side::Right)),
            on(UpperLeft).column(1).shrink_by(shape.shoulder_shrink).then(
                hub(OmniSymmetrical).faces([
                    on(OmniTopY).label(ChestUpper(Side::Left)),
                    on(OmniBotZ).label(ChestLower(Side::Left)),
                    on(OmniBotX)
                        .column(3)
                        .shrink_by(shape.arm_shrink)
                        .prism(shape.arm_prism)
                        .label(Hand(Side::Left))
                        .into(),
                ]),
            ),
            on(UpperRight).column(1).shrink_by(shape.shoulder_shrink).then(
                hub(OmniSymmetrical).faces([
                    on(OmniTopZ).label(ChestUpper(Side::Right)),
                    on(OmniBotX).label(ChestLower(Side::Right)),
                    on(OmniBotY)
                        .column(3)
                        .shrink_by(shape.arm_shrink)
                        .prism(shape.arm_prism)
                        .label(Hand(Side::Right))
                        .into(),
                ]),
            ),
        ])
        .prepare_vulcanize(0.5, VulcanizeMode::Linear)
        .space_parallel(
            Sec(2.8),
            [
                spacer([Foot(Side::Left), Hand(Side::Left)], shape.same_side),
                spacer([Foot(Side::Right), Hand(Side::Right)], shape.same_side),
                spacer([Foot(Side::Left), Hand(Side::Right)], shape.crossed),
                spacer([Foot(Side::Right), Hand(Side::Left)], shape.crossed),
                spacer([Foot(Side::Left), Foot(Side::Right)], shape.feet),
                spacer([Hand(Side::Left), Hand(Side::Right)], shape.hands),
                spacer([ChestLower(Side::Left), ChestLower(Side::Right)], shape.chest),
            ],
        )
        .vulcanize(Sec(2.1))
        .omit([
            Z(2) - LR(2),
            LR(2) - LR(3),
            LR(1) - LR(4),
            Z(3) - LR(1),
            LR(21) - LR(26),
            LR(8) - LR(23),
        ])
        .hinge([
            LR(13) | LR(13),
            LR(9) | LR(9),
            LR(10) - LR(39),
            LR(6) - LR(22),
            LR(8) - LR(18),
        ])
        .bridle([
            LR(5).bridle(Z(4), Z(2)),
            LR(55).bridle(LR(52), LR(38)),
            LR(37).bridle(LR(23), LR(21)),
            LR(20).bridle(LR(2), LR(10)),
            LR(7).bridle(LR(2), LR(1)),
            LR(11).bridle(LR(1), LR(2)),
            LR(15).bridle(Z(3), Z(1)),
            LR(33).bridle(LR(12), Z(1)),
        ])
        .down(Sec(1.0), [Foot(Side::Left), Foot(Side::Right)])
        .pretense(Sec(0.3), Pct(0.5))
        .surface_frozen()
        .fall(Sec(1.5))
        // Standing upright it sways slowly, every 4 s or so: it needs 30 s to
        // come fully to rest, so what's exported has stopped moving.
        .settle(Sec(30.0))
        .guy_lines([
            // Lengths that stand it upright: its centre of gravity over its
            // feet, both pairs lightly taut.
            // Tuned with `search_headless_hug_guy_lines`.
            guy_line(LR(14), (M(0.0), M(5.2)), Pct(90.55)),
            guy_line(LR(14), (M(0.0), M(-5.2)), Pct(108.95)),
        ])
        .animate()
        .actuator_frequency(Hz(0.24))
        .amplitude(Pct(0.75))
        .sine()
        .actuators([
            phase(Pct(0.0)).interval(LR(14) - LR(63)),
            phase(Pct(50.0)).interval(LR(14) - LR(64)),
        ])
}

pub fn get_fabric_plan(fabric_name: FabricName) -> FabricPlan {
    PLANS[fabric_name as usize]
        .get_or_init(|| fabric_name.fabric_plan())
        .clone()
}

pub fn all_fabric_plans() -> impl Iterator<Item = FabricPlan> {
    FabricName::iter().map(get_fabric_plan)
}
