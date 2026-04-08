use dioxus::prelude::*;
use keyboard_types::Key;
use litedocs_document::{doc_id_from_title, FileStorage, LocalFileStorage};
use pulldown_cmark::{html, Options, Parser};
use std::time::Duration;

use crate::components::VimMode;

const DEFAULT_DOC_CONTENT: &str = "## Overview\n\nStart writing in **Markdown**. Preview updates live below.\n\n- Local-first notes\n- Vim motions (when enabled)\n- Clean export\n\n> Tip: Use headings and lists to structure ideas.\n";

fn js_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn build_table(rows: i32, cols: i32) -> String {
    let cols = cols.max(1) as usize;
    let rows = rows.max(2) as usize;
    let header = (0..cols).map(|_| "Column").collect::<Vec<_>>().join(" | ");
    let divider = (0..cols).map(|_| "---").collect::<Vec<_>>().join(" | ");
    let mut out = String::new();
    out.push_str("\n\n| ");
    out.push_str(&header);
    out.push_str(" |\n| ");
    out.push_str(&divider);
    out.push_str(" |\n");
    for _ in 0..(rows - 1) {
        let row = (0..cols).map(|_| "Value").collect::<Vec<_>>().join(" | ");
        out.push_str("| ");
        out.push_str(&row);
        out.push_str(" |\n");
    }
    out.push_str("\n");
    out
}

fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    html_out
}

/// Push rendered HTML into `#doc-surface` and return the same HTML string for `last_html` sync.
fn apply_markdown_to_doc_surface_dom(markdown: &str) -> String {
    let html = markdown_to_html(markdown);
    let html_js = js_escape(&html);
    // No IIFE: desktop eval wraps this in an async function; only `return` at top level sends a value back.
    let _ = document::eval(&format!(
        r#"
  const el = document.getElementById("doc-surface");
  if (!el) return;
  el.innerHTML = "{html_js}";
  try {{ document.execCommand("styleWithCSS", false, false); }} catch (e) {{}}
"#,
        html_js = html_js
    ));
    html
}

/// Read `#doc-surface` HTML for html2md. Browsers often emit `<span style="font-weight:...">` for bold;
/// html2md only maps `<b>` / `<strong>`, so we normalize spans before conversion.
///
/// Must use top-level `return` (no IIFE): Dioxus desktop wraps this in `async function (dioxus) { ... }`;
/// an IIFE's result is discarded, so `.join::<String>()` would get `undefined` and deserialization fails.
const READ_DOC_SURFACE_HTML_JS: &str = r#"
  const el = document.getElementById("doc-surface");
  if (!el) return "";
  try { document.execCommand("styleWithCSS", false, false); } catch (e) {}
  const spans = Array.from(el.querySelectorAll("span[style]")).reverse();
  for (const span of spans) {
    if (!span.parentNode) continue;
    const fw = (span.style.fontWeight || "").toLowerCase();
    const fs = (span.style.fontStyle || "").toLowerCase();
    const n = parseInt(fw, 10);
    const bold = fw === "bold" || fw === "700" || fw === "bolder" || (!isNaN(n) && n >= 600);
    const italic = fs === "italic" || fs === "oblique";
    if (bold && italic) {
      const strong = document.createElement("strong");
      const em = document.createElement("em");
      span.parentNode.replaceChild(strong, span);
      strong.appendChild(em);
      while (span.firstChild) em.appendChild(span.firstChild);
    } else if (bold) {
      const strong = document.createElement("strong");
      span.parentNode.replaceChild(strong, span);
      while (span.firstChild) strong.appendChild(span.firstChild);
    } else if (italic) {
      const em = document.createElement("em");
      span.parentNode.replaceChild(em, span);
      while (span.firstChild) em.appendChild(span.firstChild);
    }
  }
  return el.innerHTML;
"#;

