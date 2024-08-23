use codee::string::FromToStringCodec;
use leptos::*;
use leptos_use::storage::use_local_storage;
use winit::event_loop::EventLoopProxy;

use crate::control_overlay::menu::Menu;
use crate::control_overlay::menu_view::MenuView;
use crate::messages::{ControlState, IntervalDetails, LabEvent};

pub mod menu;
mod menu_view;

#[component]
pub fn ControlOverlayApp(
    menu: ReadSignal<Menu>,
    control_state: ReadSignal<ControlState>,
    set_control_state: WriteSignal<ControlState>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
    let (menu_choice, set_menu_choice) = create_signal(menu.get_untracked().root().clone());
    let (scale, set_scale, _) = use_local_storage::<f32, FromToStringCodec>("scale");
    let (assigned_length, set_assigned_length) = create_signal(100.0);

    create_effect(move |_| {
        let menu_choice = menu_choice.get();
        event_loop_proxy.send_event(LabEvent::SendMenuEvent(menu_choice)).unwrap()
    });

    let formatted_interval = move |interval: &IntervalDetails| {
        format!("J{:?} {:?}({:.1}mm)  J{:?}",
                interval.alpha_index + 1,
                interval.role,
                interval.length * scale.get(),
                interval.omega_index + 1,
        )
    };
    view! {
        {move ||
            match control_state.get() {
                ControlState::Choosing => {
                    view! {
                        <div class="list">
                            <MenuView menu={menu} menu_choice={menu_choice} set_menu_choice={set_menu_choice}/>
                        </div>
                    }
                }
                ControlState::Viewing => {
                    let to_choosing =
                        move |_ev| set_control_state.set(ControlState::Choosing);
                    view!{
                        <div class="title">
                            <div>{move || format!("\"{}\"", menu_choice.get().label)}</div>
                            <div class="tiny" on:click=to_choosing>choose</div>
                        </div>
                    }
                }
                ControlState::ShowingJoint(joint_index) => {
                    web_sys::console::log_1(&"joint".into());
                    view!{
                        <div class="title">
                            <div>{move || format!("J{}", joint_index+1)}</div>
                            <div class="tiny">esc</div>
                        </div>
                    }
                }
                ControlState::ShowingInterval(interval_details) => {
                    let to_setting_length =
                        move |_ev| set_control_state.set(ControlState::SettingLength(interval_details));
                    view!{
                        <div class="title">
                            <div>{formatted_interval(&interval_details)}</div>
                            <div class="tiny" on:click=to_setting_length>set</div>
                        </div>
                    }
                }
                ControlState::SettingLength(interval_details) => {
                    let assign = move|_| {
                        set_scale.set(assigned_length.get()/interval_details.length);
                        set_control_state.set(ControlState::ShowingInterval(interval_details));
                    };
                    view!{
                        <div class="title">
                            <label for="length">Length(mm): </label>
                            <input type="text" id="length"
                                    value={move || assigned_length.get()}
                                    on:change=move |ev|
                                        set_assigned_length.set(event_target_value(&ev).parse().unwrap())
                            />
                            <button on:click=assign>Assign</button>
                        </div>
                    }
                }
            }
        }
    }
}
