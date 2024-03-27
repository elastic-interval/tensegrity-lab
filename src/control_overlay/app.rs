use std::sync::mpsc::Sender;

use leptos::{CollectView, component, create_effect, create_signal, event_target_value, IntoView, Memo, ReadSignal, SignalGet, SignalGetUntracked, SignalSet, SignalUpdate, view, WriteSignal};

use crate::control_overlay::action::Action;
use crate::control_state::{ControlState, IntervalDetails};
use crate::fabric::interval::Role;

#[component]
pub fn ControlOverlayApp(
    fabric_list: Memo<Vec<String>>,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let (name, set_name) = create_signal("".to_string());
    create_effect(move |_| {
        let name = name.get();
        if !name.is_empty() {
            set_control_state.update(|state| *state = ControlState::Viewing);
            actions_tx
                .send(Action::LoadFabric(name))
                .expect("failed to send action");
        }
    });

    let (scale, set_scale) = create_signal(100.0);

    let (assigned_length, set_assigned_length) = create_signal(100.0);

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

    let formatted_interval = move |interval: &IntervalDetails| {
        let [left, right] = match &interval.role {
            Role::Pull => ['\u{21E8}', '\u{21E6}'],
            Role::Push => ['\u{21E6}', '\u{21E8}'],
        };
        format!("J{:?} {} {:.1}mm {} J{:?}",
                interval.alpha_index + 1,
                left,
                interval.length * scale.get(),
                right,
                interval.omega_index + 1,
        )
    };

    view! {
        {move || 
            match control_state.get() {
                ControlState::Choosing => {
                    view! {<div class="choice"><div class="list"><h1>Designs</h1>{list}</div></div>}
                }
                ControlState::Viewing => {
                    view!{<div class="hidden"></div>}
                }
                ControlState::ShowingInterval(interval_details) => {
                    view!{
                        <div class="title">
                             <div on:click=move |_ev| set_control_state.set(ControlState::SettingLength(interval_details))>{formatted_interval(&interval_details)}</div>
                        </div>
                    }
                }
                ControlState::SettingLength(interval_details) => {
                    view!{
                        <div class="title">
                            <label for="length">Length(mm): </label>
                            <input type="text" id="length" 
                                    value={move || assigned_length.get()}
                                    on:change=move |ev| { set_assigned_length.set(event_target_value(&ev).parse().unwrap()); } 
                            />
                            <button on:click=move|_| {
                                set_scale.set(assigned_length.get()/interval_details.length);
                                set_control_state.set(ControlState::ShowingInterval(interval_details));
                            }>
                                Assign   
                            </button>
                        </div>
                    }
                }
            } 
        }
    }
}