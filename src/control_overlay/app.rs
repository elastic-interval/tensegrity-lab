use std::sync::mpsc::Sender;

use leptos::{CollectView, component, create_effect, create_signal, For, IntoView, Memo, ReadSignal, SignalGet, SignalSet, view};

use crate::control_overlay::action::Action;
use crate::fabric::interval::Interval;

#[component]
pub fn ControlOverlayApp(
    fabric_list: Memo<Vec<String>>,
    control_state: ReadSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let pre_text = move || format!("{:#?}", control_state.get());
    let (name, set_name) = create_signal("".to_string());
    create_effect(move |_| {
        if !name.get().is_empty() {
            actions_tx
                .send(Action::LoadFabric(name.get()))
                .expect("failed to send action");
        }
    });
    
    view! {
        <div class="inset">
            <p>
                <ul>
                    {
                        fabric_list.get().into_iter().map(|n| {
                        let label = n.clone();
                        view! {
                            <li>
                            <button on:click=move |_ev| set_name.set(n.clone())>
                                {label}
                            </button>
                            </li>
                        }})
                        .collect_view()
                    }
                </ul>
            </p>
            <h1>{name}</h1>
            <pre>{pre_text}</pre>
        </div>
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlState {
    pub(crate) picked_interval: Option<Interval>,
}
