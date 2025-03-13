use leptos::either::Either;
use leptos::prelude::*;

#[component]
pub fn FabricChoiceView(
    fabric_list: Vec<String>,
    fabric_name: ReadSignal<String>,
    set_fabric_name: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="top-center rounded">
            {move || {
                if fabric_name.get().is_empty() {
                    Either::Left(
                        view! {
                            <div class="list">
                                {fabric_list
                                    .iter()
                                    .map(|fabric_name| {
                                        view! {
                                            <div class="item">
                                                <MenuItemView
                                                    fabric_name=fabric_name.clone()
                                                    set_fabric_name=set_fabric_name
                                                />
                                            </div>
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        },
                    )
                } else {
                    Either::Right(
                        view! {
                            <div class="list" on:click=move |_| set_fabric_name.set(String::new())>
                                <b>"\""{fabric_name}"\""</b>
                            </div>
                        },
                    )
                }
            }}
        </div>
    }
}

#[component]
pub fn MenuItemView(
    fabric_name: String,
    set_fabric_name: WriteSignal<String>,
) -> impl IntoView {
    let label = fabric_name.clone();
    let click = move |_| set_fabric_name.set(fabric_name.clone());
    view! { <div on:click=click>{label}</div> }
}
