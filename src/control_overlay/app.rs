use std::sync::mpsc::Sender;

use leptos::{
    component, create_signal, event_target_value, IntoView, ReadSignal, SignalGet,
    SignalSet, view,
};

use crate::control_overlay::action::Action;
use crate::fabric::interval::Interval;

#[component]
pub fn ControlOverlayApp(
    control_state: ReadSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let pre_text = move || format!("{:#?}", control_state.get());
    let (category, set_category) = create_signal("Art".to_string());
    let (subname, set_subname) = create_signal("Halo by Crane".to_string());
    let fabric_name = move || vec![category.get(), subname.get()];
    let on_run_fabric_click = move |_ev| {
        actions_tx
            .send(Action::LoadFabric(fabric_name()))
            .expect("failed to send action");
    };
    view! {
        <div class="inset">
            <section class="left">
                <p class="input_group">
                    <input
                        type="text"
                        value=move || category.get()
                        on:change=move |ev| { set_category.set(event_target_value(&ev)); } />
                    <input
                        type="text"
                        value=move || subname.get()
                        on:change=move |ev| { set_subname.set(event_target_value(&ev)); } />
                </p>
                <p class="input_group">
                    <button on:click=on_run_fabric_click>
                        Run Fabric
                    </button>
                </p>
                <pre>{pre_text}</pre>
            </section>
            <section class="right">
                <h3>
                    Right hand side content here.
                </h3>
            </section>
        </div>
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlState {
    pub(crate) picked_interval: Option<Interval>,
}
