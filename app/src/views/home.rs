use dioxus::prelude::*;

use crate::components::{DocItem, EditorView, LibraryView, StatusBar, TemplateItem, TopBar, VimMode};

#[derive(Clone, PartialEq)]
enum ActiveView {
    Library,
    Editor,
}

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Home() -> Element {
    let mut view = use_signal(|| ActiveView::Library);
    let mut doc_title = use_signal(|| "Design Spec — Litedocs".to_string());
    let mut dark_mode = use_signal(|| false);
    let mut vim_enabled = use_signal(|| false);
    let mut vim_mode = use_signal(|| VimMode::Normal);

    let recent_docs = vec![
        DocItem {
            title: "Product Narrative".to_string(),
            meta: "Edited 12 min ago".to_string(),
            location: "Local".to_string(),
        },
        DocItem {
            title: "Litedocs Launch Plan".to_string(),
            meta: "Edited 1 hour ago".to_string(),
            location: "Local".to_string(),
        },
        DocItem {
            title: "Research Synthesis".to_string(),
            meta: "Edited yesterday".to_string(),
            location: "Local".to_string(),
        },
        DocItem {
            title: "Interview Notes".to_string(),
            meta: "Edited Feb 19".to_string(),
            location: "Local".to_string(),
        },
    ];

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
                        recent: recent_docs,
                        templates,
                        on_open: move |title| {
                            doc_title.set(title);
                            view.set(ActiveView::Editor);
                        },
                    }
                } else {
                    EditorView {
                        doc_title,
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
