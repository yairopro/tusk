use leptos::*;
use leptos_router::*;

use crate::models::{AuthStatus, CardSummary};

#[component]
pub fn HomePage() -> impl IntoView {
    let auth_res = create_local_resource(
        || (),
        |_| async move {
            gloo_net::http::Request::get("/api/auth/status")
                .send()
                .await
                .ok()?
                .json::<AuthStatus>()
                .await
                .ok()
        },
    );

    let (sync_trigger, set_sync_trigger) = create_signal(0);

    let cards_res = create_local_resource(
        move || sync_trigger.get(),
        |_| async move {
            let resp = gloo_net::http::Request::get("/api/cards").send().await.ok()?;
            if resp.ok() {
                resp.json::<Vec<CardSummary>>().await.ok()
            } else {
                None
            }
        },
    );

    view! {
        <div class="space-y-8">
            <Suspense fallback=move || view! { <div class="text-slate-400">"Loading..."</div> }>
                {move || {
                    let auth = auth_res.get().flatten();
                    let is_authed = auth.as_ref().map(|a| a.is_authenticated).unwrap_or(false);
                    let folder_name = auth.as_ref().and_then(|a| a.selected_folder_name.clone());

                    if !is_authed {
                        view! {
                            <div class="text-center py-20 px-4 rounded-2xl bg-slate-900/40 border border-slate-800">
                                <h1 class="text-3xl font-extrabold text-white tracking-tight mb-3">
                                    "Serendipitous Recall with Tusk"
                                </h1>
                                <p class="text-slate-400 max-w-md mx-auto mb-8 text-sm leading-relaxed">
                                    "Pure random flashcards stored right in your Google Drive as plain Markdown files. Keep full ownership of your knowledge."
                                </p>
                                <a
                                    href="/api/auth/login"
                                    rel="external"
                                    class="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-teal-400 hover:bg-teal-300 text-slate-950 font-bold transition shadow-lg shadow-teal-500/10"
                                >
                                    "Connect Google Drive"
                                </a>
                            </div>
                        }.into_view()
                    } else if folder_name.is_none() {
                        view! {
                            <div class="text-center py-16 px-4 rounded-2xl bg-slate-900/40 border border-amber-900/30">
                                <span class="text-4xl mb-3 inline-block">"📁"</span>
                                <h2 class="text-xl font-bold text-white mb-2">"No Flashcard Folder Selected"</h2>
                                <p class="text-slate-400 max-w-md mx-auto text-sm">
                                    "Click 'Choose Folder' in the top navigation to pick or create your flashcard folder in Google Drive."
                                </p>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="space-y-6">
                                <div class="flex items-center justify-between pb-4 border-b border-slate-800">
                                    <div>
                                        <h1 class="text-2xl font-bold text-white tracking-tight">"Flashcards"</h1>
                                        <p class="text-xs text-slate-400 mt-1">
                                            "Syncs directly with your Google Drive folder: "
                                            <span class="text-teal-400 font-semibold">{folder_name.unwrap_or_default()}</span>
                                        </p>
                                    </div>
                                    <div class="flex items-center gap-3">
                                        <button
                                            on:click=move |_| set_sync_trigger.update(|n| *n += 1)
                                            class="px-4 py-2 text-xs font-semibold uppercase tracking-wider rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 transition flex items-center gap-2"
                                        >
                                            <span>"🔄"</span>
                                            <span>"Sync Cards"</span>
                                        </button>
                                        <A
                                            href="/study"
                                            class="px-5 py-2 text-sm font-bold rounded-lg bg-teal-400 hover:bg-teal-300 text-slate-950 transition shadow-sm"
                                        >
                                            "Start Study Session"
                                        </A>
                                    </div>
                                </div>

                                <Suspense fallback=move || view! { <div class="text-slate-500 py-12 text-center">"Loading cards from Google Drive..."</div> }>
                                    {move || {
                                        cards_res.get().map(|cards_opt| {
                                            match cards_opt {
                                                Some(cards) if !cards.is_empty() => {
                                                    let count = cards.len();
                                                    view! {
                                                        <div class="space-y-4">
                                                            <div class="text-xs text-slate-500 font-mono">
                                                                {format!("{} flashcards found", count)}
                                                            </div>
                                                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                                {cards.into_iter().map(|card| {
                                                                    let edit_url = format!("/editor/{}", card.id);
                                                                    let icon = card.card_type.icon();
                                                                    let type_label = card.card_type.label();
                                                                    view! {
                                                                        <div class="p-5 rounded-xl bg-slate-900/60 border border-slate-800 hover:border-slate-700 transition flex flex-col justify-between group">
                                                                            <div>
                                                                                <div class="flex items-center justify-between mb-2">
                                                                                    <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-slate-800 text-xs font-medium text-slate-300">
                                                                                        <span>{icon}</span>
                                                                                        <span>{type_label}</span>
                                                                                    </span>
                                                                                    <span class="text-xs text-slate-500 font-mono">{card.filename}</span>
                                                                                </div>
                                                                                <h3 class="text-base font-semibold text-slate-200 group-hover:text-teal-300 transition line-clamp-2">
                                                                                    {card.title}
                                                                                </h3>
                                                                            </div>
                                                                            <div class="mt-4 pt-3 border-t border-slate-800/80 flex items-center justify-end text-xs">
                                                                                <A
                                                                                    href=edit_url
                                                                                    class="text-teal-400 hover:text-teal-300 font-medium"
                                                                                >
                                                                                    "Edit →"
                                                                                </A>
                                                                            </div>
                                                                        </div>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        </div>
                                                    }.into_view()
                                                }
                                                Some(_) => {
                                                    view! {
                                                        <div class="text-center py-16 rounded-xl bg-slate-900/30 border border-slate-800">
                                                            <p class="text-slate-400 text-sm mb-4">"No markdown cards found in this folder."</p>
                                                            <A
                                                                href="/editor"
                                                                class="px-4 py-2 text-xs font-bold rounded-lg bg-teal-400 hover:bg-teal-300 text-slate-950 transition"
                                                            >
                                                                "+ Create First Card"
                                                            </A>
                                                        </div>
                                                    }.into_view()
                                                }
                                                None => {
                                                    view! {
                                                        <div class="text-rose-400 text-sm py-4">"Failed to load cards. Check folder permissions."</div>
                                                    }.into_view()
                                                }
                                            }
                                        })
                                    }}
                                </Suspense>
                            </div>
                        }.into_view()
                    }
                }}
            </Suspense>
        </div>
    }
}
