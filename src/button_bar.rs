use crate::wgpu::button::Button;
use crate::ControlState::*;
use crate::CrucibleAction::*;
use crate::LabEvent::*;
use crate::fabric::variant::Variant;
use crate::units::Meters;
use crate::build::dsl::fabric_library::FabricName;
use crate::{ControlState, LabEvent, RunStyle, StateChange, TesterAction, TweakFeature};
use strum::IntoEnumIterator;

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
    /// Shown only while the wobble table quakes, or only while it doesn't.
    quaking: Option<bool>,
}

/// Every button in the app, in the order they line up along the bottom,
/// each showing only in the stages it applies to.
pub struct ButtonBar {
    entries: Vec<Entry>,
}

impl ButtonBar {
    /// A fabric as designed has everything but the quake, which is the
    /// wobble table under a piece on one. Only one as designed animates,
    /// takes the wind, is taken apart or shows its parts; a model, at
    /// `--model-scale`, is for building from: none of these.
    pub fn new(variant: &Variant) -> Self {
        let mut bar = ButtonBar { entries: Vec::new() };
        bar.once("Back to joint", |s| matches!(s, ShowingInterval(_)), UpdateState(StateChange::BackToJoint));
        bar.once("Deselect", |s| matches!(s, ShowingJoint(_)), Crucible(ClearSelection));
        if variant.as_designed() {
            bar.once(
                "Start animation",
                |s| matches!(s, Viewing { animation_available: true, .. }),
                Crucible(ToAnimating),
            );
            bar.entries.last_mut().unwrap().quaking = Some(false);
            bar.once("Stop animation", |s| matches!(s, Animating { .. }), Crucible(ToViewing));
        }
        if variant.as_designed() {
            bar.once("Wind", |s| matches!(s, Viewing { .. } | Animating { .. }), Crucible(CycleWind));
        }
        if variant.on_table() {
            let table = |s: &ControlState| matches!(s, Viewing { .. });
            bar.once("Quake", table, Crucible(Quake));
            bar.entries.last_mut().unwrap().quaking = Some(false);
            bar.once("Stop quake", table, Crucible(Quake));
            bar.entries.last_mut().unwrap().quaking = Some(true);
            bar.once("Demo", table, Crucible(QuakeDemo));
            bar.entries.last_mut().unwrap().quaking = Some(true);
        }
        if variant.as_designed() {
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
        if variant.as_designed() {
            bar.once(
                "Parts",
                |s| matches!(s, Viewing { .. } | Animating { .. } | ShowingJoint(_) | ShowingInterval(_)),
                UpdateState(StateChange::TogglePartsView),
            );
        }
        bar.once("Next brick", |s| matches!(s, Baking), NextBrick);
        if variant.as_designed() {
            bar.once(
                "Fast-forward",
                |s| matches!(s, Building | Assembling { fast: false } | Animating { fast: false }),
                SetTimeScale(FAST_FORWARD),
            );
            bar.once("Slow", |s| matches!(s, Assembling { fast: true } | Animating { fast: true }), SetTimeScale(1.0));
        }
        bar.once("Choose", |s| matches!(s, Viewing { .. }), ToggleChoosing);
        bar.entries.last_mut().unwrap().quaking = Some(false);
        for (label, feature) in [("Mass", TweakFeature::MassScale), ("Rigidity", TweakFeature::RigidityScale)] {
            bar.entries.push(Entry {
                label,
                shows_in: |s| matches!(s, PhysicsTesting),
                press: Press::Step {
                    down: Crucible(TesterDo(TesterAction::ScaleTweak(feature, 0.9))),
                    up: Crucible(TesterDo(TesterAction::ScaleTweak(feature, 1.1))),
                },
                quaking: None,
            });
        }
        bar
    }

    /// The buttons that apply in `control_state`, the wobble table
    /// `quaking` or not, in groups to keep together; or while `choosing`, a
    /// button for each fabric, and Back.
    pub fn buttons(&self, control_state: &ControlState, quaking: bool, choosing: bool) -> Vec<Vec<Button<LabEvent>>> {
        if choosing {
            let fabrics = FabricName::iter().map(|fabric_name| {
                let run = Run(RunStyle::Fabric { fabric_name, record: None, export_fps: 100.0 });
                vec![Button::new(&fabric_name.to_string(), run)]
            });
            return fabrics.chain([vec![Button::new("Back", ToggleChoosing)]]).collect();
        }
        self.entries
            .iter()
            .filter(|entry| (entry.shows_in)(control_state) && entry.quaking.is_none_or(|when| when == quaking))
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
        self.entries.push(Entry { label, shows_in, press: Press::Once(event), quaking: None });
    }
}