fn insert_into_textarea(prefix: &str, suffix: &str, placeholder: &str) {
    let prefix = js_escape(prefix);
    let suffix = js_escape(suffix);
    let placeholder = js_escape(placeholder);
    document::eval(&format!(
        r#"(function() {{
  const el = document.getElementById("editor-textarea");
  if (!el) return;
  const start = el.selectionStart || 0;
  const end = el.selectionEnd || 0;
  const text = el.value || "";
  const selected = text.slice(start, end) || "{placeholder}";
  const before = text.slice(0, start);
  const after = text.slice(end);
  const next = before + "{prefix}" + selected + "{suffix}" + after;
  el.value = next;
  const cursor = before.length + "{prefix}".length + selected.length + "{suffix}".length;
  el.setSelectionRange(cursor, cursor);
  el.dispatchEvent(new Event("input", {{ bubbles: true }}));
}})();"#,
        prefix = prefix,
        suffix = suffix,
        placeholder = placeholder
    ));
}

fn insert_value_textarea(value: &str) {
    let value = js_escape(value);
    document::eval(&format!(
        r#"(function() {{
  const el = document.getElementById("editor-textarea");
  if (!el) return;
  const start = el.selectionStart || 0;
  const end = el.selectionEnd || 0;
  const text = el.value || "";
  const before = text.slice(0, start);
  const after = text.slice(end);
  const next = before + "{value}" + after;
  el.value = next;
  const cursor = before.length + "{value}".length;
  el.setSelectionRange(cursor, cursor);
  el.dispatchEvent(new Event("input", {{ bubbles: true }}));
}})();"#,
        value = value
    ));
}

