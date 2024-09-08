use leptos::{component, IntoView, Signal, SignalGet, SignalSet, view, WriteSignal};

#[component]
pub fn FabricChoiceView(
    fabric_list: Vec<String>,
    fabric_name: Signal<String>,
    set_fabric_name: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="top-left rounded">
            {move || {
                if fabric_name.get().is_empty() {
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
                    }
                } else {
                    view! {
                        <div class="list" on:click=move|_|set_fabric_name.set(String::new())>
                            <b>"\""{fabric_name}"\""</b>
                        </div>
                    }
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
