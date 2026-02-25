use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct DocItem {
    pub title: String,
    pub meta: String,
    pub location: String,
}

#[derive(Clone, PartialEq)]
pub struct TemplateItem {
    pub title: String,
    pub description: String,
}

#[component]
pub fn LibraryView(
    recent: Vec<DocItem>,
    templates: Vec<TemplateItem>,
    on_open: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "library-view",
            div { class: "library-header",
                h2 { "Library" }
                div { class: "library-actions",
                    button { class: "pill", "Create doc" }
                    button { class: "pill secondary", "Import markdown" }
                }
            }
            div { class: "library-section",
                h4 { "Recent" }
                div { class: "doc-grid",
                    for item in recent.iter() {
                        button {
                            class: "doc-card",
                            onclick: {
                                let on_open = on_open.clone();
                                let title = item.title.clone();
                                move |_| on_open.call(title.clone())
                            },
                            div { class: "doc-title", "{item.title}" }
                            div { class: "doc-meta", "{item.meta} · {item.location}" }
                        }
                    }
                }
            }
            div { class: "library-section",
                h4 { "Storage" }
                div { class: "storage",
                    div { class: "storage-bar" }
                    span { "2.4 GB of 10 GB used" }
                }
            }
            div { class: "template-strip",
                h4 { "Templates" }
                div { class: "template-grid",
                    for item in templates.iter() {
                        div { class: "template-card",
                            h5 { "{item.title}" }
                            p { "{item.description}" }
                            button { class: "ghost", "Use template" }
                        }
                    }
                }
            }
        }
    }
}
