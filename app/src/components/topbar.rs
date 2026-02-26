use dioxus::prelude::*;

/// `file_url_from_input` is only a best-effort fallback for non-browser or
/// legacy scenarios. Browsers typically hide real file paths for security, so
/// the preferred preview path is the `onchange` blob URL from
/// `URL.createObjectURL`.
fn file_url_from_input(value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() || raw.contains("fakepath") {
        return String::new();
    }
    if raw.starts_with("file://") {
        return raw.to_string();
    }
    if raw.contains(':') && raw.contains('\\') {
        // Windows path
        let normalized = raw.replace('\\', "/");
        return format!("file:///{normalized}");
    }
    format!("file://{raw}")
}

fn js_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[component]
pub fn TopBar(mut dark_mode: Signal<bool>, mut vim_mode: Signal<bool>) -> Element {
    let mut show_auth_modal = use_signal(|| false);
    let mut profile_name = use_signal(String::new);
    let mut profile_pic_path = use_signal(String::new);
    let mut profile_pic_preview = use_signal(String::new);
    let mut has_pic_upload = use_signal(|| false);
    let mut crop_applied = use_signal(|| false);
    let mut auth_status = use_signal(String::new);

    rsx! {
        header {
            class: "topbar",
            div {
                class: "topbar-inner",
                div {
                    class: "brand",
                    img {
                        class: "brand-logo",
                        src: asset!("/assets/logo.svg"),
                        alt: "Litedocs logo",
                    }
                    div {
                        class: "brand-text",
                        span { class: "brand-name", "Litedocs" }
                        span { class: "brand-tagline", "Local-first writing hub" }
                    }
                }
                div {
                    class: "top-search",
                    input {
                        r#type: "search",
                        placeholder: "Search docs...",
                    }
                }
                div {
                    class: "top-actions",
                    button {
                        class: if vim_mode() { "ghost icon-btn icon-active vim-toggle is-on" } else { "ghost icon-btn vim-toggle is-off" },
                        title: if vim_mode() { "Vim enabled" } else { "Vim disabled" },
                        onclick: move |_| vim_mode.set(!vim_mode()),
                        img {
                            class: "vim-mark",
                            src: asset!("/assets/icons/Neovim-mark.svg"),
                            alt: "Neovim",
                        }
                        span { class: "state-dot" }
                    }
                    button {
                        class: if dark_mode() { "ghost icon-btn icon-active" } else { "ghost icon-btn" },
                        title: if dark_mode() { "Switch to light mode" } else { "Switch to dark mode" },
                        onclick: move |_| dark_mode.set(!dark_mode()),
                        if dark_mode() {
                            svg {
                                view_box: "0 0 24 24",
                                width: "16",
                                height: "16",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.8",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                circle { cx: "12", cy: "12", r: "4" }
                                path { d: "M12 2v2" }
                                path { d: "M12 20v2" }
                                path { d: "M4.9 4.9l1.4 1.4" }
                                path { d: "M17.7 17.7l1.4 1.4" }
                                path { d: "M2 12h2" }
                                path { d: "M20 12h2" }
                                path { d: "M4.9 19.1l1.4-1.4" }
                                path { d: "M17.7 6.3l1.4-1.4" }
                            }
                        } else {
                            svg {
                                view_box: "0 0 24 24",
                                width: "16",
                                height: "16",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.8",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                path { d: "M21 12.8A9 9 0 1 1 11.2 3a7 7 0 1 0 9.8 9.8z" }
                            }
                        }
                    }
                    button {
                        class: "outline icon-btn",
                        title: "Sign in",
                        onclick: move |_| show_auth_modal.set(true),
                        svg {
                            view_box: "0 0 24 24",
                            width: "16",
                            height: "16",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.8",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M15 3h3a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-3" }
                            path { d: "M10 17l5-5-5-5" }
                            path { d: "M15 12H3" }
                        }
                    }
                }
            }
        }
        if show_auth_modal() {
            div {
                class: "modal-overlay",
                onclick: move |_| show_auth_modal.set(false),
                div {
                    class: "auth-modal",
                    onclick: move |evt| evt.stop_propagation(),
                    div { class: "auth-header", "Sign in (Placeholder)" }
                    p { class: "auth-sub", "WebAuthn wired to local server at http://127.0.0.1:8080." }
                    label { class: "auth-label", "Display name" }
                    input {
                        class: "auth-input",
                        r#type: "text",
                        placeholder: "Your name",
                        value: profile_name(),
                        oninput: move |evt| profile_name.set(evt.value()),
                    }
                    label { class: "auth-label", "Profile picture" }
                    input {
                        class: "auth-input file",
                        r#type: "file",
                        id: "auth-pic-input",
                        accept: "image/*",
                        onchange: move |evt| {
                            let value = evt.value();
                            profile_pic_path.set(value.clone());
                            crop_applied.set(false);
                            has_pic_upload.set(true);
                            let file_url = file_url_from_input(&value);
                            if !file_url.is_empty() {
                                profile_pic_preview.set(file_url);
                            } else {
                                profile_pic_preview.set(String::new());
                            }
                            let mut profile_pic_preview = profile_pic_preview;
                            spawn(async move {
                                if let Ok(blob_url) = document::eval(
                                    r#"(function() {
  const el = document.getElementById("auth-pic-input");
  const file = el && el.files && el.files[0];
  if (!file) return "";
  return URL.createObjectURL(file);
})()"#,
                                )
                                .join::<String>()
                                .await
                                {
                                    if !blob_url.is_empty() {
                                        profile_pic_preview.set(blob_url);
                                    }
                                }
                            });
                        },
                    }
                    if !profile_pic_path().is_empty() {
                        p { class: "auth-file-hint", "Selected: {profile_pic_path}" }
                    }
                    if has_pic_upload() {
                        div { class: "crop-wrap",
                            div { class: "crop-frame",
                                if !profile_pic_preview().is_empty() {
                                    div {
                                        class: "crop-canvas",
                                        img {
                                            class: "crop-image",
                                            src: "{profile_pic_preview}",
                                            alt: "Profile crop preview",
                                        }
                                        div {
                                            class: "crop-selection",
                                            id: "crop-selection",
                                            title: "Drag the resize handle in the corner",
                                        }
                                    }
                                } else {
                                    div { class: "crop-placeholder", "Image preview unavailable" }
                                }
                            }
                            div { class: "crop-controls",
                                p { class: "auth-file-hint", "Resize the square directly in preview." }
                                button {
                                    class: "ghost crop-apply",
                                    onclick: move |_| {
                                        document::eval(
                                            r#"(function() {
  const box = document.getElementById("crop-selection");
  if (!box) return;
  box.style.left = "12px";
  box.style.top = "12px";
  box.style.width = "86px";
  box.style.height = "86px";
})()"#,
                                        );
                                        crop_applied.set(false);
                                    },
                                    "Reset crop box"
                                }
                                button {
                                    class: "ghost crop-apply",
                                    onclick: move |_| crop_applied.set(true),
                                    "Apply crop"
                                }
                                if crop_applied() {
                                    p { class: "auth-file-hint", "Crop applied (preview)." }
                                }
                            }
                        }
                    }
                    div { class: "auth-actions",
                        button {
                            class: "ghost",
                            onclick: move |_| show_auth_modal.set(false),
                            "Cancel"
                        }
                        button {
                            class: "primary",
                            onclick: move |_| {
                                let name = profile_name();
                                if name.trim().is_empty() {
                                    auth_status.set("Name is required".to_string());
                                    return;
                                }
                                auth_status.set("Starting WebAuthn registration...".to_string());
                                let mut auth_status = auth_status;
                                let name_js = js_escape(&name);
                                spawn(async move {
                                    let script = format!(
                                        r#"(async function() {{
  const base = "http://127.0.0.1:8080";
  const b64urlToBuf = (s) => {{
    const pad = "=".repeat((4 - (s.length % 4)) % 4);
    const b64 = (s + pad).replace(/-/g, "+").replace(/_/g, "/");
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out.buffer;
  }};
  const bufToB64url = (buf) => {{
    const bytes = new Uint8Array(buf);
    let bin = "";
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
    return btoa(bin).replace(/\\+/g, "-").replace(/\\//g, "_").replace(/=+$/g, "");
  }};
  const regStart = await fetch(base + "/api/webauthn/register/start", {{
    method: "POST",
    headers: {{ "Content-Type": "application/json" }},
    body: JSON.stringify({{ name: "{name_js}" }})
  }});
  if (!regStart.ok) {{
    return "Registration start failed: " + regStart.status;
  }}
  const regStartJson = await regStart.json();
  const creation = regStartJson.challenge;
  creation.publicKey.challenge = b64urlToBuf(creation.publicKey.challenge);
  creation.publicKey.user.id = b64urlToBuf(creation.publicKey.user.id);
  if (Array.isArray(creation.publicKey.excludeCredentials)) {{
    creation.publicKey.excludeCredentials = creation.publicKey.excludeCredentials.map((c) => ({{ ...c, id: b64urlToBuf(c.id) }}));
  }}
  const att = await navigator.credentials.create({{ publicKey: creation.publicKey }});
  if (!att) {{
    return "Registration cancelled";
  }}
  const regPayload = {{
    id: att.id,
    rawId: bufToB64url(att.rawId),
    type: att.type,
    response: {{
      attestationObject: bufToB64url(att.response.attestationObject),
      clientDataJSON: bufToB64url(att.response.clientDataJSON),
      transports: att.response.getTransports ? att.response.getTransports() : []
    }},
    clientExtensionResults: att.getClientExtensionResults ? att.getClientExtensionResults() : {{}}
  }};
  const regFinish = await fetch(base + "/api/webauthn/register/finish", {{
    method: "POST",
    headers: {{ "Content-Type": "application/json" }},
    body: JSON.stringify({{ name: "{name_js}", credential: regPayload }})
  }});
  if (!regFinish.ok) {{
    return "Registration finish failed: " + regFinish.status;
  }}
  const authStart = await fetch(base + "/api/webauthn/auth/start", {{
    method: "POST",
    headers: {{ "Content-Type": "application/json" }},
    body: JSON.stringify({{ name: "{name_js}" }})
  }});
  if (!authStart.ok) {{
    return "Auth start failed: " + authStart.status;
  }}
  const authStartJson = await authStart.json();
  const request = authStartJson.challenge;
  request.publicKey.challenge = b64urlToBuf(request.publicKey.challenge);
  if (Array.isArray(request.publicKey.allowCredentials)) {{
    request.publicKey.allowCredentials = request.publicKey.allowCredentials.map((c) => ({{ ...c, id: b64urlToBuf(c.id) }}));
  }}
  const assertion = await navigator.credentials.get({{ publicKey: request.publicKey }});
  if (!assertion) {{
    return "Authentication cancelled";
  }}
  const authPayload = {{
    id: assertion.id,
    rawId: bufToB64url(assertion.rawId),
    type: assertion.type,
    response: {{
      authenticatorData: bufToB64url(assertion.response.authenticatorData),
      clientDataJSON: bufToB64url(assertion.response.clientDataJSON),
      signature: bufToB64url(assertion.response.signature),
      userHandle: assertion.response.userHandle ? bufToB64url(assertion.response.userHandle) : null
    }},
    clientExtensionResults: assertion.getClientExtensionResults ? assertion.getClientExtensionResults() : {{}}
  }};
  const authFinish = await fetch(base + "/api/webauthn/auth/finish", {{
    method: "POST",
    headers: {{ "Content-Type": "application/json" }},
    body: JSON.stringify({{ name: "{name_js}", credential: authPayload }})
  }});
  if (!authFinish.ok) {{
    return "Auth finish failed: " + authFinish.status;
  }}
  return "WebAuthn registration + sign-in complete";
}})()"#,
                                        name_js = name_js
                                    );
                                    match document::eval(&script).join::<String>().await {
                                        Ok(msg) => auth_status.set(msg),
                                        Err(_) => auth_status.set("Failed to call auth server".to_string()),
                                    }
                                });
                            },
                            "Continue"
                        }
                    }
                    if !auth_status().is_empty() {
                        p { class: "auth-file-hint", "{auth_status}" }
                    }
                }
            }
        }
    }
}
