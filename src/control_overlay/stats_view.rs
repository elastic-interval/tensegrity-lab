use leptos::{component, create_signal, event_target_value, IntoView, ReadSignal, Signal, SignalGet, SignalGetUntracked, SignalSet, view, WriteSignal};
use crate::fabric::FabricStats;

#[component]
pub fn StatsView(
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
                                max_height,
                                push_count,
                                push_total,
                                push_range,
                                pull_count,
                                pull_range,
                                pull_total,
                            },
                        ) => {
                            let scale = scale.get();
                            format!(
                                "The structure has {:?} joints (up to height {:.0}mm), {:?} pushes ({:.1}mm to {:.1}mm, total {:.2}m), and {:?} pulls ({:.1}mm to {:.1}mm, total {:.2}m).",
                                joint_count,
                                max_height * scale,
                                push_count,
                                push_range.0 * scale,
                                push_range.1 * scale,
                                push_total * scale / 1000.0,
                                pull_count,
                                pull_range.0 * scale,
                                pull_range.1 * scale,
                                pull_total * scale / 1000.0,
                            )
                        }
                        None => "".to_string(),
                    };
                    view! {
                        <div>
                            <p>"Scale 1:"{scale_value}". "<button on:click=move |_| set_edit.set(true)>Set scale</button></p>
                            <p>{stats}</p>
                        </div>
                    }
                }
            }}
        </div>
    }
}