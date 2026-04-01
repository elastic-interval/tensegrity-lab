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
    #[strum(serialize = "Open Claw")]
    OpenClaw,
    Triped,
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
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX)
                        .column(4)
                        .shrink_by(Pct(20.0))
                        .mark(End)
                        .prism(Pct(250.0)),
                    on(OmniBotY)
                        .column(4)
                        .shrink_by(Pct(20.0))
                        .mark(End)
                        .prism(Pct(250.0)),
                    on(OmniBotZ)
                        .column(4)
                        .shrink_by(Pct(20.0))
                        .mark(End)
                        .prism(Pct(250.0)),
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
                .space(Sec(2.8), End, Pct(48.2))
                .vulcanize(Sec(1.0))
                .zero_g_pretense(Sec(0.1), Pct(0.08), Pct(0.0))
                .surface_frozen()
                .fall(Sec(1.5))
                .settle(Sec(1.5))
                .grav_pretense(Sec(0.1), Pct(0.12))
                .done(),
            Triped => self
                .build(FabricDimensions::default())
                .seed(OmniSymmetrical, Seed(1))
                .faces([
                    on(OmniBotX)
                        .column(8)
                        .shrink_by(Pct(10.0))
                        .mark(End)
                        .prism(Pct(100.0)),
                    on(OmniBotY)
                        .column(8)
                        .shrink_by(Pct(10.0))
                        .mark(End)
                        .prism(Pct(100.0)),
                    on(OmniBotZ)
                        .column(8)
                        .shrink_by(Pct(10.0))
                        .mark(End)
                        .prism(Pct(100.0)),
                    on(OmniTop).prism(Pct(100.0)),
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
                .space(Sec(3.0), End, Pct(25.0))
                .vulcanize(Sec(1.0))
                .zero_g_pretense(Sec(0.2), Pct(0.08), Pct(0.0))
                .surface_frozen()
                .fall(Sec(2.0))
                .settle(Sec(3.0))
                .grav_pretense(Sec(0.3), Pct(0.12))
                .animate()
                .actuator_frequency(Hz(0.73))
                .amplitude(Pct(3.0))
                .stiffness(Pct(2.0))
                .sine()
                .actuators([
                    phase(Pct(0.0)).between("AX8YZ1", "CX1Z3"),
                    phase(Pct(0.0)).between("BX8YZ1", "AX1Z3"),
                    phase(Pct(0.0)).between("CX8YZ1", "BX1Z3"),
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
                .zero_g_pretense(Sec(0.02), Pct(0.1), Pct(1.0))
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
                        on(OmniTopX).column(12).shrink_by(Pct(8.0)).mark(HaloEnd),
                        on(OmniTopY).column(11).shrink_by(Pct(8.0)).mark(HaloEnd),
                    ]),
                )])
                .join(Sec(10.0), HaloEnd)
                .vulcanize(Sec(5.0))
                .zero_g_pretense(Sec(0.02), Pct(0.1), Pct(1.0))
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
                .zero_g_pretense(Sec(0.02), Pct(0.1), Pct(1.0))
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
                .zero_g_pretense(Sec(0.02), Pct(0.1), Pct(1.0))
                .surface_frozen(),

            HeadlessHug => self
                .build(
                    FabricDimensions::default()
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
                .zero_g_pretense(Sec(0.02), Pct(0.1), Pct(1.0))
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
