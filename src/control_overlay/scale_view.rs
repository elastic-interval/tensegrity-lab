use leptos::{component, create_signal, event_target_value, IntoView, Signal, SignalGet, SignalGetUntracked, SignalSet, view, WriteSignal};

#[component]
pub fn ScaleView(
    scale: Signal<f32>,
    set_scale: WriteSignal<f32>,
) -> impl IntoView {
    let (edit, set_edit) = create_signal(false);
    let (scale_value, set_scale_value) = create_signal(scale.get_untracked());
    view! {
        <div class="scale rounded">
            {move || {
                if edit.get() {
                    let assign = move |_| {
                        set_scale.set(scale_value.get());
                        set_edit.set(false);
                    };
                    view! {
                        <div>
                            <label for="scale">Scale (mm):</label>
                            <input
                                type="text"
                                id="scale"
                                value=move || scale.get()
                                on:change=move |ev| {
                                    set_scale_value.set(event_target_value(&ev).parse().unwrap())
                                }
                            />
                            <button on:click=assign>Set</button>
                        </div>
                    }
                } else {
                    let scale_value = move || format!("{0:.1} mm", scale.get());
                    view! {
                        <div>
                            <p on:click=move |_| set_edit.set(true)>"Scale 1:"{scale_value}"."</p>
                        </div>
                    }
                }
            }}
        </div>
    }
}