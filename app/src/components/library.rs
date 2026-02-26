use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct DocItem {
    pub id: String,
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
    on_create: EventHandler<()>,
    on_delete: EventHandler<String>,
) -> Element {
    let limited_recent: Vec<DocItem> = recent.iter().take(6).cloned().collect();
    let total_recent = recent.len();
    rsx! {
        div { class: "library-view",
            div { class: "library-header",
                h2 { "Library" }
                div { class: "library-actions",
                    button {
                        class: "pill",
                        onclick: move |_| on_create.call(()),
                        "Create doc"
                    }
                    button { class: "pill secondary", "Import markdown" }
                }
            }
            div { class: "library-section",
                h4 { "Recent" }
                if recent.is_empty() {
                    div { class: "empty-state", "No local docs yet. Create one to get started." }
                } else {
                    div { class: "doc-scroll",
                        div { class: "doc-grid",
                            for item in limited_recent.iter() {
                                div {
                                    key: "{item.id}",
                                    class: "doc-card",
                                    button {
                                        class: "doc-open",
                                        onclick: {
                                            let on_open = on_open.clone();
                                            let id = item.id.clone();
                                            move |_| on_open.call(id.clone())
                                        },
                                        div { class: "doc-title", "{item.title}" }
                                        div { class: "doc-meta", "{item.meta} · {item.location}" }
                                    }
                                    button {
                                        class: "doc-delete",
                                        onclick: {
                                            let on_delete = on_delete.clone();
                                            let id = item.id.clone();
                                            move |_| on_delete.call(id.clone())
                                        },
                                        "Delete"
                                    }
                                }
                            }
                        }
                        if total_recent > limited_recent.len() {
                            div { class: "doc-more", "Showing {limited_recent.len()} of {total_recent} · scroll to see earlier" }
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
