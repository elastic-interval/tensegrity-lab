use codee::string::FromToStringCodec;
use leptos::*;
use leptos_use::storage::use_local_storage;
use winit::event_loop::EventLoopProxy;

use crate::control_overlay::details_view::DetailsView;
use crate::control_overlay::menu::MenuItem;
use crate::control_overlay::menu_view::MenuView;
use crate::messages::{ControlState, LabEvent};

pub mod menu;
mod menu_view;
mod details_view;

#[component]
pub fn ControlOverlayApp(
    initial_menu_item: MenuItem,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
    let (menu_choice, set_menu_choice) = create_signal(vec![initial_menu_item]);
    let (scale, set_scale, _) = use_local_storage::<f32, FromToStringCodec>("scale");

    create_effect(move |_| {
        let choice = menu_choice.get().last().unwrap().clone();
        event_loop_proxy.send_event(LabEvent::SendMenuEvent(choice)).unwrap()
    });

    view! {
        <div>
            <MenuView menu_choice=menu_choice set_menu_choice=set_menu_choice />
            <DetailsView
                control_state=control_state
                set_control_state=set_control_state
                scale=scale
                set_scale=set_scale
            />
        </div>
    }
}
