use leptos::{component, create_effect, view, IntoView, ReadSignal, SignalGet};

use crate::control_overlay::Message;

#[component]
pub fn ControlOverlayApp(message: ReadSignal<Message>) -> impl IntoView {
    let text = move || match message.get() {
        Message::PickedInterval(interval) => format!("{interval:#?}"),
        Message::Init => "".to_string(),
    };
    view! {
        <div>
            <pre>{text}</pre>
        </div>
    }
}
