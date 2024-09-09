use leptos::{component, create_effect, create_signal, IntoView, ReadSignal, Signal, SignalGet, SignalSet, view};

use crate::fabric::interval::Role;
use crate::messages::{ControlState, JointDetails};

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
                            <p>"To select a joint, right-click on it."</p>
                        </div>
                    }
                }
                ControlState::ShowingJoint(joint_details) => {
                    view! {
                        <div>
                            <p>"Joint "<b>{move || format!("\"J{}\"", joint_details.index + 1)}</b></p>
                            <p>"Height: "<b>{move || format!("{0:.0} mm", joint_height.get())}</b></p>
                            <p>"Click an interval for details, or right-click for an adjacent joint."</p>
                        </div>
                    }
                }
                ControlState::ShowingInterval(interval_details) => {
                    let role = move || match interval_details.role {
                        Role::Push => "strut",
                        Role::Pull => "cable",
                        Role::Spring => "spring",
                    };
                    let joint = move |index| {
                        format!("J{}", index)
                    };
                    let length = move || {
                        format!("{0:.1} mm", interval_details.length * scale.get())
                    };
                    view! {
                        <div>
                            <p>"Joint "<b>{joint(interval_details.near_joint)}</b></p>
                            <p>"The green interval is a " <b>{role}</b> " to " <b>{joint(interval_details.far_joint)}</b> "."</p>
                            <p>"Length: " <b>{length}</b> "."</p>
                            <p>"Click it again to jump across the interval to "{joint(interval_details.far_joint)}</p>
                        </div>
                    }
                }
            }}
        </div>
    }
}