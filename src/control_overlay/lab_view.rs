use leptos::{component, IntoView, ReadSignal, SignalGet, SignalUpdate, view, WriteSignal};

#[component]
pub fn LabView(
    animated: ReadSignal<bool>,
    set_animated: WriteSignal<bool>,
) -> impl IntoView {
    let toggle = move |_| set_animated.update(|x| *x = !*x);
    view! {
        <div class="bottom-left rounded">
            <div on:click=toggle>
                {move || {
                    if animated.get() { "Stop Muscle Cycle" } else { "Start Muscle Cycle" }
                }}
            </div>
        </div>
    }
}