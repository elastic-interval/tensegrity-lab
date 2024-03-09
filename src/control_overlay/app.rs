use std::sync::mpsc::Sender;

use crate::build::tenscript::fabric_library::FabricLibrary;
use leptos::{
    component, create_effect, create_signal, event_target_value, view, IntoView, ReadSignal,
    SignalGet, SignalSet,
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
    let (category, set_category) = create_signal("Art".to_string());
    let (subname, set_subname) = create_signal("Halo by Crane".to_string());
    let (tenscript, set_tenscript, _) =
        use_local_storage::<String, FromToStringCodec>(SAVED_TENSCRIPT_KEY);
    let fabric_name = move || vec![category.get(), subname.get()];
    let on_run_fabric_click = move |_ev| {
        actions_tx
            .send(Action::LoadFabric(fabric_name()))
            .expect("failed to send action");
    };
    let on_load_fabric_into_editor_click = move |_ev| {
        let Ok(Some(fabric)) = FabricLibrary::load_specific_fabric_source(fabric_name()) else {
            return;
        };
        set_tenscript.set(fabric.clone());
        #[cfg(target_arch = "wasm32")]
        {
            let fabric_safe = fabric.replace('"', "\\\"");
            let js = format!("window.AceEditor.setValue(`{fabric_safe}`, 1)");
            js_sys::eval(&js).expect("JS failed to run");
        }
    };

    let save_tenscript = use_debounce_fn(
        move || {
            #[cfg(target_arch = "wasm32")]
            {
                let new_tenscript = js_sys::eval("window.AceEditor.getValue()")
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
                value=move || category.get()
                on:change=move |ev| { set_category.set(event_target_value(&ev)); } />
                <input
                    type="text"
                    value=move || subname.get()
                    on:change=move |ev| { set_subname.set(event_target_value(&ev)); } />
                <button on:click=on_run_fabric_click>
                    Run Fabric
                </button>
                <button on:click=on_load_fabric_into_editor_click>
                    Load Fabric Into Editor
                </button>
                <pre>{pre_text}</pre>
            </section>
            <section class="right">
                <div id="tenscript_editor" on:change=move |_ev| { save_tenscript(); }>
                    {move || tenscript.get()}
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
