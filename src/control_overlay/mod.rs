use codee::string::FromToStringCodec;
use leptos::*;
use leptos_use::storage::use_local_storage;
use winit::event_loop::EventLoopProxy;

use crate::control_overlay::details_view::DetailsView;
use crate::control_overlay::menu_view::MenuView;
use crate::messages::{ControlState, LabEvent};

mod menu_view;
mod details_view;

#[component]
pub fn ControlOverlayApp(
    fabric_list: Vec<String>,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
    let (fabric_name, set_fabric_name, _) = use_local_storage::<String, FromToStringCodec>("fabric");
    let (scale, set_scale, _) = use_local_storage::<f32, FromToStringCodec>("scale");

    create_effect(move |_| event_loop_proxy.send_event(LabEvent::LoadFabric(fabric_name.get())).unwrap());

    view! {
        <div>
            <MenuView
                fabric_list=fabric_list
                fabric_name=fabric_name
                set_fabric_name=set_fabric_name
            />
            <DetailsView
                control_state=control_state
                set_control_state=set_control_state
                scale=scale
                set_scale=set_scale
            />
        </div>
    }
}
