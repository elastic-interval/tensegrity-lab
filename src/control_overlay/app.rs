use std::sync::mpsc::Sender;

use leptos::{CollectView, component, create_effect, create_signal, IntoView, Memo, ReadSignal, SignalGet, SignalSet, SignalUpdate, view, WriteSignal};

use crate::control_overlay::action::Action;
use crate::control_state::ControlState;

#[component]
pub fn ControlOverlayApp(
    fabric_list: Memo<Vec<String>>,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    log::info!(
        "control overlay app"
    );
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
                                        <li>
                                        <button on:click=move |_ev| set_name.set(n.clone())>
                                            {label}
                                        </button>
                                        </li>
                                    }
            })
            .collect_view()
    };

    view! {
        <div class="inset">
        {move||{
            match control_state.get() {
                ControlState::Choosing => {
                    view! {
                        <div>
                            <ul>{list}</ul>
                        </div>
                    }
    
                }
                ControlState::Viewing => {
                    view!{<div class="hidden"></div>}
                }
                ControlState::ShowingInterval(interval) => {
                    view!{<div><pre>{
                        format!("{:#?}", interval)
                    }</pre></div>}
                }} }
        }
        </div>
    }
}