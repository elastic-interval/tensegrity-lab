use crate::wgpu::button::Button;
use crate::ControlState::*;
use crate::CrucibleAction::*;
use crate::LabEvent::*;
use crate::units::Meters;
use crate::{ControlState, LabEvent, StateChange, TesterAction, TweakFeature};

/// How much faster than real time Fast-forward runs.
pub const FAST_FORWARD: f32 = 20.0;

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
}

/// Every button in the app, in the order they line up along the bottom,
/// each showing only in the stages it applies to.
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
            bar.once("Stop animation", |s| matches!(s, Animating), Crucible(ToViewing));
            bar.once(
                "Start animation",
                |s| matches!(s, Viewing { animation_available: true, .. }),
                Crucible(ToAnimating),
            );
            bar.once("Wind", |s| matches!(s, Viewing { .. } | Animating), Crucible(CycleWind));
            bar.once("Quake", |s| matches!(s, Viewing { .. } | Animating), Crucible(Quake));
            bar.once("Take apart", |s| matches!(s, Viewing { assembly_available: true, .. }), Crucible(Assemble));
        }
        bar.once(
            "Physics test",
            |s| matches!(s, Viewing { physics_testing_available: true, .. }),
            Crucible(ToPhysicsTesting),
        );
        bar.once("Exit test", |s| matches!(s, PhysicsTesting), Crucible(ToViewing));
        bar.once(
            "Movement stats",
            |s| matches!(s, PhysicsTesting),
            Crucible(TesterDo(TesterAction::ToggleMovementSampler)),
        );
        bar.once("Color by role", |s| matches!(s, PhysicsTesting), UpdateState(StateChange::ToggleColorByRole));
        bar.once("Jump", |s| matches!(s, PhysicsTesting), Crucible(CentralizeFabric(Some(Meters(1.0)))));
        bar.once("Background", |_| true, UpdateState(StateChange::ToggleBackground));
        if !model {
            bar.once(
                "Parts",
                |s| matches!(s, Viewing { .. } | Animating | ShowingJoint(_) | ShowingInterval(_)),
                UpdateState(StateChange::TogglePartsView),
            );
        }
        bar.once("Next brick", |s| matches!(s, Baking), NextBrick);
        bar.once("Fast-forward", |s| matches!(s, Building | Assembling { fast: false }), SetTimeScale(FAST_FORWARD));
        bar.once("Slow", |s| matches!(s, Assembling { fast: true }), SetTimeScale(1.0));
        bar.once("Reload", |s| !matches!(s, Baking | Building), RebuildFabric);
        if !model {
            bar.step(
                "Frequency",
                |s| matches!(s, Animating | ShowingJoint(_)),
                Crucible(AdjustAnimationFrequency(0.99)),
                Crucible(AdjustAnimationFrequency(1.01)),
            );
        }
        for (label, feature) in [("Mass", TweakFeature::MassScale), ("Rigidity", TweakFeature::RigidityScale)] {
            bar.step(
                label,
                |s| matches!(s, PhysicsTesting),
                Crucible(TesterDo(TesterAction::ScaleTweak(feature, 0.9))),
                Crucible(TesterDo(TesterAction::ScaleTweak(feature, 1.1))),
            );
        }
        bar
    }

    /// The buttons showing in `control_state`, in groups to keep together.
    pub fn buttons(&self, control_state: &ControlState) -> Vec<Vec<Button<LabEvent>>> {
        self.entries
            .iter()
            .filter(|entry| (entry.shows_in)(control_state))
            .map(|entry| match &entry.press {
                Press::Once(event) => vec![Button::new(entry.label, event.clone())],
                Press::Step { down, up } => vec![
                    Button::new("−", down.clone()),
                    Button::caption(entry.label),
                    Button::new("+", up.clone()),
                ],
            })
            .collect()
    }

    fn once(&mut self, label: &'static str, shows_in: fn(&ControlState) -> bool, event: LabEvent) {
        self.entries.push(Entry { label, shows_in, press: Press::Once(event) });
    }

    fn step(&mut self, label: &'static str, shows_in: fn(&ControlState) -> bool, down: LabEvent, up: LabEvent) {
        self.entries.push(Entry { label, shows_in, press: Press::Step { down, up } });
    }
}
