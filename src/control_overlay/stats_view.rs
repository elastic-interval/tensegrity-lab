use crate::fabric::FabricStats;
use leptos::{component, view, IntoView, ReadSignal, SignalGet};

#[component]
pub fn StatsView(fabric_stats: ReadSignal<Option<FabricStats>>) -> impl IntoView {
    view! {
        <div class="bottom-right rounded">
            {move || {
                match fabric_stats.get() {
                    Some(stats) => {
                        let stats_string = format!("{:#?}", stats);
                        view! {
                            <div>
                                <p>{stats_string}</p>
                            </div>
                        }
                    }
                    None => {
                        view! { <div>Waiting for statistics</div> }
                    }
                }
            }}
        </div>
    }
}
