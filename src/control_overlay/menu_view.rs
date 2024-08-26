use leptos::{component, IntoView, ReadSignal, SignalGet, SignalUpdate, view, WriteSignal};

use crate::control_overlay::menu::{MenuContent, MenuItem};

#[component]
pub fn MenuView(
    menu_choice: ReadSignal<Vec<MenuItem>>,
    set_menu_choice: WriteSignal<Vec<MenuItem>>,
) -> impl IntoView {
    let menu_up = move |_| {
        set_menu_choice
            .update(|stack| {
                (*stack).pop();
            })
    };
    view! {
        <div class="menu">
            {move || {
                let item = menu_choice.get().last().unwrap().clone();
                let label = item.label.clone();
                match item.content {
                    MenuContent::Empty | MenuContent::Event(_) => {
                        view! {
                            <div>
                                <h1>{label}</h1>
                            </div>
                        }
                    }
                    MenuContent::Submenu(item_list) => {
                        view! {
                            <div>
                                <h1>{label.clone()}</h1>
                                <div class="list">
                                    {item_list
                                        .iter()
                                        .map(|sub_item| {
                                            view! {
                                                <div class="item">
                                                    <MenuItemView
                                                        menu_item=sub_item.clone()
                                                        set_menu_choice=set_menu_choice
                                                    />
                                                </div>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    }
                }
            }}
            <div
                on:click=menu_up
                style:visibility=move || {
                    if menu_choice.get().len() == 1 { "hidden" } else { "visible" }
                }
            >
                "⬆️"
            </div>

        </div>
    }
}

#[component]
pub fn MenuItemView(
    menu_item: MenuItem,
    set_menu_choice: WriteSignal<Vec<MenuItem>>,
) -> impl IntoView {
    let label = menu_item.label.clone();
    let click = move |_| {
        set_menu_choice.update(|stack| { (*stack).push(menu_item.clone()) });
    };
    view! { <div on:click=click>{label}</div> }
}
