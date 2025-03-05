use crate::application::OverlayChange;
use crate::control_overlay::details_view::DetailsView;
use crate::control_overlay::fabric_choice_view::FabricChoiceView;
use crate::control_overlay::stats_view::StatsView;
use crate::fabric::FabricStats;
use crate::messages::{ControlState, LabEvent};
use codee::string::FromToStringCodec;
use leptos::*;
use leptos_use::storage::use_local_storage;
use winit::event_loop::EventLoopProxy;

mod details_view;
mod fabric_choice_view;
mod stats_view;

#[derive(Clone)]
pub struct OverlayState {
    pub control_state: ReadSignal<ControlState>,
    pub set_control_state: WriteSignal<ControlState>,
    pub fabric_stats: ReadSignal<Option<FabricStats>>,
    pub set_fabric_stats: WriteSignal<Option<FabricStats>>,
}

impl Default for OverlayState {
    fn default() -> Self {
        let (control_state, set_control_state) = create_signal(ControlState::default());
        let (fabric_stats, set_fabric_stats) = create_signal::<Option<FabricStats>>(None);
        Self {
            control_state,
            set_control_state,
            fabric_stats,
            set_fabric_stats,
        }
    }
}

impl OverlayState {
    pub fn change_happened(&mut self, app_change: OverlayChange) {
        match app_change {
            OverlayChange::SetControlState(control_state) => {
                self.set_control_state.set(control_state)
            }
            OverlayChange::SetFabricStats(fabric_stats) => {
                self.set_fabric_stats.set(fabric_stats);
            }
        }
    }
}

#[component]
pub fn ControlOverlayApp(
    fabric_list: Vec<String>,
    control_state: ReadSignal<ControlState>,
    fabric_stats: ReadSignal<Option<FabricStats>>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
    let (fabric_name, set_fabric_name, _) =
        use_local_storage::<String, FromToStringCodec>("fabric");
    let (animated, set_animated) = create_signal(false);
    let event_loop_proxy_1 = event_loop_proxy.clone();
    create_effect(move |_| {
        event_loop_proxy
            .send_event(LabEvent::LoadFabric(fabric_name.get()))
            .unwrap()
    });

    view! {
        <div>
            <FabricChoiceView
                fabric_list=fabric_list
                fabric_name=fabric_name
                set_fabric_name=set_fabric_name
            />
            <DetailsView control_state=control_state fabric_stats=fabric_stats />
            <StatsView fabric_stats=fabric_stats />
        </div>
    }
}
