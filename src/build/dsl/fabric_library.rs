use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName::*, Side};
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
                    ("C2", "B3"),
                    ("C2", "A1"),
                    ("A2", "B1"),
                    ("A2", "C3"),
                    ("A3", "B2"),
                    ("B2", "C1"),
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
                .with_scale(M(0.8))
                .with_connector(),
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
            ("Z2", "R2"),
            ("Z2", "L2"),
            ("L2", "L3"),
            ("R2", "R3"),
            ("R1", "R4"),
            ("L1", "L4"),
            ("Z3", "R1"),
            ("Z3", "L1"),
            ("L21", "L26"),
            ("R21", "R26"),
            ("L8", "L23"),
            ("R8", "R23"),
        ])
        .hinge([
            ("L13", "R13"),
            ("L9", "R9"),
            ("L10", "L39"),
            ("R10", "R39"),
            ("L6", "L22"),
            ("R6", "R22"),
            ("L8", "L18"),
            ("R8", "R18"),
        ])
        .bridle([
            ("R5", "Z4", "Z2"),
            ("L5", "Z4", "Z2"),
            ("L55", "L52", "L38"),
            ("R55", "R52", "R38"),
            ("L37", "L23", "L21"),
            ("R37", "R23", "R21"),
            ("R20", "R2", "R10"),
            ("L20", "L2", "L10"),
            ("R7", "R2", "R1"),
            ("L7", "L2", "L1"),
            ("R11", "R1", "R2"),
            ("L11", "L1", "L2"),
            ("R15", "Z3", "Z1"),
            ("L15", "Z3", "Z1"),
            ("L33", "L12", "Z1"),
            ("R33", "R12", "Z1"),
        ])
        .down(Sec(1.0), [Foot(Side::Left), Foot(Side::Right)])
        .pretense(Sec(0.3), Pct(1.0))
        .surface_frozen()
        .fall(Sec(1.5))
        .settle(Sec(1.5))
        .guy_lines([
            guy_line("L14", (M(0.0), M(5.5)), Pct(97.0)),
            guy_line("R14", (M(0.0), M(5.5)), Pct(97.0)),
            guy_line("L14", (M(0.0), M(-5.5)), Pct(102.7)),
            guy_line("R14", (M(0.0), M(-5.5)), Pct(102.7)),
        ])
        .animate()
        .actuator_frequency(Hz(1.0))
        .amplitude(Pct(0.75))
        .sine()
        .actuators([
            phase(Pct(0.0)).interval("L14", "L63"),
            phase(Pct(0.0)).interval("R14", "R63"),
            phase(Pct(50.0)).interval("L14", "L64"),
            phase(Pct(50.0)).interval("R14", "R64"),
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
