use crate::wgpu::button::Button;
use crate::ControlState::*;
use crate::CrucibleAction::*;
use crate::LabEvent::*;
use crate::units::Meters;
use crate::{ControlState, LabEvent, StateChange, TesterAction, TweakFeature};

/// How much faster than real time Fast-forward runs.
pub const FAST_FORWARD: f32 = 10.0;

/// What pressing sends: one event, or a step down or up either side of the
/// label.
enum Press {
    Once(LabEvent),
    Step { down: LabEvent, up: LabEvent },
}

struct Entry {
    label: &'static str,
    shows_in: fn(&ControlState) -> bool,
    press: Press,
    /// On the bar all the time, disabled where it doesn't apply; otherwise
    /// it shows only where it applies, the buttons of a tool of its own.
    fixed: bool,
}

/// Every button in the app, in the order they line up along the bottom. The
/// bar holds still: its buttons are all there all the time, each enabled
/// only in the stages it applies to. Physics testing and brick baking are
/// tools of their own, their buttons showing only while they run.
pub struct ButtonBar {
    entries: Vec<Entry>,
}

impl ButtonBar {
    /// A model, at `--model-scale`, is for building from: no connectors, so no
    /// parts, and no animation, wind, quake or assembly.
    pub fn new(model: bool) -> Self {
        let mut bar = ButtonBar { entries: Vec::new() };
        bar.once("Back to joint", |s| matches!(s, ShowingInterval(_)), UpdateState(StateChange::BackToJoint));
        bar.once("Deselect", |s| matches!(s, ShowingJoint(_)), Crucible(ClearSelection));
        if !model {
            bar.once(
                "Start animation",
                |s| matches!(s, Viewing { animation_available: true, .. }),
                Crucible(ToAnimating),
            );
            bar.once("Stop animation", |s| matches!(s, Animating { .. }), Crucible(ToViewing));
            bar.once("Wind", |s| matches!(s, Viewing { .. } | Animating { .. }), Crucible(CycleWind));
            bar.once("Quake", |s| matches!(s, Viewing { .. } | Animating { .. }), Crucible(Quake));
            bar.once("Take apart", |s| matches!(s, Viewing { assembly_available: true, .. }), Crucible(Assemble));
        }
        bar.tool(
            "Physics test",
            |s| matches!(s, Viewing { physics_testing_available: true, .. }),
            Crucible(ToPhysicsTesting),
        );
        bar.tool("Exit test", |s| matches!(s, PhysicsTesting), Crucible(ToViewing));
        bar.tool(
            "Movement stats",
            |s| matches!(s, PhysicsTesting),
            Crucible(TesterDo(TesterAction::ToggleMovementSampler)),
        );
        bar.tool("Color by role", |s| matches!(s, PhysicsTesting), UpdateState(StateChange::ToggleColorByRole));
        bar.tool("Jump", |s| matches!(s, PhysicsTesting), Crucible(CentralizeFabric(Some(Meters(1.0)))));
        bar.once("Background", |_| true, UpdateState(StateChange::ToggleBackground));
        if !model {
            bar.once(
                "Parts",
                |s| matches!(s, Viewing { .. } | Animating { .. } | ShowingJoint(_) | ShowingInterval(_)),
                UpdateState(StateChange::TogglePartsView),
            );
        }
        bar.tool("Next brick", |s| matches!(s, Baking), NextBrick);
        bar.once(
            "Fast-forward",
            |s| matches!(s, Building | Assembling { fast: false } | Animating { fast: false }),
            SetTimeScale(FAST_FORWARD),
        );
        bar.once("Slow", |s| matches!(s, Assembling { fast: true } | Animating { fast: true }), SetTimeScale(1.0));
        bar.once("Reload", |s| !matches!(s, Baking | Building), RebuildFabric);
        for (label, feature) in [("Mass", TweakFeature::MassScale), ("Rigidity", TweakFeature::RigidityScale)] {
            bar.entries.push(Entry {
                label,
                shows_in: |s| matches!(s, PhysicsTesting),
                press: Press::Step {
                    down: Crucible(TesterDo(TesterAction::ScaleTweak(feature, 0.9))),
                    up: Crucible(TesterDo(TesterAction::ScaleTweak(feature, 1.1))),
                },
                fixed: false,
            });
        }
        bar
    }

    /// The buttons in `control_state`, in groups to keep together: all the
    /// bar's, those that don't apply disabled, and a tool's while it applies.
    pub fn buttons(&self, control_state: &ControlState) -> Vec<Vec<Button<LabEvent>>> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let enabled = (entry.shows_in)(control_state);
                if !enabled && !entry.fixed {
                    return None;
                }
                let button = |label: &str, event: &LabEvent| {
                    let button = Button::new(label, event.clone());
                    if enabled { button } else { button.disabled() }
                };
                Some(match &entry.press {
                    Press::Once(event) => vec![button(entry.label, event)],
                    Press::Step { down, up } => {
                        vec![button("−", down), Button::caption(entry.label), button("+", up)]
                    }
                })
            })
            .collect()
    }

    fn once(&mut self, label: &'static str, shows_in: fn(&ControlState) -> bool, event: LabEvent) {
        self.entries.push(Entry { label, shows_in, press: Press::Once(event), fixed: true });
    }

    fn tool(&mut self, label: &'static str, shows_in: fn(&ControlState) -> bool, event: LabEvent) {
        self.entries.push(Entry { label, shows_in, press: Press::Once(event), fixed: false });
    }
}
