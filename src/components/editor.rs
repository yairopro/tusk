use leptos::*;
use leptos_router::*;

use crate::models::{Card, SaveCardRequest};

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
    let (name, set_name) = create_signal(String::new());
    let (tags_str, set_tags_str) = create_signal(String::new());
    let (question, set_question) = create_signal(String::new());
    let (answer, set_answer) = create_signal(String::new());
    let (is_saving, set_is_saving) = create_signal(false);
    let (save_message, set_save_message) = create_signal(Option::<String>::None);
    let (active_tab, set_active_tab) = create_signal("edit"); // "edit" or "preview"

    // Load existing card if editing
    let existing_card = create_resource(
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
            set_name.set(card.name);
            set_tags_str.set(card.tags.join(", "));
            set_question.set(card.question);
            set_answer.set(card.answer);
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_is_saving.set(true);
        set_save_message.set(None);

        let t = title.get();
        let n = if name.get().trim().is_empty() {
            t.to_lowercase().replace(' ', "-")
        } else {
            name.get().trim().to_string()
        };

        let tags: Vec<String> = tags_str
            .get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let req = SaveCardRequest {
            card_id: card_id(),
            name: n,
            title: t,
            tags,
            question: question.get(),
            answer: answer.get(),
        };

        spawn_local(async move {
            let resp = gloo_net::http::Request::post("/api/cards/save")
                .header("Content-Type", "application/json")
                .json(&req);

            match resp {
                Ok(r) => {
                    let res = r.send().await;
                    set_is_saving.set(false);
                    match res {
                        Ok(resp) if resp.ok() => {
                            set_save_message.set(Some("Saved successfully to Google Drive!".to_string()));
                        }
                        _ => {
                            set_save_message.set(Some("Failed to save card. Check folder selection.".to_string()));
                        }
                    }
                }
                Err(_) => {
                    set_is_saving.set(false);
                    set_save_message.set(Some("Error creating request.".to_string()));
                }
            }
        });
    };

    view! {
        <div class="max-w-3xl mx-auto space-y-6">
            <div class="flex items-center justify-between pb-4 border-b border-slate-800">
                <div>
                    <h1 class="text-2xl font-bold text-white tracking-tight">
                        {move || if card_id().is_some() { "Edit Flashcard" } else { "Create Flashcard" }}
                    </h1>
                    <p class="text-xs text-slate-400 mt-1">
                        "Stored directly in your selected Google Drive folder as Markdown"
                    </p>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        on:click=move |_| set_active_tab.set("edit")
                        class=move || {
                            if active_tab.get() == "edit" {
                                "px-3 py-1.5 text-xs font-semibold rounded-lg bg-teal-400 text-slate-950 transition"
                            } else {
                                "px-3 py-1.5 text-xs font-semibold rounded-lg bg-slate-800 text-slate-300 hover:bg-slate-700 transition"
                            }
                        }
                    >
                        "Edit"
                    </button>
                    <button
                        type="button"
                        on:click=move |_| set_active_tab.set("preview")
                        class=move || {
                            if active_tab.get() == "preview" {
                                "px-3 py-1.5 text-xs font-semibold rounded-lg bg-teal-400 text-slate-950 transition"
                            } else {
                                "px-3 py-1.5 text-xs font-semibold rounded-lg bg-slate-800 text-slate-300 hover:bg-slate-700 transition"
                            }
                        }
                    >
                        "Live Preview"
                    </button>
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
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label class="block text-xs font-semibold text-slate-300 mb-1">"Card Title"</label>
                        <input
                            type="text"
                            required
                            placeholder="e.g. Mitochondria Function"
                            prop:value=move || title.get()
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            class="w-full px-3.5 py-2 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition"
                        />
                    </div>
                    <div>
                        <label class="block text-xs font-semibold text-slate-300 mb-1">"File Name (.md)"</label>
                        <input
                            type="text"
                            placeholder="e.g. mitochondria-function"
                            prop:value=move || name.get()
                            on:input=move |ev| set_name.set(event_target_value(&ev))
                            class="w-full px-3.5 py-2 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition font-mono"
                        />
                    </div>
                </div>

                <div>
                    <label class="block text-xs font-semibold text-slate-300 mb-1">"Tags (comma separated)"</label>
                    <input
                        type="text"
                        placeholder="biology, cells, exam2026"
                        prop:value=move || tags_str.get()
                        on:input=move |ev| set_tags_str.set(event_target_value(&ev))
                        class="w-full px-3.5 py-2 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition font-mono text-xs"
                    />
                </div>

                {move || {
                    if active_tab.get() == "edit" {
                        view! {
                            <div class="space-y-6">
                                <div>
                                    <div class="flex items-center justify-between mb-1">
                                        <label class="text-xs font-semibold text-slate-300">"Question (Front - Markdown)"</label>
                                        <span class="text-[11px] text-slate-500">"Relative media: ![alt](diagram.png)"</span>
                                    </div>
                                    <textarea
                                        rows="6"
                                        required
                                        placeholder="What is the primary role of the mitochondria?"
                                        prop:value=move || question.get()
                                        on:input=move |ev| set_question.set(event_target_value(&ev))
                                        class="w-full p-3.5 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition font-mono resize-y"
                                    />
                                </div>

                                <div>
                                    <label class="block text-xs font-semibold text-slate-300 mb-1">"Answer (Back - Markdown)"</label>
                                    <textarea
                                        rows="6"
                                        required
                                        placeholder="Generates most of the chemical energy needed to power the cell's biochemical reactions (ATP)."
                                        prop:value=move || answer.get()
                                        on:input=move |ev| set_answer.set(event_target_value(&ev))
                                        class="w-full p-3.5 rounded-lg bg-slate-900 border border-slate-800 focus:border-teal-500 text-white text-sm outline-none transition font-mono resize-y"
                                    />
                                </div>
                            </div>
                        }.into_view()
                    } else {
                        let q_preview = render_markdown(&question.get());
                        let a_preview = render_markdown(&answer.get());
                        view! {
                            <div class="space-y-6">
                                <div class="p-5 rounded-xl bg-slate-900/60 border border-slate-800">
                                    <h3 class="text-xs font-mono uppercase tracking-wider text-slate-400 mb-3">"Question Preview"</h3>
                                    <div class="prose prose-invert max-w-none text-slate-200" inner_html=q_preview />
                                </div>
                                <div class="p-5 rounded-xl bg-slate-900/60 border border-slate-800">
                                    <h3 class="text-xs font-mono uppercase tracking-wider text-teal-400 mb-3">"Answer Preview"</h3>
                                    <div class="prose prose-invert max-w-none text-slate-200" inner_html=a_preview />
                                </div>
                            </div>
                        }.into_view()
                    }
                }}

                <div class="flex items-center justify-end gap-3 pt-4 border-t border-slate-800">
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
                        {move || if is_saving.get() { "Saving to Drive..." } else { "Save Card to Drive" }}
                    </button>
                </div>
            </form>
        </div>
    }
}
