use leptos::{component, create_signal, event_target_value, IntoView, ReadSignal, Signal, SignalGet, SignalSet, view, WriteSignal};
use crate::messages::{ControlState, IntervalDetails};

#[component]
pub fn DetailsView(
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    scale: Signal<f32>,
    set_scale: WriteSignal<f32>,
) -> impl IntoView {
    let (assigned_length, set_assigned_length) = create_signal(100.0);
    view! {
        <div class="details">
            {move || match control_state.get() {
                ControlState::Viewing => {
                    view! { <div>"Click near a joint to highlight it."</div> }
                }
                ControlState::ShowingJoint(joint_index) => {
                    view! {
                        <div>
                            <div>
                                "Joint highlighted:" {move || format!("J{}", joint_index + 1)}
                            </div>
                            <div>Click near an interval to show its details.</div>
                        </div>
                    }
                }
                ControlState::ShowingInterval(interval_details) => {
                    let formatted_interval = move |interval: &IntervalDetails| {
                        format!(
                            "J{:?} {:?}({:.1}mm)  J{:?}",
                            interval.alpha_index + 1,
                            interval.role,
                            interval.length * scale.get(),
                            interval.omega_index + 1,
                        )
                    };
                    let to_setting_length = move |_ev| {
                        set_control_state.set(ControlState::SettingLength(interval_details))
                    };
                    view! {
                        <div>
                            <div>
                                "Interval highlighted:" {formatted_interval(&interval_details)}
                            </div>
                            <div on:click=to_setting_length>"Set this interval's length"</div>
                        </div>
                    }
                }
                ControlState::SettingLength(interval_details) => {
                    let assign = move |_| {
                        set_scale.set(assigned_length.get() / interval_details.length);
                        set_control_state.set(ControlState::ShowingInterval(interval_details));
                    };
                    view! {
                        <div>
                            <label for="length">Length(mm):</label>
                            <input
                                type="text"
                                id="length"
                                value=move || assigned_length.get()
                                on:change=move |ev| {
                                    set_assigned_length.set(event_target_value(&ev).parse().unwrap())
                                }
                            />
                            <button on:click=assign>Assign</button>
                        </div>
                    }
                }
            }}
        </div>
    }
}