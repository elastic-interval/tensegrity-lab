use leptos::{component, view, IntoView, ReadSignal, SignalGet};

use crate::fabric::interval::Interval;

#[component]
pub fn ControlOverlayApp(control_state: ReadSignal<ControlState>) -> impl IntoView {
    let text = move || format!("{:#?}", control_state.get());
    view! {
        <div class="inset">
            <pre>{text}</pre>
        </div>
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlState {
    pub(crate) picked_interval: Option<Interval>,
}
