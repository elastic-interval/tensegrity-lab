use std::sync::mpsc::Sender;

use leptos::{
    component, create_signal, event_target_value, view, IntoView, ReadSignal, SignalGet, SignalSet,
};
use leptos_use::storage::use_local_storage;
use leptos_use::use_debounce_fn;
use leptos_use::utils::FromToStringCodec;

use crate::control_overlay::action::Action;
use crate::fabric::interval::Interval;

const SAVED_TENSCRIPT_KEY: &str = "SAVED_TENSCRIPT";

#[component]
pub fn ControlOverlayApp(
    control_state: ReadSignal<ControlState>,
    actions_tx: Sender<Action>,
) -> impl IntoView {
    let pre_text = move || format!("{:#?}", control_state.get());
    let load_fabric = move |name: Vec<String>| {
        actions_tx
            .send(Action::LoadFabric(name))
            .expect("failed to send action");
    };
    let (category, set_category) = create_signal("Art".to_string());
    let (subname, set_subname) = create_signal("Halo by Crane".to_string());
    let (tenscript, set_tenscript, _) =
        use_local_storage::<String, FromToStringCodec>(SAVED_TENSCRIPT_KEY);
    if tenscript.get() == "" {
        set_tenscript.set(include_str!("../../fabric_library.scm").to_string());
    }
    let save_tenscript = use_debounce_fn(
        move || {
            #[cfg(target_arch = "wasm32")]
            {
                use js_sys::eval;
                let new_tenscript = eval("window.AceEditor.getValue()")
                    .expect("could not get Ace editor content")
                    .as_string()
                    .expect("not a string");
                set_tenscript.set(new_tenscript);
            }
        },
        1000.,
    );
    view! {
        <div class="inset">
            <section class="left">
                <input
                type="text"
                value=category.get()
                on:change=move |ev| { set_category.set(event_target_value(&ev)); } />
                <input
                    type="text"
                    value=subname.get()
                    on:change=move |ev| { set_subname.set(event_target_value(&ev)); } />
                <button on:click=move |_ev| { load_fabric(vec![category.get(), subname.get()]) }>
                    Load Fabric
                </button>
                <pre>{pre_text}</pre>
            </section>
            <section class="right">
                <div id="tenscript_editor" on:change=move |_ev| { save_tenscript(); }>
                    {tenscript.get()}
                </div>
            </section>
            <script>
                let editor = ace.edit("tenscript_editor");
                editor.setTheme("ace/theme/github_dark");
                editor.session.setMode("ace/mode/scheme");
                window.AceEditor = editor;
            </script>
        </div>
    }
}

#[derive(Clone, Debug, Default)]
pub struct ControlState {
    pub(crate) picked_interval: Option<Interval>,
}
