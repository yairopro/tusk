use leptos::*;
use leptos_router::*;
use wasm_bindgen::JsCast;

use crate::models::{Card, CardType, SaveTextCardRequest};

fn render_markdown(md: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(md, options);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

#[component]
pub fn EditorPage() -> impl IntoView {
    let params = use_params_map();
    let card_id = move || params.with(|p| p.get("id").cloned());

    let (title, set_title) = create_signal(String::new());
    let (card_mode, set_card_mode) = create_signal("text"); // "text" or "media"
    let (text_content, set_text_content) = create_signal(String::new());

    // Selected media file & preview
    let (selected_file, set_selected_file) = create_signal(Option::<web_sys::File>::None);
    let (preview_url, set_preview_url) = create_signal(Option::<String>::None);
    let (media_type, set_media_type) = create_signal(Option::<CardType>::None);
    let (existing_media_url, set_existing_media_url) = create_signal(Option::<String>::None);

    let (is_saving, set_is_saving) = create_signal(false);
    let (save_message, set_save_message) = create_signal(Option::<String>::None);
    let (active_tab, set_active_tab) = create_signal("edit"); // "edit" or "preview"

    // Load existing card if editing
    let existing_card = create_local_resource(
        move || card_id(),
        |id_opt| async move {
            let id = id_opt?;
            let url = format!("/api/cards/{}", id);
            let resp = gloo_net::http::Request::get(&url).send().await.ok()?;
            if resp.ok() {
                resp.json::<Card>().await.ok()
            } else {
                None
            }
        },
    );

    create_effect(move |_| {
        if let Some(Some(card)) = existing_card.get() {
            set_title.set(card.title);
            match card.card_type {
                CardType::Text => {
                    set_card_mode.set("text");
                    set_text_content.set(card.content);
                }
                ctype => {
                    set_card_mode.set("media");
                    set_media_type.set(Some(ctype));
                    set_existing_media_url.set(Some(card.content));
                }
            }
        }
    });

    let on_file_change = move |ev: leptos::ev::Event| {
        let target = ev.target();
        if let Some(target) = target {
            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                if let Some(files) = input.files() {
                    if let Some(file) = files.get(0) {
                        let fname = file.name();
                        let ext = fname.split('.').last().unwrap_or("").to_lowercase();
                        let ctype = CardType::from_extension(&ext);
                        set_media_type.set(ctype);

                        // Auto-fill title if empty
                        if title.get().trim().is_empty() {
                            let name_part = fname.rsplit_once('.').map(|(n, _)| n).unwrap_or(&fname);
                            set_title.set(name_part.to_string());
                        }

                        // Create local object URL for instant live preview
                        if let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) {
                            set_preview_url.set(Some(url));
                        }
                        set_selected_file.set(Some(file));
                    }
                }
            }
        }
    };

    let on_delete = move |_| {
        if let Some(id) = card_id() {
            spawn_local(async move {
                let url = format!("/api/cards/{}", id);
                let _ = gloo_net::http::Request::delete(&url).send().await;
                if let Some(window) = web_sys::window() {
                    let _ = window.location().set_href("/");
                }
            });
        }
    };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_is_saving.set(true);
        set_save_message.set(None);

        let t = title.get().trim().to_string();
        let mode = card_mode.get();
        let cid = card_id();

        if mode == "text" {
            let req = SaveTextCardRequest {
                folder_id: cid,
                title: t,
                content: text_content.get(),
            };

            spawn_local(async move {
                let resp = gloo_net::http::Request::post("/api/cards/save")
                    .header("Content-Type", "application/json")
                    .json(&req);

                set_is_saving.set(false);
                match resp {
                    Ok(r) => match r.send().await {
                        Ok(resp) if resp.ok() => {
                            set_save_message.set(Some("Saved successfully to Google Drive!".to_string()));
                        }
                        _ => {
                            set_save_message.set(Some("Failed to save card. Check folder selection.".to_string()));
                        }
                    },
                    Err(_) => {
                        set_save_message.set(Some("Error creating request.".to_string()));
                    }
                }
            });
        } else {
            // Media card submission via FormData
            let file_opt = selected_file.get();
            if file_opt.is_none() && cid.is_none() {
                set_is_saving.set(false);
                set_save_message.set(Some("Please select an audio, video, or image file.".to_string()));
                return;
            }

            let form_data = match web_sys::FormData::new() {
                Ok(fd) => fd,
                Err(_) => {
                    set_is_saving.set(false);
                    set_save_message.set(Some("Could not initialize file upload.".to_string()));
                    return;
                }
            };

            let _ = form_data.append_with_str("title", &t);
            if let Some(id) = cid {
                let _ = form_data.append_with_str("folder_id", &id);
            }
            if let Some(file) = file_opt {
                let _ = form_data.append_with_blob_and_filename("file", &file, &file.name());
            }

            spawn_local(async move {
                let resp = gloo_net::http::Request::post("/api/cards/upload")
                    .body(form_data);

                set_is_saving.set(false);
                match resp {
                    Ok(r) => match r.send().await {
                        Ok(res) if res.ok() => {
                            set_save_message.set(Some("Saved media card successfully to Google Drive!".to_string()));
                        }
                        _ => {
                            set_save_message.set(Some("Failed to upload media. Check folder selection.".to_string()));
                        }
                    },
                    Err(_) => {
                        set_save_message.set(Some("Error initiating file upload.".to_string()));
                    }
                }
            });
        }
    };

    view! {
        <div class="max-w-3xl mx-auto space-y-6">
            <div class="flex items-center justify-between pb-4 border-b border-slate-800">
                <div>
                    <h1 class="text-2xl font-bold text-white tracking-tight">
                        {move || if card_id().is_some() { "Edit Flashcard" } else { "Create Flashcard" }}
                    </h1>
                    <p class="text-xs text-slate-400 mt-1">
                        "Single-sided flashcard stored in Google Drive as "
                        <code class="font-mono text-teal-400">"<card-name>/index.<ext>"</code>
                    </p>
                </div>
                <div class="flex items-center gap-2">
                    {move || if card_mode.get() == "text" {
                        view! {
                            <div class="flex items-center gap-1 bg-slate-900 p-1 rounded-lg border border-slate-800">
                                <button
                                    type="button"
                                    on:click=move |_| set_active_tab.set("edit")
                                    class=move || {
                                        if active_tab.get() == "edit" {
                                            "px-3 py-1.5 text-xs font-semibold rounded-md bg-teal-400 text-slate-950 transition"
                                        } else {
                                            "px-3 py-1.5 text-xs font-semibold rounded-md text-slate-300 hover:bg-slate-800 transition"
                                        }
                                    }
                                >
                                    "Edit Note"
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| set_active_tab.set("preview")
                                    class=move || {
                                        if active_tab.get() == "preview" {
                                            "px-3 py-1.5 text-xs font-semibold rounded-md bg-teal-400 text-slate-950 transition"
                                        } else {
                                            "px-3 py-1.5 text-xs font-semibold rounded-md text-slate-300 hover:bg-slate-800 transition"
                                        }
                                    }
                                >
                                    "Preview"
                                </button>
                            </div>
                        }.into_view()
                    } else {
                        view! { <div /> }.into_view()
                    }}
                </div>
            </div>

            {move || save_message.get().map(|msg| {
                let is_ok = msg.contains("success");
                let color_class = if is_ok {
                    "bg-emerald-950/60 border-emerald-800/80 text-emerald-300"
                } else {
                    "bg-rose-950/60 border-rose-800/80 text-rose-300"
                };
                view! {
                    <div class=format!("p-3 rounded-lg border text-xs {}", color_class)>
                        {msg}
                    </div>
                }
            })}

            <form on:submit=on_submit class="space-y-6">
                // Card Title (Folder Name)
                <div>
                    <label class="block text-xs font-semibold text-slate-300 mb-1">
                        "Card Title (Folder Name in Drive)"
                    </label>
                    <input
                        type="text"
                        required
                        placeholder="e.g. guitar-c-major-scale"
                        prop:value=move || title.get()
                        on:input=move |ev| set_title.set(event_target_value(&ev))
                        class="w-full px-3.5 py-2 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition font-mono"
                    />
                </div>

                // Card Type Choice: Text Note vs Media File
                <div>
                    <label class="block text-xs font-semibold text-slate-300 mb-2">"Card Content Type"</label>
                    <div class="grid grid-cols-2 gap-3">
                        <button
                            type="button"
                            on:click=move |_| set_card_mode.set("text")
                            class=move || {
                                if card_mode.get() == "text" {
                                    "p-3 rounded-xl border border-teal-500 bg-teal-950/20 text-left transition flex items-center gap-3"
                                } else {
                                    "p-3 rounded-xl border border-slate-800 bg-slate-900/60 text-left hover:border-slate-700 transition flex items-center gap-3"
                                }
                            }
                        >
                            <span class="text-2xl">"📝"</span>
                            <div>
                                <div class="text-sm font-semibold text-white">"Text Note"</div>
                                <div class="text-xs text-slate-400">"Markdown file (index.md)"</div>
                            </div>
                        </button>

                        <button
                            type="button"
                            on:click=move |_| set_card_mode.set("media")
                            class=move || {
                                if card_mode.get() == "media" {
                                    "p-3 rounded-xl border border-teal-500 bg-teal-950/20 text-left transition flex items-center gap-3"
                                } else {
                                    "p-3 rounded-xl border border-slate-800 bg-slate-900/60 text-left hover:border-slate-700 transition flex items-center gap-3"
                                }
                            }
                        >
                            <span class="text-2xl">"🎬"</span>
                            <div>
                                <div class="text-sm font-semibold text-white">"Media File"</div>
                                <div class="text-xs text-slate-400">"Audio, Video, or Image"</div>
                            </div>
                        </button>
                    </div>
                </div>

                // Content Panel depending on mode
                {move || {
                    if card_mode.get() == "text" {
                        if active_tab.get() == "edit" {
                            view! {
                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">"Note Content (Markdown)"</label>
                                    <textarea
                                        rows="10"
                                        required
                                        placeholder="Write your note or flashcard content here..."
                                        prop:value=move || text_content.get()
                                        on:input=move |ev| set_text_content.set(event_target_value(&ev))
                                        class="w-full p-3.5 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition font-mono resize-y"
                                    />
                                </div>
                            }.into_view()
                        } else {
                            let preview = render_markdown(&text_content.get());
                            view! {
                                <div class="p-6 rounded-xl bg-slate-900/60 border border-slate-800 min-h-[240px]">
                                    <h3 class="text-xs font-mono uppercase tracking-wider text-teal-400 mb-3">"Rendered Note Preview"</h3>
                                    <div class="prose prose-invert max-w-none text-slate-200" inner_html=preview />
                                </div>
                            }.into_view()
                        }
                    } else {
                        // Media file upload & live local preview
                        let active_media_url = preview_url.get().or_else(|| existing_media_url.get());
                        let cur_ctype = media_type.get();

                        view! {
                            <div class="space-y-4">
                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">
                                        "Select Media File (Audio, Video, or Image)"
                                    </label>
                                    <input
                                        type="file"
                                        accept="audio/*,video/*,image/*"
                                        on:change=on_file_change
                                        class="block w-full text-xs text-slate-400 file:mr-4 file:py-2.5 file:px-4 file:rounded-lg file:border-0 file:text-xs file:font-semibold file:bg-teal-400 file:text-slate-950 hover:file:bg-teal-300 file:cursor-pointer cursor-pointer bg-slate-900 border border-slate-800 rounded-lg p-2"
                                    />
                                    <p class="text-[11px] text-slate-500 mt-1">
                                        "No file size limit. Uploaded directly to your Google Drive card folder on save."
                                    </p>
                                </div>

                                // Live Preview
                                {if let Some(src) = active_media_url {
                                    view! {
                                        <div class="p-5 rounded-xl bg-slate-900/80 border border-slate-800 space-y-3">
                                            <div class="flex items-center justify-between text-xs text-slate-400">
                                                <span class="font-semibold text-teal-300">"Live Local Preview"</span>
                                                <span class="font-mono text-slate-500">
                                                    {if selected_file.get().is_some() { "Local File" } else { "Current Drive Media" }}
                                                </span>
                                            </div>

                                            <div class="flex justify-center items-center py-4 bg-slate-950/60 rounded-lg">
                                                {match cur_ctype {
                                                    Some(CardType::Audio) => view! {
                                                        <audio controls=true class="w-full max-w-md" src=src />
                                                    }.into_view(),
                                                    Some(CardType::Video) => view! {
                                                        <video controls=true class="max-h-[50vh] w-full rounded" src=src />
                                                    }.into_view(),
                                                    Some(CardType::Image) => view! {
                                                        <img class="max-h-[50vh] rounded object-contain mx-auto" src=src alt="Preview" />
                                                    }.into_view(),
                                                    _ => view! {
                                                        // Fallback attempt with video / audio
                                                        <video controls=true class="max-h-[50vh] w-full rounded" src=src />
                                                    }.into_view(),
                                                }}
                                            </div>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="text-center py-12 border-2 border-dashed border-slate-800 rounded-xl text-slate-500 text-xs">
                                            "Select an audio or video file above to view live preview"
                                        </div>
                                    }.into_view()
                                }}
                            </div>
                        }.into_view()
                    }
                }}

                <div class="flex items-center justify-between pt-4 border-t border-slate-800">
                    <div>
                        {move || if card_id().is_some() {
                            view! {
                                <button
                                    type="button"
                                    on:click=on_delete
                                    class="px-4 py-2 text-xs font-semibold text-rose-400 hover:text-rose-300 transition"
                                >
                                    "Delete Card"
                                </button>
                            }.into_view()
                        } else {
                            view! { <div /> }.into_view()
                        }}
                    </div>
                    <div class="flex items-center gap-3">
                        <A
                            href="/"
                            class="px-4 py-2 text-xs font-medium text-slate-400 hover:text-white transition"
                        >
                            "Cancel"
                        </A>
                        <button
                            type="submit"
                            disabled=move || is_saving.get()
                            class="px-6 py-2.5 text-xs font-bold rounded-lg bg-teal-400 hover:bg-teal-300 disabled:opacity-50 text-slate-950 transition shadow-md shadow-teal-500/10"
                        >
                            {move || if is_saving.get() { "Uploading & Saving..." } else { "Save Card to Drive" }}
                        </button>
                    </div>
                </div>
            </form>
        </div>
    }
}
