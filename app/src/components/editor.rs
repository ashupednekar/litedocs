use dioxus::prelude::*;
use keyboard_types::Key;
use pulldown_cmark::{html, Options, Parser};
use std::time::Duration;

use crate::components::VimMode;
use crate::internal::files::{FileStorage, LocalFileStorage};

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
    vim_enabled: Signal<bool>,
    vim_mode: Signal<VimMode>,
    on_back: EventHandler<()>,
) -> Element {
    let mut rich_mode = use_signal(|| true);
    let mut show_table_picker = use_signal(|| false);
    let mut table_rows = use_signal(|| 3);
    let mut table_cols = use_signal(|| 3);
    let mut font_family = use_signal(|| "Roboto".to_string());
    let mut font_color = use_signal(|| "#e5e7eb".to_string());
    let mut pending_dd = use_signal(|| false);
    let mut content = use_signal(|| DEFAULT_DOC_CONTENT.to_string());
    let mut autosave_status = use_signal(|| "Autosaving locally".to_string());
    let mut save_revision = use_signal(|| 0_u64);
    let storage = use_hook(LocalFileStorage::default);
    let mut insert_markdown_owned = move |value: String| {
        if rich_mode() {
            let mut text = content();
            text.push_str(&value);
            content.set(text);
            return;
        }
        insert_value_textarea(&value);
    };
    let mut last_html = use_signal(String::new);
    let exec_command = |cmd: &str, value: Option<&str>| {
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
    };
    let insert_rich_html = |html: &str| {
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

    // Load local draft when the active document title changes.
    use_effect(move || {
        let title = doc_title();
        let doc_id = LocalFileStorage::doc_id_from_title(&title);

        match storage_for_load.read(&doc_id) {
            Ok(bytes) if !bytes.is_empty() => {
                if let Ok(text) = String::from_utf8(bytes) {
                    content.set(text);
                    autosave_status.set("Loaded local draft".to_string());
                }
            }
            Ok(_) => {
                content.set(DEFAULT_DOC_CONTENT.to_string());
                autosave_status.set("Autosaving locally".to_string());
            }
            Err(err) => {
                autosave_status.set(format!("Load failed: {err}"));
            }
        }
    });

    // Debounced local-first autosave after user inactivity.
    use_effect(move || {
        let title = doc_title();
        let text = content();
        let doc_id = LocalFileStorage::doc_id_from_title(&title);
        *save_revision.write() += 1;
        let revision = save_revision();
        autosave_status.set("Autosaving locally...".to_string());

        let mut autosave_status = autosave_status;
        let save_revision = save_revision;
        let storage = storage_for_save.clone();

        spawn(async move {
            tokio::time::sleep(Duration::from_millis(900)).await;
            if save_revision() != revision {
                return;
            }

            match storage.write_full(&doc_id, text.as_bytes()) {
                Ok(()) => autosave_status.set("Saved locally".to_string()),
                Err(err) => autosave_status.set(format!("Save failed: {err}")),
            }
        });
    });

    let html_text = markdown_to_html(&content());
    use_effect(move || {
        if !rich_mode() {
            return;
        }
        let html_text = markdown_to_html(&content());
        if html_text != last_html() {
            last_html.set(html_text.clone());
            let html_js = js_escape(&html_text);
            document::eval(&format!(
                r#"(function() {{
  const el = document.getElementById("doc-surface");
  if (!el) return;
  el.innerHTML = "{html_js}";
}})();"#,
            ));
        }
    });
    rsx! {
        div { class: "breadcrumbs", "Library / {doc_title().to_uppercase()}" }
        div {
            class: "doc-header",
            div { class: "doc-actions",
                button {
                    class: "ghost compact-back",
                    onclick: move |_| on_back.call(()),
                    "← Back to library"
                }
            }
            input {
                class: "doc-title-input",
                r#type: "text",
                value: doc_title(),
                oninput: move |e| doc_title.set(e.value()),
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
                    onclick: move |_| rich_mode.set(false),
                    "Markdown"
                }
                button {
                    class: if rich_mode() { "active" } else { "" },
                    onclick: move |_| rich_mode.set(true),
                    "Doc"
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
                        dangerous_inner_html: "{html_text}",
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
                            let mut content = content;
                            let mut last_html = last_html;
                            spawn(async move {
                                if let Ok(html_value) = document::eval(
                                    r#"document.getElementById("doc-surface")?.innerHTML || """#,
                                )
                                .join::<String>()
                                .await
                                {
                                    if html_value != last_html() {
                                        last_html.set(html_value.clone());
                                        let markdown = html2md::parse_html(&html_value);
                                        content.set(markdown);
                                    }
                                }
                            });
                        }
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
                    oninput: move |evt| content.set(evt.value()),
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
