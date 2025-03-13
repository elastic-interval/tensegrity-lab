use crate::fabric::FabricStats;
use leptos::either::Either;
use leptos::prelude::*;

#[component]
pub fn StatsView(fabric_stats: ReadSignal<Option<FabricStats>>) -> impl IntoView {
    view! {
        <div class="center-screen rounded">
            {move || {
                match fabric_stats.get() {
                    Some(stats) => {
                        let stats_string = format!("{:#?}", stats);
                        Either::Left(
                            view! {
                                <div>
                                    <p>{stats_string}</p>
                                </div>
                            },
                        )
                    }
                    None => Either::Right(view! { <div>Waiting to finish...</div> }),
                }
            }}
        </div>
    }
}
