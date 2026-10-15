use leptos::*;
use rand::seq::SliceRandom;

use crate::models::{Card, CardSummary};

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
    let card_summaries = create_resource(
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
    let (is_flipped, set_is_flipped) = create_signal(false);

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
                set_is_flipped.set(false);
                let mut rng = rand::thread_rng();
                // Pure random with replacement as designed
                if let Some(picked) = summaries.choose(&mut rng) {
                    set_current_card_id.set(Some(picked.id.clone()));
                }
            }
        }
    };

    view! {
        <div class="max-w-2xl mx-auto space-y-6">
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-xl font-bold text-white tracking-tight">"Recall Session"</h1>
                    <p class="text-xs text-slate-400">"Pure random draws from your Drive collection"</p>
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
                            let q_html = render_markdown(&card.question);
                            let a_html = render_markdown(&card.answer);

                            view! {
                                <div class="space-y-4">
                                    // Card title & tags
                                    <div class="flex items-center justify-between text-xs text-slate-400 px-1">
                                        <span class="font-semibold text-slate-300">{card.title.clone()}</span>
                                        <div class="flex items-center gap-1 font-mono">
                                            {card.tags.iter().map(|tag| view! {
                                                <span class="bg-slate-800/80 px-2 py-0.5 rounded text-teal-400 text-[10px]">
                                                    {format!("#{}", tag)}
                                                </span>
                                            }).collect_view()}
                                        </div>
                                    </div>

                                    // Interactive Flashcard Box
                                    <div
                                        on:click=move |_| set_is_flipped.update(|f| *f = !*f)
                                        class="cursor-pointer min-h-[380px] p-8 rounded-2xl bg-slate-900 border border-slate-800 hover:border-slate-700 transition shadow-2xl flex flex-col justify-between select-none relative group"
                                    >
                                        <div class="prose prose-invert max-w-none">
                                            {move || {
                                                if is_flipped.get() {
                                                    view! {
                                                        <div>
                                                            <div class="text-[11px] font-mono uppercase tracking-wider text-teal-400 mb-4 pb-2 border-b border-slate-800 flex items-center justify-between">
                                                                <span>"Answer"</span>
                                                                <span class="text-slate-500 text-[10px]">"Click to see question"</span>
                                                            </div>
                                                            <div class="text-slate-200 text-base leading-relaxed" inner_html=a_html.clone() />
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    view! {
                                                        <div>
                                                            <div class="text-[11px] font-mono uppercase tracking-wider text-slate-500 mb-4 pb-2 border-b border-slate-800 flex items-center justify-between">
                                                                <span>"Question"</span>
                                                                <span class="text-slate-500 text-[10px]">"Click to reveal answer"</span>
                                                            </div>
                                                            <div class="text-slate-100 text-lg font-medium leading-relaxed" inner_html=q_html.clone() />
                                                        </div>
                                                    }.into_view()
                                                }
                                            }}
                                        </div>

                                        <div class="mt-6 pt-4 border-t border-slate-800/60 flex items-center justify-between text-xs text-slate-500">
                                            <span>
                                                {move || if is_flipped.get() { "Showing Answer" } else { "Showing Question" }}
                                            </span>
                                            <span class="group-hover:text-slate-300 transition">
                                                "Click to flip 🔄"
                                            </span>
                                        </div>
                                    </div>

                                    // Action buttons
                                    <div class="flex items-center justify-center gap-4 pt-2">
                                        <button
                                            on:click=move |_| set_is_flipped.update(|f| *f = !*f)
                                            class="px-5 py-2.5 text-xs font-semibold rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 transition"
                                        >
                                            {move || if is_flipped.get() { "Show Question" } else { "Reveal Answer" }}
                                        </button>
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
