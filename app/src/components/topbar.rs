use dioxus::prelude::*;

#[component]
pub fn TopBar(mut dark_mode: Signal<bool>, mut vim_mode: Signal<bool>) -> Element {
    rsx! {
        header {
            class: "topbar",
            div {
                class: "brand",
                div { class: "mark" }
                div {
                    class: "brand-text",
                    span { class: "brand-name", "Litedocs" }
                    span { class: "brand-sub", "Local-first writing space" }
                }
            }
            div {
                class: "top-search",
                input {
                    r#type: "search",
                    placeholder: "Search local library...",
                }
            }
            div {
                class: "top-actions",
                div { class: "sync-pill", "Offline · Local only" }
                button {
                    class: "ghost",
                    onclick: move |_| vim_mode.set(!vim_mode()),
                    if vim_mode() { "Vim on" } else { "Vim off" }
                }
                button {
                    class: "ghost",
                    onclick: move |_| dark_mode.set(!dark_mode()),
                    if dark_mode() { "Light mode" } else { "Dark mode" }
                }
                button { class: "outline", "Sign in" }
            }
        }
    }
}
