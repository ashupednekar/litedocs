use dioxus::prelude::*;
use std::time::{Duration, SystemTime};

use crate::components::{DocItem, EditorView, LibraryView, StatusBar, TemplateItem, TopBar, VimMode};
use crate::util::doc_id_from_title;

#[derive(Clone, PartialEq)]
enum ActiveView {
    Library,
    Editor,
}

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Home() -> Element {
    let mut view = use_signal(|| ActiveView::Library);
    let mut doc_title = use_signal(|| "Untitled".to_string());
    let mut active_doc_id = use_signal(|| Option::<String>::None);
    let dark_mode = use_signal(|| false);
    let vim_enabled = use_signal(|| false);
    let vim_mode = use_signal(|| VimMode::Normal);
    let mut recent_docs = use_signal(Vec::<DocItem>::new);

    let templates = vec![
        TemplateItem {
            title: "Blank doc".to_string(),
            description: "Start with a clean page".to_string(),
        },
        TemplateItem {
            title: "Design brief".to_string(),
            description: "Problem, audience, constraints".to_string(),
        },
        TemplateItem {
            title: "Meeting notes".to_string(),
            description: "Agenda + action items".to_string(),
        },
        TemplateItem {
            title: "PRD".to_string(),
            description: "Goals, scope, milestones".to_string(),
        },
    ];

    rsx! {
        div {
            class: if dark_mode() { "app-shell dark" } else { "app-shell" },

            TopBar { dark_mode, vim_mode: vim_enabled }

            main {
                class: "editor-area",
                if view() == ActiveView::Library {
                    LibraryView {
                        recent: recent_docs(),
                        templates,
                        on_open: move |doc_id: String| {
                            let selected_title = recent_docs()
                                .iter()
                                .find(|item| item.id == doc_id)
                                .map(|item| item.title.clone())
                                .unwrap_or_else(|| "Untitled".to_string());
                            active_doc_id.set(Some(doc_id));
                            doc_title.set(selected_title);
                            view.set(ActiveView::Editor);
                        },
                        on_create: move |_| {
                            let ts = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or(Duration::from_secs(0))
                                .as_secs();
                            let title = format!("Untitled {ts}");
                            let id = doc_id_from_title(&title);
                            recent_docs.with_mut(|docs| {
                                docs.retain(|d| d.id != id);
                                docs.insert(
                                    0,
                                    DocItem {
                                        id: id.clone(),
                                        title: title.clone(),
                                        meta: "Just now".to_string(),
                                        location: "This session".to_string(),
                                    },
                                );
                            });
                            active_doc_id.set(Some(id));
                            doc_title.set(title);
                            view.set(ActiveView::Editor);
                        },
                        on_delete: move |doc_id: String| {
                            recent_docs.with_mut(|docs| docs.retain(|d| d.id != doc_id));
                        },
                    }
                } else {
                    EditorView {
                        doc_title,
                        active_doc_id,
                        vim_enabled,
                        vim_mode,
                        on_back: move |_| view.set(ActiveView::Library),
                    }
                }
            }

            StatusBar { vim_enabled, vim_mode }
        }

    }
}
