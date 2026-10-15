use leptos::*;
use rand::seq::SliceRandom;

use crate::models::{Card, CardSummary, CardType};

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
pub fn StudyPage() -> impl IntoView {
    let card_summaries = create_local_resource(
        || (),
        |_| async move {
            let resp = gloo_net::http::Request::get("/api/cards").send().await.ok()?;
            if resp.ok() {
                resp.json::<Vec<CardSummary>>().await.ok()
            } else {
                None
            }
        },
    );

    let (current_card_id, set_current_card_id) = create_signal(Option::<String>::None);

    // Pick initial random card when summaries load
    create_effect(move |_| {
        if let Some(Some(summaries)) = card_summaries.get() {
            if !summaries.is_empty() && current_card_id.get().is_none() {
                let mut rng = rand::thread_rng();
                if let Some(picked) = summaries.choose(&mut rng) {
                    set_current_card_id.set(Some(picked.id.clone()));
                }
            }
        }
    });

    // Resource for currently picked card
    let active_card = create_resource(
        move || current_card_id.get(),
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

    let pick_next_card = move || {
        if let Some(Some(summaries)) = card_summaries.get() {
            if !summaries.is_empty() {
                let mut rng = rand::thread_rng();
                if let Some(picked) = summaries.choose(&mut rng) {
                    set_current_card_id.set(Some(picked.id.clone()));
                }
            }
        }
    };

    view! {
        <div class="max-w-3xl mx-auto space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-xl font-bold text-white tracking-tight">"Recall Session"</h1>
                    <p class="text-xs text-slate-400">"Single-sided review from your Google Drive deck"</p>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        on:click=move |_| pick_next_card()
                        class="px-4 py-2 text-xs font-semibold rounded-lg bg-teal-400 hover:bg-teal-300 text-slate-950 transition flex items-center gap-1.5 shadow-sm"
                    >
                        <span>"Next Card 🎲"</span>
                    </button>
                </div>
            </div>

            <Suspense fallback=move || view! { <div class="text-slate-500 py-24 text-center">"Drawing a card..."</div> }>
                {move || {
                    match active_card.get() {
                        Some(Some(card)) => {
                            let icon = card.card_type.icon();
                            let label = card.card_type.label();
                            let card_content = card.content.clone();
                            let ctype = card.card_type;

                            view! {
                                <div class="space-y-4">
                                    <div class="p-6 md:p-8 rounded-2xl bg-slate-900 border border-slate-800 shadow-2xl flex flex-col justify-between">
                                        <div class="flex items-center justify-between pb-4 mb-4 border-b border-slate-800 text-xs text-slate-400">
                                            <div class="flex items-center gap-2">
                                                <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-slate-800 text-xs font-medium text-slate-300">
                                                    <span>{icon}</span>
                                                    <span>{label}</span>
                                                </span>
                                                <span class="font-semibold text-slate-200 text-base">{card.title.clone()}</span>
                                            </div>
                                            <span class="font-mono text-slate-500 text-[11px]">{card.filename.clone()}</span>
                                        </div>

                                        <div class="py-4">
                                            {match ctype {
                                                CardType::Text => {
                                                    let html = render_markdown(&card_content);
                                                    view! {
                                                        <div class="prose prose-invert max-w-none text-slate-100 text-base leading-relaxed" inner_html=html />
                                                    }.into_view()
                                                }
                                                CardType::Audio => {
                                                    view! {
                                                        <div class="flex flex-col items-center justify-center py-8 px-4 bg-slate-950/60 rounded-xl border border-slate-800/80">
                                                            <div class="text-4xl mb-4">"🎵"</div>
                                                            <audio controls=true class="w-full max-w-md" src=card_content />
                                                        </div>
                                                    }.into_view()
                                                }
                                                CardType::Video => {
                                                    view! {
                                                        <div class="flex justify-center bg-black rounded-xl overflow-hidden border border-slate-800">
                                                            <video controls=true class="w-full max-h-[60vh]" src=card_content />
                                                        </div>
                                                    }.into_view()
                                                }
                                                CardType::Image => {
                                                    view! {
                                                        <div class="flex justify-center py-2 bg-slate-950/40 rounded-xl border border-slate-800/60">
                                                            <img src=card_content alt=card.title.clone() class="max-w-full max-h-[60vh] rounded-lg object-contain" />
                                                        </div>
                                                    }.into_view()
                                                }
                                            }}
                                        </div>
                                    </div>

                                    <div class="flex items-center justify-center gap-4 pt-2">
                                        <button
                                            on:click=move |_| pick_next_card()
                                            class="px-6 py-2.5 text-xs font-bold rounded-xl bg-teal-400 hover:bg-teal-300 text-slate-950 transition shadow-md shadow-teal-500/10"
                                        >
                                            "Next Random Card →"
                                        </button>
                                    </div>
                                </div>
                            }.into_view()
                        }
                        Some(None) => view! {
                            <div class="text-center py-24 rounded-2xl bg-slate-900/40 border border-slate-800">
                                <p class="text-slate-400 text-sm mb-4">"No cards found to study. Pick a folder or create new cards."</p>
                                <a href="/" class="text-xs text-teal-400 font-semibold underline">"Return to Dashboard"</a>
                            </div>
                        }.into_view(),
                        None => view! {
                            <div class="text-center py-24 text-slate-500 text-sm">"Drawing random card..."</div>
                        }.into_view(),
                    }
                }}
            </Suspense>
        </div>
    }
}