#[component]
pub fn EditorView(
    mut doc_title: Signal<String>,
    mut active_doc_id: Signal<Option<String>>,
    vim_enabled: Signal<bool>,
    vim_mode: Signal<VimMode>,
    on_back: EventHandler<()>,
) -> Element {
    // Markdown is the reliable path (textarea ↔ `content`). Rich mode depends on JS eval to read the DOM.
    let mut rich_mode = use_signal(|| false);
    let mut show_table_picker = use_signal(|| false);
    let mut table_rows = use_signal(|| 3);
    let mut table_cols = use_signal(|| 3);
    let mut font_family = use_signal(|| "Roboto".to_string());
    let mut font_color = use_signal(|| "#e5e7eb".to_string());
    let mut pending_dd = use_signal(|| false);
    let mut content = use_signal(|| DEFAULT_DOC_CONTENT.to_string());
    let mut autosave_status = use_signal(|| "Saved locally".to_string());
    let mut save_revision = use_signal(|| 0_u64);
    // Bumps on every edit so debounced autosave runs even when rich-mode DOM lags the `content` signal.
    let mut editor_dirty = use_signal(|| 0_u64);
    let stable_doc_id = use_signal(move || {
        active_doc_id()
            .unwrap_or_else(|| doc_id_from_title(&doc_title()))
    });
    let storage = use_hook(LocalFileStorage::default);
    let mut insert_markdown_owned = move |value: String| {
        if rich_mode() {
            let mut text = content();
            text.push_str(&value);
            content.set(text);
            editor_dirty.with_mut(|n| *n += 1);
            return;
        }
        insert_value_textarea(&value);
    };
    let last_html = use_signal(String::new);
    let mut editor_dirty_for_rich = editor_dirty.clone();
    let mut exec_command = move |cmd: &str, value: Option<&str>| {
        let cmd = js_escape(cmd);
        let value = value.map(js_escape).unwrap_or_default();
        document::eval(&format!(
            r#"(function() {{
  const el = document.getElementById("doc-surface");
  if (!el) return;
  el.focus();
  document.execCommand("{cmd}", false, "{value}");
}})();"#,
            cmd = cmd,
            value = value
        ));
        editor_dirty_for_rich.with_mut(|n| *n += 1);
    };
    let mut editor_dirty_for_insert = editor_dirty.clone();
    let mut insert_rich_html = move |html: &str| {
        let html = js_escape(html);
        document::eval(&format!(
            r#"(function() {{
  const el = document.getElementById("doc-surface");
  if (!el) return;
  el.focus();
  document.execCommand("insertHTML", false, "{html}");
}})();"#,
            html = html
        ));
        editor_dirty_for_insert.with_mut(|n| *n += 1);
    };
    let vim_move = |direction: &str, granularity: &str, extend: bool| {
        let direction = js_escape(direction);
        let granularity = js_escape(granularity);
        let mode = if extend { "extend" } else { "move" };
        document::eval(&format!(
            r#"(function() {{
  const sel = window.getSelection();
  if (!sel || !sel.modify) return;
  sel.modify("{mode}", "{direction}", "{granularity}");
}})();"#,
            direction = direction,
            granularity = granularity,
            mode = mode
        ));
    };
    let vim_move_textarea_word = |direction: &str| {
        let direction = js_escape(direction);
        document::eval(&format!(
            r#"(function() {{
  const el = document.getElementById("editor-textarea");
  if (!el) return;
  const text = el.value || "";
  let pos = el.selectionEnd || 0;
  const isWord = (c) => /[A-Za-z0-9_]/.test(c);
  if ("{direction}" === "forward") {{
    while (pos < text.length && !isWord(text[pos])) pos++;
    while (pos < text.length && isWord(text[pos])) pos++;
  }} else {{
    pos = Math.max(0, pos - 1);
    while (pos > 0 && !isWord(text[pos])) pos--;
    while (pos > 0 && isWord(text[pos - 1])) pos--;
  }}
  el.setSelectionRange(pos, pos);
}})();"#,
            direction = direction
        ));
    };
    let vim_delete_textarea_line = || {
        document::eval(
            r#"(function() {
  const el = document.getElementById("editor-textarea");
  if (!el) return;
  const text = el.value || "";
  let pos = el.selectionStart || 0;
  let start = text.lastIndexOf("\n", pos - 1);
  let end = text.indexOf("\n", pos);
  if (start === -1) start = 0; else start = start + 1;
  if (end === -1) end = text.length; else end = end + 1;
  el.value = text.slice(0, start) + text.slice(end);
  el.setSelectionRange(start, start);
  el.dispatchEvent(new Event("input", { bubbles: true }));
})();"#,
        );
    };
    let vim_delete_doc_line = || {
        document::eval(
            r#"(function() {
  const el = document.getElementById("doc-surface");
  if (!el) return;
  el.focus();
  const sel = window.getSelection();
  if (!sel || !sel.modify) return;
  sel.modify("move", "backward", "line");
  sel.modify("extend", "forward", "line");
  document.execCommand("delete");
})();"#,
        );
    };

    let storage_for_load = storage.clone();
    let storage_for_save = storage.clone();
    let storage_for_back = storage.clone();
    let storage_for_manual = storage.clone();
    let storage_for_title_rename = storage.clone();
    let manual_save = {
        let storage = storage_for_manual.clone();
        let stable_doc_id = stable_doc_id;
        let autosave_status = autosave_status.clone();
        let mut content_signal = content.clone();
        let last_html_signal = last_html.clone();
        let rich_mode_signal = rich_mode.clone();

        move |_: MouseEvent| {
            let doc_id = stable_doc_id();
            let rich = rich_mode_signal();
            let mut snapshot = content_signal();
            let mut autosave_status = autosave_status.clone();
            let storage = storage.clone();
            let mut last_html = last_html_signal.clone();

            spawn(async move {
                if rich {
                    match document::eval(READ_DOC_SURFACE_HTML_JS)
                        .join::<String>()
                        .await
                    {
                        Ok(html_value) => {
                            let trimmed = html_value.trim();
                            let emptyish = trimmed.is_empty()
                                || trimmed.eq_ignore_ascii_case("<br>")
                                || trimmed.eq_ignore_ascii_case("<br/>")
                                || trimmed == "<div><br></div>"
                                || trimmed == "<br><br>";
                            if emptyish && !snapshot.trim().is_empty() {
                                // keep snapshot (markdown buffer)
                            } else {
                                last_html.set(html_value.clone());
                                snapshot = html2md::parse_html(&html_value);
                                content_signal.set(snapshot.clone());
                            }
                        }
                        Err(_) => {
                            // keep snapshot
                        }
                    }
                }

                match storage.write_full(&doc_id, snapshot.as_bytes()) {
                    Ok(()) => autosave_status.set("Saved locally".to_string()),
                    Err(err) => autosave_status.set(format!("Save failed: {err}")),
                }
            });
        }
    };

    // Load local draft when the document id changes (not when the title field is edited).
    let mut last_html_for_load = last_html.clone();
    use_effect(move || {
        let doc_id = stable_doc_id();

        match storage_for_load.read(&doc_id) {
            Ok(bytes) if !bytes.is_empty() => {
                if let Ok(text) = String::from_utf8(bytes) {
                    content.set(text.clone());
                    autosave_status.set("Loaded local draft".to_string());
                    if rich_mode() {
                        spawn(async move {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            let html = apply_markdown_to_doc_surface_dom(&text);
                            last_html_for_load.set(html);
                        });
                    }
                }
            }
            Ok(_) => {
                let default = DEFAULT_DOC_CONTENT.to_string();
                content.set(default.clone());
                autosave_status.set("New draft".to_string());
                if rich_mode() {
                    spawn(async move {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        let html = apply_markdown_to_doc_surface_dom(&default);
                        last_html_for_load.set(html);
                    });
                }
            }
            Err(err) => {
                autosave_status.set(format!("Load failed: {err}"));
            }
        }
    });

    // Debounced local-first autosave after user inactivity.
    use_effect(move || {
        let _ = doc_title();
        let _ = editor_dirty();
        let _ = content();
        let doc_id = stable_doc_id();
        let rich = rich_mode();
        let revision = save_revision.with_mut(|rev| {
            *rev += 1;
            *rev
        });

        let mut autosave_status = autosave_status;
        let save_revision = save_revision;
        let storage = storage_for_save.clone();
        let mut content = content;
        let mut last_html = last_html;

        spawn(async move {
            tokio::time::sleep(Duration::from_millis(900)).await;
            if save_revision() != revision {
                return;
            }

            // Read after debounce so toolbar/execCommand edits are reflected in `content` (oninput)
            // and we don't snapshot stale markdown from when the effect first ran.
            let mut payload = content();
            let mut used_rich_fallback = false;
            if rich {
                match document::eval(READ_DOC_SURFACE_HTML_JS)
                    .join::<String>()
                    .await
                {
                    Ok(html_value) => {
                        let trimmed = html_value.trim();
                        // Missing #doc-surface or empty WebView snapshot: keep last markdown buffer.
                        let emptyish = trimmed.is_empty()
                            || trimmed.eq_ignore_ascii_case("<br>")
                            || trimmed.eq_ignore_ascii_case("<br/>")
                            || trimmed == "<div><br></div>"
                            || trimmed == "<br><br>";
                        if emptyish && !payload.trim().is_empty() {
                            used_rich_fallback = true;
                        } else {
                            last_html.set(html_value.clone());
                            payload = html2md::parse_html(&html_value);
                            content.set(payload.clone());
                        }
                    }
                    Err(_) => {
                        used_rich_fallback = true;
                    }
                }
            }

            match storage.write_full(&doc_id, payload.as_bytes()) {
                Ok(()) => {
                    if used_rich_fallback && rich {
                        autosave_status
                            .set("Saved locally (markdown buffer; rich DOM was not readable)".to_string());
                    } else {
                        autosave_status.set("Saved locally".to_string());
                    }
                }
                Err(err) => autosave_status.set(format!("Save failed: {err}")),
            }
        });
    });

    rsx! {
        div { class: "breadcrumbs", "Library / {doc_title().to_uppercase()}" }
        div {
            class: "doc-header",
            div { class: "doc-actions",
                button {
                    class: "ghost compact-back",
                    onclick: move |_| {
                        let doc_id = stable_doc_id();
                        let current_text = content();
                        let mut autosave_status = autosave_status;
                        let storage = storage_for_back.clone();
                        let on_back = on_back.clone();

                        if rich_mode() {
                            let mut content = content;
                            let mut last_html = last_html;
                            spawn(async move {
                                let markdown = match document::eval(READ_DOC_SURFACE_HTML_JS)
                                    .join::<String>()
                                    .await
                                {
                                    Ok(html_value) => {
                                        last_html.set(html_value.clone());
                                        html2md::parse_html(&html_value)
                                    }
                                    Err(_) => current_text,
                                };
                                content.set(markdown.clone());
                                match storage.write_full(&doc_id, markdown.as_bytes()) {
                                    Ok(()) => autosave_status.set("Saved locally".to_string()),
                                    Err(err) => autosave_status.set(format!("Save failed: {err}")),
                                }
                                on_back.call(());
                            });
                        } else {
                            match storage.write_full(&doc_id, current_text.as_bytes()) {
                                Ok(()) => autosave_status.set("Saved locally".to_string()),
                                Err(err) => autosave_status.set(format!("Save failed: {err}")),
                            }
                            on_back.call(());
                        }
                    },
                    "← Back to library"
                }
            }
            input {
                class: "doc-title-input",
                r#type: "text",
                value: doc_title(),
                oninput: move |e| doc_title.set(e.value()),
                onblur: move |_| {
                    let new_title = doc_title();
                    let new_id = doc_id_from_title(&new_title);
                    let old_id = stable_doc_id();
                    if new_id == old_id {
                        return;
                    }
                    let storage = storage_for_title_rename.clone();
                    let mut stable_doc_id = stable_doc_id;
                    let mut active_doc_id = active_doc_id;
                    let mut autosave_status = autosave_status.clone();
                    let rich = rich_mode();
                    let current_text = content();
                    let mut content_sig = content.clone();
                    let mut last_html_sig = last_html.clone();

                    if rich {
                        spawn(async move {
                            let markdown = match document::eval(READ_DOC_SURFACE_HTML_JS)
                                .join::<String>()
                                .await
                            {
                                Ok(html_value) => {
                                    last_html_sig.set(html_value.clone());
                                    html2md::parse_html(&html_value)
                                }
                                Err(_) => current_text,
                            };
                            content_sig.set(markdown.clone());
                            if let Err(err) = storage.write_full(&old_id, markdown.as_bytes()) {
                                autosave_status.set(format!("Save failed: {err}"));
                                return;
                            }
                            match storage.rename_doc(&old_id, &new_id) {
                                Ok(()) => {
                                    stable_doc_id.set(new_id.clone());
                                    active_doc_id.set(Some(new_id));
                                    autosave_status.set("Renamed document".to_string());
                                }
                                Err(err) => autosave_status.set(format!("Rename failed: {err}")),
                            }
                        });
                    } else {
                        if let Err(err) = storage.write_full(&old_id, current_text.as_bytes()) {
                            autosave_status.set(format!("Save failed: {err}"));
                            return;
                        }
                        match storage.rename_doc(&old_id, &new_id) {
                            Ok(()) => {
                                stable_doc_id.set(new_id.clone());
                                active_doc_id.set(Some(new_id));
                                autosave_status.set("Renamed document".to_string());
                            }
                            Err(err) => autosave_status.set(format!("Rename failed: {err}")),
                        }
                    }
                },
            }
            div { class: "doc-meta-row",
                span { class: "tag", "Local" }
                span { class: "tag", "Draft" }
                span { class: "tag", "Last synced: —" }
            }
        }
        div { class: "toolbar",
            button { "A" }
            button {
                onclick: move |_| {
                    if rich_mode() {
                        exec_command("bold", None);
                    } else {
                        insert_into_textarea("**", "**", "bold text");
                    }
                },
                "B"
            }
            button {
                onclick: move |_| {
                    if rich_mode() {
                        exec_command("italic", None);
                    } else {
                        insert_into_textarea("*", "*", "italic text");
                    }
                },
                "I"
            }
            button {
                onclick: move |_| {
                    if rich_mode() {
                        exec_command("formatBlock", Some("h1"));
                    } else {
                        insert_into_textarea("# ", "", "Heading");
                    }
                },
                "H1"
            }
            button {
                onclick: move |_| {
                    if rich_mode() {
                        exec_command("formatBlock", Some("blockquote"));
                    } else {
                        insert_into_textarea("> ", "", "Quote");
                    }
                },
                "Quote"
            }
            button {
                onclick: move |_| {
                    if rich_mode() {
                        exec_command("insertUnorderedList", None);
                    } else {
                        insert_into_textarea("- ", "", "List item");
                    }
                },
                "List"
            }
            div { class: "toolbar-divider" }
            button {
                onclick: move |_| show_table_picker.set(!show_table_picker()),
                "Insert table"
            }
            button { "Comment" }
            button { "Share" }
            div { class: "toolbar-divider" }
            div { class: "doc-controls",
                select {
                    class: "doc-select",
                    value: font_family(),
                    oninput: move |evt| {
                        let value = evt.value();
                        font_family.set(value.clone());
                        if rich_mode() {
                            exec_command("fontName", Some(&value));
                        }
                    },
                    option { value: "Roboto", "Roboto" }
                    option { value: "Times New Roman", "Times New" }
                    option { value: "Helvetica", "Helvetica" }
                    option { value: "Comic Sans MS", "Comic Sans" }
                }
                input {
                    class: "doc-color",
                    r#type: "color",
                    value: "{font_color}",
                    oninput: move |evt| {
                        let value = evt.value();
                        font_color.set(value.clone());
                        if rich_mode() {
                            exec_command("foreColor", Some(&value));
                        }
                    }
                }
            }
            div { class: "toggle-group",
                button {
                    class: if !rich_mode() { "active" } else { "" },
                    onclick: move |_| {
                        if rich_mode() {
                            let mut content = content;
                            let mut last_html = last_html;
                            let mut rich_mode = rich_mode;
                            spawn(async move {
                                if let Ok(html_value) = document::eval(READ_DOC_SURFACE_HTML_JS)
                                    .join::<String>()
                                    .await
                                {
                                    last_html.set(html_value.clone());
                                    content.set(html2md::parse_html(&html_value));
                                }
                                rich_mode.set(false);
                            });
                        } else {
                            rich_mode.set(false);
                        }
                    },
                    "Markdown"
                }
                button {
                    class: if rich_mode() { "active" } else { "" },
                    onclick: move |_| {
                        let text = content();
                        let mut last_html_sig = last_html.clone();
                        rich_mode.set(true);
                        spawn(async move {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            let html = apply_markdown_to_doc_surface_dom(&text);
                            last_html_sig.set(html);
                        });
                    },
                    "Doc"
                }
                button {
                    class: "ghost icon-btn save-btn",
                    title: "Save now",
                    onclick: manual_save,
                    img {
                        src: asset!("/assets/icons/save.svg"),
                        alt: "Save icon"
                    }
                }
            }
        }
        if show_table_picker() {
            div { class: "table-picker",
                div { class: "table-field",
                    span { "Rows" }
                    input {
                        r#type: "number",
                        min: 2,
                        max: 12,
                        value: "{table_rows}",
                        oninput: move |evt| {
                            if let Ok(value) = evt.value().parse::<i32>() {
                                table_rows.set(value);
                            }
                        }
                    }
                }
                div { class: "table-field",
                    span { "Cols" }
                    input {
                        r#type: "number",
                        min: 1,
                        max: 8,
                        value: "{table_cols}",
                        oninput: move |evt| {
                            if let Ok(value) = evt.value().parse::<i32>() {
                                table_cols.set(value);
                            }
                        }
                    }
                }
                button {
                    class: "primary",
                    onclick: move |_| {
                        let table = build_table(table_rows(), table_cols());
                        if rich_mode() {
                            let rows = table_rows().max(2) as usize;
                            let cols = table_cols().max(1) as usize;
                            let mut html_table = String::from("<table><thead><tr>");
                            for _ in 0..cols {
                                html_table.push_str("<th>Column</th>");
                            }
                            html_table.push_str("</tr></thead><tbody>");
                            for _ in 0..(rows - 1) {
                                html_table.push_str("<tr>");
                                for _ in 0..cols {
                                    html_table.push_str("<td>Value</td>");
                                }
                                html_table.push_str("</tr>");
                            }
                            html_table.push_str("</tbody></table>");
                            insert_rich_html(&html_table);
                        } else {
                            insert_markdown_owned(table);
                        }
                        show_table_picker.set(false);
                    },
                    "Insert"
                }
            }
        }
        div { class: "page",
            div { class: "page-header",
                span { class: "page-label", "Draft" }
                span { class: "page-status", "{autosave_status}" }
            }
            if rich_mode() {
                div {
                    class: "doc-mode",
                    div {
                        class: "doc-surface",
                        id: "doc-surface",
                        contenteditable: "true",
                        onkeydown: move |evt| {
                            if !vim_enabled() {
                                return;
                            }

                            if let Key::Escape = evt.key() {
                                evt.prevent_default();
                                vim_mode.set(VimMode::Normal);
                                pending_dd.set(false);
                                return;
                            }

                            if vim_mode() == VimMode::Insert {
                                pending_dd.set(false);
                                return;
                            }

                            match evt.key() {
                                Key::Character(ref ch) if ch == "i" => {
                                    evt.prevent_default();
                                    vim_mode.set(VimMode::Insert);
                                    pending_dd.set(false);
                                    return;
                                }
                                Key::Character(ref ch) if ch == "v" => {
                                    evt.prevent_default();
                                    if vim_mode() == VimMode::Visual {
                                        vim_mode.set(VimMode::Normal);
                                    } else {
                                        vim_mode.set(VimMode::Visual);
                                    }
                                    pending_dd.set(false);
                                    return;
                                }
                                _ => {}
                            }

                            if let Key::Character(ref ch) = evt.key() {
                                let extend = vim_mode() == VimMode::Visual;
                                match ch.as_str() {
                                    "h" => {
                                        evt.prevent_default();
                                        vim_move("backward", "character", extend);
                                        pending_dd.set(false);
                                        return;
                                    }
                                    "l" => {
                                        evt.prevent_default();
                                        vim_move("forward", "character", extend);
                                        pending_dd.set(false);
                                        return;
                                    }
                                    "j" => {
                                        evt.prevent_default();
                                        vim_move("forward", "line", extend);
                                        pending_dd.set(false);
                                        return;
                                    }
                                    "k" => {
                                        evt.prevent_default();
                                        vim_move("backward", "line", extend);
                                        pending_dd.set(false);
                                        return;
                                    }
                                    "w" => {
                                        evt.prevent_default();
                                        vim_move("forward", "word", extend);
                                        pending_dd.set(false);
                                        return;
                                    }
                                    "b" => {
                                        evt.prevent_default();
                                        vim_move("backward", "word", extend);
                                        pending_dd.set(false);
                                        return;
                                    }
                                    "d" => {
                                        evt.prevent_default();
                                        if pending_dd() {
                                            vim_delete_doc_line();
                                            pending_dd.set(false);
                                        } else {
                                            pending_dd.set(true);
                                        }
                                        return;
                                    }
                                    _ => {
                                        pending_dd.set(false);
                                    }
                                }
                            }

                            if matches!(evt.key(), Key::Backspace | Key::Delete) {
                                pending_dd.set(false);
                                return;
                            }

                            if let Key::Tab = evt.key() {
                                evt.prevent_default();
                                insert_into_textarea("  ", "", "");
                                return;
                            }

                            match evt.key() {
                                Key::Character(_) | Key::Enter | Key::Backspace | Key::Delete => {
                                    evt.prevent_default();
                                }
                                _ => {}
                            }
                        },
                        oninput: move |_| {
                            editor_dirty.with_mut(|n| *n += 1);
                            let mut content = content;
                            let mut last_html = last_html;
                            spawn(async move {
                                if let Ok(html_value) = document::eval(READ_DOC_SURFACE_HTML_JS)
                                .join::<String>()
                                .await
                                {
                                    last_html.set(html_value.clone());
                                    let markdown = html2md::parse_html(&html_value);
                                    content.set(markdown);
                                }
                            });
                        },
                        onblur: move |_| {
                            let mut content = content;
                            let mut last_html = last_html;
                            spawn(async move {
                                if let Ok(html_value) = document::eval(READ_DOC_SURFACE_HTML_JS)
                                    .join::<String>()
                                    .await
                                {
                                    last_html.set(html_value.clone());
                                    content.set(html2md::parse_html(&html_value));
                                }
                            });
                        },
                    }
                }
            } else {
                textarea {
                    class: "editor-text",
                    id: "editor-textarea",
                    rows: 14,
                    value: "{content}",
                    onkeydown: move |evt| {
                        if !vim_enabled() {
                            return;
                        }

                        if let Key::Escape = evt.key() {
                            evt.prevent_default();
                            vim_mode.set(VimMode::Normal);
                            pending_dd.set(false);
                            return;
                        }

                        if vim_mode() == VimMode::Insert {
                            pending_dd.set(false);
                            return;
                        }

                        match evt.key() {
                            Key::Character(ref ch) if ch == "i" => {
                                evt.prevent_default();
                                vim_mode.set(VimMode::Insert);
                                pending_dd.set(false);
                                return;
                            }
                            Key::Character(ref ch) if ch == "v" => {
                                evt.prevent_default();
                                if vim_mode() == VimMode::Visual {
                                    vim_mode.set(VimMode::Normal);
                                } else {
                                    vim_mode.set(VimMode::Visual);
                                }
                                pending_dd.set(false);
                                return;
                            }
                            _ => {}
                        }

                        let direction = match evt.key() {
                            Key::Character(ref ch) if ch == "h" => Some("left"),
                            Key::Character(ref ch) if ch == "j" => Some("down"),
                            Key::Character(ref ch) if ch == "k" => Some("up"),
                            Key::Character(ref ch) if ch == "l" => Some("right"),
                            _ => None,
                        };

                        if let Some(direction) = direction {
                            evt.prevent_default();
                            let mode = if vim_mode() == VimMode::Visual {
                                "visual"
                            } else {
                                "normal"
                            };
                            document::eval(&format!(
                                r#"(function() {{
  const el = document.getElementById("editor-textarea");
  if (!el) return;
  el.focus();
  const text = el.value || "";
  const pos = el.selectionEnd || 0;
  const anchor = el.selectionStart || 0;
  const lines = text.split("\n");
  let starts = [];
  let idx = 0;
  for (let i = 0; i < lines.length; i++) {{
    starts.push(idx);
    idx += lines[i].length + 1;
  }}
  let line = 0;
  for (let i = 0; i < starts.length; i++) {{
    if (pos >= starts[i]) line = i;
  }}
  let col = pos - starts[line];
  if ("{direction}" === "left") {{
    col = Math.max(0, col - 1);
  }} else if ("{direction}" === "right") {{
    col = Math.min(lines[line].length, col + 1);
  }} else if ("{direction}" === "up") {{
    line = Math.max(0, line - 1);
    col = Math.min(lines[line].length, col);
  }} else if ("{direction}" === "down") {{
    line = Math.min(lines.length - 1, line + 1);
    col = Math.min(lines[line].length, col);
  }}
  const newPos = starts[line] + col;
  if ("{mode}" === "visual") {{
    const base = Math.min(anchor, pos);
    const end = Math.max(anchor, pos);
    const keep = (newPos <= base) ? end : base;
    el.setSelectionRange(keep, newPos);
  }} else {{
    el.setSelectionRange(newPos, newPos);
  }}
}})();"#,
                                direction = direction,
                                mode = mode
                            ));
                            return;
                        }

                        if let Key::Character(ref ch) = evt.key() {
                            match ch.as_str() {
                                "w" => {
                                    evt.prevent_default();
                                    vim_move_textarea_word("forward");
                                    pending_dd.set(false);
                                    return;
                                }
                                "b" => {
                                    evt.prevent_default();
                                    vim_move_textarea_word("backward");
                                    pending_dd.set(false);
                                    return;
                                }
                                "d" => {
                                    evt.prevent_default();
                                    if pending_dd() {
                                        vim_delete_textarea_line();
                                        pending_dd.set(false);
                                    } else {
                                        pending_dd.set(true);
                                    }
                                    return;
                                }
                                _ => {
                                    pending_dd.set(false);
                                }
                            }
                        }

                        if matches!(evt.key(), Key::Backspace | Key::Delete) {
                            pending_dd.set(false);
                            return;
                        }

                        if let Key::Tab = evt.key() {
                            evt.prevent_default();
                            insert_into_textarea("  ", "", "");
                            return;
                        }

                        match evt.key() {
                            Key::Character(_) | Key::Enter | Key::Backspace | Key::Delete => {
                                evt.prevent_default();
                            }
                            _ => {}
                        }
                    },
                    oninput: move |evt| {
                        content.set(evt.value());
                        editor_dirty.with_mut(|n| *n += 1);
                    },
                    placeholder: "Start writing. Everything is saved locally as you type.\n\nIdeas:\n- Outline your goals\n- Add constraints\n- Draft the first paragraph",
                }
            }
            div { class: "page-footer",
                span { "Words: 312" }
                span { "Reading time: 2 min" }
                span { "Last local save: just now" }
            }
        }
    }
}
