use crate::fabric::interval::Role;
use crate::fabric::FabricStats;
use crate::messages::ControlState;
use cgmath::Point3;
use leptos::either::EitherOf4;
use leptos::prelude::*;
use leptos::{component, view, IntoView};

#[component]
pub fn DetailsView(
    control_state: ReadSignal<ControlState>,
    fabric_stats: ReadSignal<Option<FabricStats>>,
) -> impl IntoView {
    let location_format = move |location: Point3<f32>| {
        let Point3 { x, y, z } = location;
        format!("({x:.3}, {y:.3}, {z:.3})")
    };
    view! {
        <div class="bottom-center rounded">
            {move || match control_state.get() {
                ControlState::Waiting => EitherOf4::A(view! { <div>"Waiting to finish..."</div> }),
                ControlState::Viewing => {
                    EitherOf4::B(
                        view! {
                            <div>
                                <p>"To select a joint, right-click on it."</p>
                            </div>
                        },
                    )
                }
                ControlState::ShowingJoint(joint_details) => {
                    EitherOf4::C(
                        view! {
                            <div>
                                <p>
                                    "Joint "
                                    <b>{move || format!("\"J{}\"", joint_details.index + 1)}</b>
                                </p>
                                <p>"Location: "<b>{location_format(joint_details.location)}</b></p>
                                <p>
                                    "Click an interval for details, or right-click for an adjacent joint."
                                </p>
                            </div>
                        },
                    )
                }
                ControlState::ShowingInterval(interval_details) => {
                    let role = move || match interval_details.role {
                        Role::Push => "strut",
                        Role::Pull => "cable",
                        Role::Spring => "spring",
                    };
                    let joint = move |index| { format!("J{}", index) };
                    let length = move || {
                        if let Some(stats) = fabric_stats.get() {
                            format!("{0:.1} mm", interval_details.length * stats.scale)
                        } else {
                            "?".to_string()
                        }
                    };
                    EitherOf4::D(
                        view! {
                            <div>
                                <p>"Joint "<b>{joint(interval_details.near_joint + 1)}</b></p>
                                <p>
                                    "The green interval is a " <b>{role}</b> " to "
                                    <b>{joint(interval_details.far_joint + 1)}</b> "."
                                </p>
                                <p>"Length: " <b>{length}</b> "."</p>
                                <p>
                                    "Click it again to jump across the interval to "
                                    {joint(interval_details.far_joint + 1)}
                                </p>
                            </div>
                        },
                    )
                }
            }}
        </div>
    }
}
