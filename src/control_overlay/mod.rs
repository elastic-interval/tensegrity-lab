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
    pub show_details: ReadSignal<bool>,
    pub set_show_details: WriteSignal<bool>,
    pub show_stats: ReadSignal<bool>,
    pub set_show_stats: WriteSignal<bool>,
    pub fabric_name: ReadSignal<String>,
    pub set_fabric_name: WriteSignal<String>,
}

impl Default for OverlayState {
    fn default() -> Self {
        let (control_state, set_control_state) = signal(ControlState::default());
        let (fabric_stats, set_fabric_stats) = signal::<Option<FabricStats>>(None);
        let (show_details, set_show_details) = signal(false);
        let (show_stats, set_show_stats) = signal(false);
        let (fabric_name, set_fabric_name) = signal("De Twips".to_string());
        Self {
            control_state,
            set_control_state,
            fabric_stats,
            set_fabric_stats,
            show_details,
            set_show_details,
            show_stats,
            set_show_stats,
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
            OverlayChange::ToggleShowDetails => {
                self.set_show_details.update(|show| *show = !*show);
            }
            OverlayChange::ToggleShowStats => {
                self.set_show_stats.update(|show| *show = !*show);
            }
        }
    }
}

#[component]
pub fn ControlOverlayApp(
    fabric_list: Vec<String>,
    control_state: ReadSignal<ControlState>,
    fabric_stats: ReadSignal<Option<FabricStats>>,
    show_details: ReadSignal<bool>,
    show_stats: ReadSignal<bool>,
    fabric_name: ReadSignal<String>,
    set_fabric_name: WriteSignal<String>,
    event_loop_proxy: EventLoopProxy<LabEvent>,
) -> impl IntoView {
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
            <Show
                when=move || { show_details.get() }
                fallback=|| view! { <div class="bottom-center rounded">"[D]etails [S]tats"</div> }
            >
                <DetailsView control_state=control_state fabric_stats=fabric_stats />
            </Show>
            <Show when=move || { show_stats.get() } fallback=|| view! { <div /> }>
                <StatsView fabric_stats=fabric_stats />
            </Show>
        </div>
    }
}
