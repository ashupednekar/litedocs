use dioxus::prelude::*;
use litedocs_document::{FileStorage, LocalFileStorage};
use std::time::{Duration, SystemTime};

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
    let mut doc_title = use_signal(|| "Untitled".to_string());
    let mut active_doc_id = use_signal(|| Option::<String>::None);
    let dark_mode = use_signal(|| false);
    let vim_enabled = use_signal(|| false);
    let vim_mode = use_signal(|| VimMode::Normal);
    let mut recent_docs = use_signal(Vec::<DocItem>::new);
    let storage = use_hook(LocalFileStorage::default);
    let storage_for_load = storage.clone();

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

    use_effect(move || {
        let _ = view();
        recent_docs.set(load_recent_docs(&storage_for_load));
    });
    let storage_for_delete = storage.clone();

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
                            active_doc_id.set(None);
                            doc_title.set(format!("Untitled {ts}"));
                            view.set(ActiveView::Editor);
                        },
                        on_delete: move |doc_id: String| {
                            let _ = storage_for_delete.delete(&doc_id);
                            recent_docs.set(load_recent_docs(&storage_for_delete));
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

fn load_recent_docs(storage: &LocalFileStorage) -> Vec<DocItem> {
    storage
        .list_docs()
        .unwrap_or_default()
        .into_iter()
        .map(|doc| DocItem {
            id: doc.id.to_string(),
            title: doc.title.to_string(),
            meta: format_updated(doc.updated_at),
            location: "Local".to_string(),
        })
        .collect()
}

fn format_updated(updated_at: SystemTime) -> String {
    let now = SystemTime::now();
    match now.duration_since(updated_at) {
        Ok(age) if age.as_secs() < 60 => "Edited just now".to_string(),
        Ok(age) if age.as_secs() < 3600 => format!("Edited {} min ago", age.as_secs() / 60),
        Ok(age) if age.as_secs() < 86_400 => format!("Edited {} hours ago", age.as_secs() / 3600),
        Ok(age) => format!("Edited {} days ago", age.as_secs() / 86_400),
        Err(_) => "Edited recently".to_string(),
    }
}
