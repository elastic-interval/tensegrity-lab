use std::sync::mpsc::Sender;

use leptos::{CollectView, component, create_effect, create_signal, event_target_value, IntoView, Memo, ReadSignal, SignalGet, SignalSet, SignalUpdate, view, WriteSignal};
use leptos_use::storage::use_local_storage;
use leptos_use::utils::FromToStringCodec;

use crate::control_overlay::action::Action;
use crate::control_state::{ControlState, IntervalDetails};
use crate::crucible::CrucibleAction;

#[component]
pub fn ControlOverlayApp(
    fabric_list: Memo<Vec<String>>,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let (name, set_name, _) = use_local_storage::<String, FromToStringCodec>("name");
    let (scale, set_scale, _) = use_local_storage::<f32, FromToStringCodec>("scale");
    let name_sender = actions_tx.clone();
    let activate_sender = actions_tx.clone();
    create_effect(move |_| {
        let name = name.get();
        if !name.is_empty() {
            set_control_state.update(|state| *state = ControlState::Viewing);
            name_sender
                .send(Action::LoadFabric(name))
                .expect("failed to send action");
        }
    });
    create_effect(move |_| {
        let control_state = control_state.get();
        if matches!(control_state, ControlState::Animating) {
            activate_sender
                .send(Action::Crucible(CrucibleAction::ActivateMuscles))
                .expect("failed to send action");
        }
    });

    let (assigned_length, set_assigned_length) = create_signal(100.0);

    let list = move || {
        fabric_list
            .get()
            .into_iter()
            .map(|n| {
                let label = format!("\"{}\"", n.clone());
                view! {
                    <div class="item" on:click=move |_ev| set_name.set(n.clone())>
                        {label}
                    </div>
                }
            })
            .collect_view()
    };

    let formatted_interval = move |interval: &IntervalDetails| {
        format!("J{:?} {:?}({:.1}mm)  J{:?}",
                interval.alpha_index + 1,
                interval.role,
                interval.length * scale.get(),
                interval.omega_index + 1,
        )
    };

    view! {
        {move ||
            match control_state.get() {
                ControlState::Choosing => {
                    view! {<div class="list">{list}</div>}
                }
                ControlState::Viewing => {
                    let to_choosing =
                        move |_ev| set_control_state.set(ControlState::Choosing);
                    let to_animate =
                        move |_ev| set_control_state.set(ControlState::Animating);
                    view!{
                        <div class="title">
                            <div>{move || format!("\"{}\"", name.get())}</div>
                            <div class="tiny">
                                <span on:click=to_choosing>choose another</span>
                                <span on:click=to_animate>animate</span>
                            </div>
                        </div>
                    }
                }
                ControlState::Animating => {
                    view!{
                        <div class="title">
                            <div>{move || format!("Animating \"{}\"", name.get())}</div>
                        </div>
                    }
                }
                ControlState::ShowingJoint(joint_index) => {
                    view!{
                        <div class="title">
                            <div>{move || format!("J{}", joint_index+1)}</div>
                            <div class="tiny">esc</div>
                        </div>
                    }
                }
                ControlState::ShowingInterval(interval_details) => {
                    let to_setting_length =
                        move |_ev| set_control_state.set(ControlState::SettingLength(interval_details));
                    view!{
                        <div class="title">
                            <div>{formatted_interval(&interval_details)}</div>
                            <div class="tiny" on:click=to_setting_length>set</div>
                        </div>
                    }
                }
                ControlState::SettingLength(interval_details) => {
                    let assign = move|_| {
                        set_scale.set(assigned_length.get()/interval_details.length);
                        set_control_state.set(ControlState::ShowingInterval(interval_details));
                    };
                    view!{
                        <div class="title">
                            <label for="length">Length(mm): </label>
                            <input type="text" id="length"
                                    value={move || assigned_length.get()}
                                    on:change=move |ev|
                                        set_assigned_length.set(event_target_value(&ev).parse().unwrap())
                            />
                            <button on:click=assign>Assign</button>
                        </div>
                    }
                }
            }
        }
    }
}