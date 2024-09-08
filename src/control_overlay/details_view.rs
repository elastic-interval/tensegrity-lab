use leptos::{component, create_effect, create_signal, IntoView, ReadSignal, Signal, SignalGet, SignalSet, view};

use crate::fabric::interval::Role;
use crate::messages::{ControlState, IntervalDetails, JointDetails};

#[component]
pub fn DetailsView(
    control_state: ReadSignal<ControlState>,
    scale: Signal<f32>,
) -> impl IntoView {
    let (joint_height, set_joint_height) = create_signal(1f32);

    create_effect(move |_| {
        if let ControlState::ShowingJoint(JointDetails { height, .. }) = control_state.get() {
            set_joint_height.set(height * scale.get())
        }
    });
    view! {
        <div class="top-right rounded">
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
                    let length = move || format!("{0:.1} mm", interval_details.length * scale.get());
                    view! {
                        <div>
                            <p>
                                "The highlighted green interval is a " <b>{role}</b>" "<b>{formatted_interval(&interval_details)}</b> "."
                            </p>
                            <p>"Its length is " <b>{length}</b> "."</p>
                        </div>
                    }
                }
            }}
        </div>
    }
}