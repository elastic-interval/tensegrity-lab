use leptos::{component, create_signal, event_target_value, IntoView, ReadSignal, Signal, SignalGet, SignalGetUntracked, SignalSet, view, WriteSignal};
use crate::fabric::FabricStats;

#[component]
pub fn ScaleView(
    scale: Signal<f32>,
    set_scale: WriteSignal<f32>,
    fabric_stats: ReadSignal<Option<FabricStats>>,
) -> impl IntoView {
    let (edit, set_edit) = create_signal(false);
    let (scale_value, set_scale_value) = create_signal(scale.get_untracked());
    view! {
        <div class="bottom-right rounded">
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
                    let scale_value = move || format!("{:.1} mm", scale.get());
                    let stats = move || match fabric_stats.get() {
                        Some(
                            FabricStats {
                                joint_count,
                                push_count,
                                push_range,
                                pull_count,
                                pull_range,
                            },
                        ) => {
                            let scale = scale.get();
                            format!(
                                "The structure has {:?} joints, {:?} pushes ({:.1} mm to {:.1} mm), and {:?} pulls ({:.1} mm to {:.1} mm).",
                                joint_count,
                                push_count,
                                push_range.0 * scale,
                                push_range.1 * scale,
                                pull_count,
                                pull_range.0 * scale,
                                pull_range.1 * scale,
                            )
                        }
                        None => "".to_string(),
                    };
                    view! {
                        <div>
                            <p>{stats}</p>
                            <p on:click=move |_| set_edit.set(true)>"Scale 1:"{scale_value}"."</p>
                        </div>
                    }
                }
            }}
        </div>
    }
}