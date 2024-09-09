use codee::string::FromToStringCodec;
use leptos::*;
use leptos_use::storage::use_local_storage;
use winit::event_loop::EventLoopProxy;

use crate::control_overlay::details_view::DetailsView;
use crate::control_overlay::fabric_choice_view::FabricChoiceView;
use crate::control_overlay::lab_view::LabView;
use crate::control_overlay::scale_view::ScaleView;
use crate::crucible::CrucibleAction::Experiment;
use crate::crucible::LabAction;
use crate::messages::{ControlState, LabEvent};

mod fabric_choice_view;
mod details_view;
mod scale_view;
mod lab_view;

#[component]
pub fn ControlOverlayApp(
    fabric_list: Vec<String>,
    control_state: ReadSignal<ControlState>,
    lab_control: ReadSignal<bool>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
    let (fabric_name, set_fabric_name, _) = use_local_storage::<String, FromToStringCodec>("fabric");
    let (scale, set_scale, _) = use_local_storage::<f32, FromToStringCodec>("scale");
    let (animated, set_animated) = create_signal(false);

    let muscle_proxy = event_loop_proxy.clone();
    create_effect(move |_| event_loop_proxy.send_event(LabEvent::LoadFabric(fabric_name.get())).unwrap());
    
    create_effect(move |_| {
        muscle_proxy.send_event(LabEvent::Crucible(Experiment(LabAction::MuscleTest(animated.get())))).unwrap()
    });

    view! {
        <div>
            <FabricChoiceView
                fabric_list=fabric_list
                fabric_name=fabric_name
                set_fabric_name=set_fabric_name
            />
            <DetailsView control_state=control_state scale=scale />
            <ScaleView scale=scale set_scale=set_scale />
            <Show when=move || lab_control.get() fallback=|| view! { <div /> }>
                <LabView animated=animated set_animated=set_animated />
            </Show>
        </div>
    }
}
