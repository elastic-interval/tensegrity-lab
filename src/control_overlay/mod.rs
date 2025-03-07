use crate::application::OverlayChange;
use crate::control_overlay::details_view::DetailsView;
use crate::control_overlay::fabric_choice_view::FabricChoiceView;
use crate::control_overlay::stats_view::StatsView;
use crate::fabric::FabricStats;
use crate::messages::{ControlState, LabEvent};
use leptos::prelude::*;
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
    pub fabric_name: ReadSignal<String>,
    pub set_fabric_name: WriteSignal<String>,
}

impl Default for OverlayState {
    fn default() -> Self {
        let (control_state, set_control_state) = signal(ControlState::default());
        let (fabric_stats, set_fabric_stats) = signal::<Option<FabricStats>>(None);
        let (fabric_name, set_fabric_name) = signal("Shimmy".to_string());
        Self {
            control_state,
            set_control_state,
            fabric_stats,
            set_fabric_stats,
            fabric_name,
            set_fabric_name,
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
                self.set_control_state.set(if fabric_stats.is_some() {
                    ControlState::Viewing
                } else {
                    ControlState::Waiting
                });
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
    fabric_name: ReadSignal<String>,
    set_fabric_name: WriteSignal<String>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
    // let (fabric_name, set_fabric_name, _) =
    //     use_local_storage::<String, FromToStringCodec>("fabric");
    Effect::new(move |_| {
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
