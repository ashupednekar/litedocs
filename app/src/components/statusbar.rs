use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
}

#[component]
pub fn StatusBar(vim_enabled: Signal<bool>, vim_mode: Signal<VimMode>) -> Element {
    let mode_label = match vim_mode() {
        VimMode::Normal => "Normal",
        VimMode::Insert => "Insert",
        VimMode::Visual => "Visual",
    };
    rsx! {
        footer {
            class: "statusbar",
            div {
                class: "statusbar-inner",
                div { "Local-first mode" }
                div {
                    if vim_enabled() { "Vim: {mode_label}" } else { "Vim: off" }
                }
                div { "Sync: not connected" }
            }
        }
    }
}
