use leptos::{component, create_effect, create_signal, event_target_value, IntoView, ReadSignal, Signal, SignalGet, SignalSet, view, WriteSignal};
use crate::fabric::interval::Role;

use crate::messages::{ControlState, IntervalDetails, JointDetails};

#[component]
pub fn DetailsView(
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    scale: Signal<f32>,
    set_scale: WriteSignal<f32>,
) -> impl IntoView {
    let (assigned_length, set_assigned_length) = create_signal(100.0);
    let (joint_height, set_joint_height) = create_signal(1f32);

    create_effect(move |_| {
        if let ControlState::ShowingJoint(JointDetails { height, .. }) = control_state.get() {
            set_joint_height.set(height * scale.get())
        }
    });
    view! {
        <div class="details rounded">
            {move || match control_state.get() {
                ControlState::Viewing => {
                    view! {
                        <div>
                            <p>
                                "To select a joint, click near it with the right mouse button."
                            </p>
                        </div>
                    }
                }
                ControlState::ShowingJoint(joint_details) => {
                    view! {
                        <div>
                            <p>
                                "Joint "
                                <b>{move || format!("\"J{}\"", joint_details.index + 1)}</b>
                                " and its adjacent intervals are highlighted."
                            </p>
                            <p>
                                "It is located "
                                <b>{move || format!("{0:.0}mm", joint_height.get())}</b>
                                " above the ground."
                            </p>
                            <p>"Click near one of its intervals to select it and show details."</p>
                            <p>"Click again to jump across the interval to the other joint."</p>
                        </div>
                    }
                }
                ControlState::ShowingInterval(interval_details) => {
                    let role = move || match interval_details.role {
                        Role::Push => "strut",
                        Role::Pull => "cable",
                        Role::Spring => "spring",
                    };
                    let formatted_interval = move |interval: &IntervalDetails| {
                        format!("from J{} to J{}", interval.near_joint + 1, interval.far_joint + 1)
                    };
                    let to_setting_length = move |_ev| {
                        set_control_state.set(ControlState::SettingLength(interval_details))
                    };
                    let scale_value = move || format!("{0:.1}mm", scale.get());
                    let length = move || format!("{0:.1}mm", interval_details.length * scale.get());
                    view! {
                        <div>
                            <p>
                                "The highlighted green interval is a " <b>{role}</b>" "<b>{formatted_interval(&interval_details)}</b> "."
                            </p>
                            <p>"Its length is " <b>{length}</b> "."</p>
                            <p on:click=to_setting_length>
                                "The scale is currently 1:"{scale_value}". "
                                "Click here to set this interval's length to set global scale."
                            </p>
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
                                    set_assigned_length
                                        .set(event_target_value(&ev).parse().unwrap())
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