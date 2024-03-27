use std::sync::mpsc::Sender;

use leptos::{CollectView, component, create_effect, create_signal, IntoView, Memo, ReadSignal, SignalGet, SignalGetUntracked, SignalSet, SignalUpdate, view, WriteSignal};

use crate::control_overlay::action::Action;
use crate::control_state::ControlState;
use crate::fabric::interval::{Interval, Material, Role, Span};

#[component]
pub fn ControlOverlayApp(
    fabric_list: Memo<Vec<String>>,
    materials: Memo<[Material; 5]>,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let (name, set_name) = create_signal("".to_string());
    create_effect(move |_| {
        if !name.get().is_empty() {
            set_control_state.update(|state| *state = ControlState::Viewing);
            actions_tx
                .send(Action::LoadFabric(name.get()))
                .expect("failed to send action");
        }
    });

    let list = move || {
        fabric_list
            .get()
            .into_iter()
            .map(|n| {
                let label = n.clone();
                view! {
                    <button on:click=move |_ev| set_name.set(n.clone())>
                        {label}
                    </button>
                }
            })
            .collect_view()
    };

    let formatted_interval = move |interval: Interval| {
        let [left, right] = match &materials.get_untracked()[interval.material].role {
            Role::Pull => ['\u{21E8}', '\u{21E6}'],
            Role::Push => ['\u{21E6}', '\u{21E8}'],
        };
        format!("(J{:?}) {} ({}) {} (J{:?})",
                interval.alpha_index + 1,
                left,
                match interval.span {
                    Span::Fixed { length } => { length.to_string() }
                    _ => { "?".to_string() }
                },
                right,
                interval.omega_index + 1,
        )
    };

    view! {
        {move||{
            match control_state.get() {
                ControlState::Choosing => {
                    view! {<div class="choice"><div class="list"><h1>Designs</h1>{list}</div></div>}
                }
                ControlState::Viewing => {
                    view!{<div class="hidden"></div>}
                }
                ControlState::ShowingInterval(interval) => {
                    view!{<div class="title"><h1>{formatted_interval(interval)}</h1></div>}
                }} 
            }
        }
    }
}