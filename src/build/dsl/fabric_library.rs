use crate::build::dsl::brick_dsl::{BrickName::*, BrickRole::*, FaceLabel::*, FaceName::*};
use crate::build::dsl::fabric_dsl::{on, *};
use crate::build::dsl::fabric_plan::FabricPlan;
use std::sync::OnceLock;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

static PLANS: [OnceLock<FabricPlan>; 7] = [
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
    Mockup,
    Vertebra,
    Flagellum,
    #[strum(serialize = "Halo by Crane")]
    HaloByCrane,
    #[strum(serialize = "Headless Hug")]
    HeadlessHug,
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
                    ("B00.2", "C00.3"),
                    ("B00.2", "A00.1"),
                    ("A00.2", "C00.1"),
                    ("A00.2", "B00.3"),
                    ("A00.3", "C00.2"),
                    ("C00.2", "B00.1"),
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
                    hub(OmniSymmetrical).faces([
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
                        .with_altitude(M(12.0))
                        .with_scale(M(1.0)),
                )
                .seed(OmniSymmetrical, Seed(2))
                .faces([
                    on(LowerLeft).column(6).shrink_by(Pct(12.0)).label(LeftFoot),
                    on(LowerRight)
                        .column(6)
                        .shrink_by(Pct(12.0))
                        .label(RightFoot),
                    on(UpperLeft).column(2).shrink_by(Pct(15.0)).then(
                        hub(OmniSymmetrical).faces([
                            on(OmniTopZ).label(LeftChestUpper),
                            on(OmniBotX).label(LeftChestLower),
                            on(OmniBotY)
                                .column(6)
                                .shrink_by(Pct(10.0))
                                .label(LeftHand)
                                .into(),
                        ]),
                    ),
                    on(UpperRight).column(2).shrink_by(Pct(15.0)).then(
                        hub(OmniSymmetrical).faces([
                            on(OmniTopY).label(RightChestUpper),
                            on(OmniBotZ).label(RightChestLower),
                            on(OmniBotX)
                                .column(6)
                                .shrink_by(Pct(10.0))
                                .label(RightHand)
                                .into(),
                        ]),
                    ),
                ])
                .prepare_vulcanize(0.5, VulcanizeMode::Linear)
                .space_parallel(
                    Sec(8.0),
                    [
                        spacer([LeftFoot, LeftHand], Pct(100.0)),
                        spacer([RightFoot, RightHand], Pct(100.0)),
                        spacer([LeftFoot, RightHand], Pct(102.0)),
                        spacer([RightFoot, LeftHand], Pct(102.0)),
                        spacer([LeftFoot, RightFoot], Pct(30.0)),
                        spacer([LeftHand, RightHand], Pct(20.0)),
                        spacer([LeftChestLower, RightChestLower], Pct(50.0)),
                    ],
                )
                .vulcanize(Sec(2.0))
                .add(Sec(1.0), [("C03.10", "D03.10", Pct(90.0))])
                .down(Sec(1.0), [LeftFoot, RightFoot])
                .pretense(Sec(1.0), Pct(1.0))
                .surface_frozen()
                .fall(Sec(1.5))
                .settle(Sec(1.5)),

            Diamond => self
                // Port of pretenst Diamond (../pretenst/.../bootstrap.ts).
                // Seed has 4 branches: 'a' through the bottom-apex, 'b/c/d'
                // through three top-side faces. Each branch is col-4 into a
                // hub. The 'a' branch's hub splits into 3 sub-branches
                // (each col-4 → hub → 2 leaves); the 'b/c/d' branches split
                // into 2 leaves directly. 12 leaves total, labelled
                // Mark1..Mark6 in pairs (each label appears twice — the
                // pairs are what pretenst's `join` would bring together).
                // Hub roles auto-derive from parent face spin; child face
                // names are picked so leaf positions match the original.
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBot).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopX).column(4).then(hub(OmniSymmetrical).faces([
                            on(OmniTopZ).column(2).label(Mark3),
                            on(OmniTopX).column(2).label(Mark4),
                        ])),
                        on(OmniTopY).column(4).then(hub(OmniSymmetrical).faces([
                            on(OmniTopZ).column(2).label(Mark1),
                            on(OmniTopX).column(2).label(Mark5),
                        ])),
                        on(OmniTopZ).column(4).then(hub(OmniSymmetrical).faces([
                            on(OmniTopZ).column(2).label(Mark6),
                            on(OmniTopX).column(2).label(Mark2),
                        ])),
                    ])),
                    on(OmniTopX).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopZ).column(2).label(Mark5),
                        on(OmniTopX).column(2).label(Mark3),
                    ])),
                    on(OmniTopY).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopZ).column(2).label(Mark2),
                        on(OmniTopX).column(2).label(Mark1),
                    ])),
                    on(OmniTopZ).column(4).then(hub(OmniSymmetrical).faces([
                        on(OmniTopZ).column(2).label(Mark4),
                        on(OmniTopX).column(2).label(Mark6),
                    ])),
                ])
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
