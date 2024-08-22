use leptos::{component, For, IntoView, ReadSignal, SignalGet, SignalSet, view, WriteSignal};
use crate::control_overlay::menu::{Menu, MenuContent, MenuItem};

#[component]
pub fn MenuView(
    menu: ReadSignal<Menu>,
    set_menu_choice: WriteSignal<MenuItem>,
) -> impl IntoView {
    let item_list = move || {
        match menu.get().root_item.content {
            MenuContent::Event(_) => { vec![] }
            MenuContent::Submenu(list) => { list }
        }
    };
    view! {
        <div class="list">
        <For
            each=item_list
            key=|item| item.label.clone()
            children=move |item| {
                let label = item.label.clone();
                let click = move |_event| {
                    set_menu_choice.set(item.clone());
                };
                view! {
                    <div class="item" on:click=click>
                        {move || label.clone()}
                    </div>
                }
            }
        />
        </div>
    }
}