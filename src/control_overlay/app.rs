use std::sync::mpsc::Sender;

use leptos::{
    component, create_signal, event_target_value, view, IntoView, ReadSignal, SignalGet, SignalSet,
};

use crate::control_overlay::action::Action;
use crate::fabric::interval::Interval;

#[component]
pub fn ControlOverlayApp(
    control_state: ReadSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let pre_text = move || format!("{:#?}", control_state.get());
    let load_fabric = move |name: Vec<String>| {
        actions_tx
            .send(Action::LoadFabric(name))
            .expect("failed to send action");
    };
    let (category, set_category) = create_signal("Art".to_string());
    let (subname, set_subname) = create_signal("Halo by Crane".to_string());
    view! {
        <div class="inset">
            <input
                type="text"
                value=category.get()
                on:change=move |ev| { set_category.set(event_target_value(&ev)); } />
            <input
                type="text"
                value=subname.get()
                on:change=move |ev| { set_subname.set(event_target_value(&ev)); } />
            <button on:click=move |_ev| { load_fabric(vec![category.get(), subname.get()]) }>
                Load Fabric
            </button>
            <pre>{pre_text}</pre>
        </div>
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlState {
    pub(crate) picked_interval: Option<Interval>,
}
