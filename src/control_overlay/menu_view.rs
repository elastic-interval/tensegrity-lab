use leptos::{component, IntoView, ReadSignal, SignalGet, SignalSet, view, WriteSignal};

use crate::control_overlay::menu::{Menu, MenuContent, MenuItem};

#[component]
pub fn MenuView(
    menu: ReadSignal<Menu>,
    menu_choice: ReadSignal<MenuItem>,
    set_menu_choice: WriteSignal<MenuItem>,
) -> impl IntoView {
    view! {
        <div>
        { move || {
            let item = menu_choice.get();
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
                            {
                                item_list
                                    .iter()
                                    .map(|sub_item|{
                                        view! {
                                            <div class="item">
                                                <MenuItemView menu_item={sub_item.clone()} set_menu_choice={set_menu_choice}/>
                                            </div>
                                        }
                                    })
                                .collect::<Vec<_>>()
                            }
                            </div>
                        </div>
                    }
                }
            }
        }}
        </div>
    }
}

#[component]
pub fn MenuItemView(
    menu_item:MenuItem,
    set_menu_choice: WriteSignal<MenuItem>,
) -> impl IntoView {
    let label = menu_item.label.clone();
    let click = move |_| {
        set_menu_choice.set(menu_item.clone());
    };
    view! {
        <div on:click=click>
            {label}
        </div>
    }
}
