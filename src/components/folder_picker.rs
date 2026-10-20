use leptos::*;
use serde::{Deserialize, Serialize};

use crate::models::{CreateFolderRequest, FolderItem};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct BreadcrumbItem {
    id: String,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SelectFolderBody {
    folder_id: String,
    folder_name: String,
}

#[component]
pub fn FolderPicker(
    #[prop(optional)] api_key: String,
    #[prop(optional)] client_id: String,
    current_folder_name: Option<String>,
    on_folder_selected: Callback<()>,
) -> impl IntoView {
    let _ = (&api_key, &client_id);

    let (is_open, set_is_open) = create_signal(false);
    let (breadcrumbs, set_breadcrumbs) = create_signal(vec![BreadcrumbItem {
        id: "root".to_string(),
        name: "My Drive".to_string(),
    }]);

    let (refresh_trigger, set_refresh_trigger) = create_signal(0);
    let (error_msg, set_error_msg) = create_signal(Option::<String>::None);
    let (is_saving, set_is_saving) = create_signal(false);

    let (show_new_folder_input, set_show_new_folder_input) = create_signal(false);
    let (new_folder_name, set_new_folder_name) = create_signal(String::new());
    let (is_creating_folder, set_is_creating_folder) = create_signal(false);

    let current_dir = move || {
        breadcrumbs
            .get()
            .last()
            .cloned()
            .unwrap_or(BreadcrumbItem {
                id: "root".to_string(),
                name: "My Drive".to_string(),
            })
    };

    let folders_resource = create_local_resource(
        move || (is_open.get(), current_dir().id, refresh_trigger.get()),
        move |(open, parent_id, _)| async move {
            if !open {
                return Vec::new();
            }
            let url = format!("/api/folders?parent_id={}", urlencoding::encode(&parent_id));
            match gloo_net::http::Request::get(&url).send().await {
                Ok(resp) if resp.ok() => resp.json::<Vec<FolderItem>>().await.unwrap_or_default(),
                Ok(resp) => {
                    let err = resp.text().await.unwrap_or_else(|_| "Error".to_string());
                    set_error_msg.set(Some(format!("Failed to load folders: {}", err)));
                    Vec::new()
                }
                Err(e) => {
                    set_error_msg.set(Some(e.to_string()));
                    Vec::new()
                }
            }
        },
    );

    let open_modal = move |_| {
        set_error_msg.set(None);
        set_show_new_folder_input.set(false);
        set_new_folder_name.set(String::new());
        set_breadcrumbs.set(vec![BreadcrumbItem {
            id: "root".to_string(),
            name: "My Drive".to_string(),
        }]);
        set_is_open.set(true);
    };

    let close_modal = move |_| {
        set_is_open.set(false);
        set_show_new_folder_input.set(false);
    };

    let navigate_to_crumb = move |idx: usize| {
        set_error_msg.set(None);
        set_show_new_folder_input.set(false);
        set_breadcrumbs.update(|crumbs| {
            if idx + 1 < crumbs.len() {
                crumbs.truncate(idx + 1);
            }
        });
    };

    let drill_into = move |folder: FolderItem| {
        set_error_msg.set(None);
        set_show_new_folder_input.set(false);
        set_breadcrumbs.update(|crumbs| {
            crumbs.push(BreadcrumbItem {
                id: folder.id,
                name: folder.name,
            });
        });
    };

    let create_folder_action = move || {
        let name = new_folder_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let parent = current_dir().id;
        set_is_creating_folder.set(true);
        set_error_msg.set(None);

        spawn_local(async move {
            let body = CreateFolderRequest {
                parent_id: Some(parent),
                name,
            };
            let resp = gloo_net::http::Request::post("/api/folders/create")
                .json(&body)
                .unwrap()
                .send()
                .await;

            set_is_creating_folder.set(false);
            match resp {
                Ok(r) if r.ok() => {
                    set_show_new_folder_input.set(false);
                    set_new_folder_name.set(String::new());
                    set_refresh_trigger.update(|n| *n += 1);
                }
                Ok(r) => {
                    let err = r.text().await.unwrap_or_default();
                    set_error_msg.set(Some(format!("Failed to create folder: {}", err)));
                }
                Err(e) => {
                    set_error_msg.set(Some(e.to_string()));
                }
            }
        });
    };

    let select_current_folder = move |_| {
        let current = current_dir();
        set_is_saving.set(true);
        set_error_msg.set(None);

        spawn_local(async move {
            let body = SelectFolderBody {
                folder_id: current.id.clone(),
                folder_name: current.name.clone(),
            };
            let resp = gloo_net::http::Request::post("/api/folder/select")
                .json(&body)
                .unwrap()
                .send()
                .await;

            set_is_saving.set(false);
            match resp {
                Ok(r) if r.ok() => {
                    set_is_open.set(false);
                    on_folder_selected.call(());
                }
                Ok(r) => {
                    let err = r.text().await.unwrap_or_default();
                    set_error_msg.set(Some(format!("Failed to select folder: {}", err)));
                }
                Err(e) => {
                    set_error_msg.set(Some(e.to_string()));
                }
            }
        });
    };

    view! {
        <div class="flex items-center gap-2">
            {match current_folder_name {
                Some(name) => view! {
                    <span class="text-xs sm:text-sm font-medium text-emerald-400 bg-emerald-950/60 border border-emerald-800/80 px-2.5 sm:px-3 py-1 sm:py-1.5 rounded-lg flex items-center gap-1.5 max-w-[140px] sm:max-w-xs truncate">
                        <span>"📁"</span>
                        <span class="truncate">{name}</span>
                    </span>
                }.into_view(),
                None => view! {
                    <span class="text-xs sm:text-sm text-amber-400 bg-amber-950/50 border border-amber-800/60 px-2.5 sm:px-3 py-1 sm:py-1.5 rounded-lg whitespace-nowrap">
                        "No folder"
                    </span>
                }.into_view(),
            }}

            <button
                on:click=open_modal
                class="px-3 py-1 sm:py-1.5 text-xs sm:text-sm font-medium rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 transition flex items-center gap-1.5 whitespace-nowrap"
            >
                <span>"📁"</span>
                <span class="hidden sm:inline">"Change Folder"</span>
                <span class="sm:hidden">"Change"</span>
            </button>

            // Modal
            {move || if is_open.get() {
                view! {
                    <div class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-slate-950/80 backdrop-blur-sm">
                        <div class="relative w-full max-w-lg bg-slate-900 border border-slate-800 rounded-2xl shadow-2xl flex flex-col max-h-[88vh] overflow-hidden animate-in fade-in zoom-in-95 duration-150">
                            // Header
                            <div class="px-4 sm:px-5 py-3.5 border-b border-slate-800 flex items-center justify-between bg-slate-900/90">
                                <div>
                                    <h3 class="text-base font-bold text-white flex items-center gap-2">
                                        <span>"📁"</span>
                                        <span>"Select Flashcards Folder"</span>
                                    </h3>
                                    <p class="text-xs text-slate-400 mt-0.5">
                                        "Browse your Google Drive and pick your flashcards deck."
                                    </p>
                                </div>
                                <button
                                    on:click=close_modal
                                    class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 transition text-lg leading-none"
                                    aria-label="Close"
                                >
                                    "✕"
                                </button>
                            </div>

                            // Breadcrumbs navigation
                            <div class="px-4 sm:px-5 py-2.5 bg-slate-950/40 border-b border-slate-800/80 flex items-center gap-1 text-xs text-slate-400 overflow-x-auto whitespace-nowrap scrollbar-thin">
                                {move || {
                                    let crumbs = breadcrumbs.get();
                                    let len = crumbs.len();
                                    crumbs.into_iter().enumerate().map(|(idx, crumb)| {
                                        let is_last = idx == len - 1;
                                        view! {
                                            <div class="flex items-center gap-1">
                                                {if idx > 0 {
                                                    view! { <span class="text-slate-600">"/"</span> }.into_view()
                                                } else {
                                                    view! { <span class="text-slate-500">"Drive:"</span> }.into_view()
                                                }}
                                                <button
                                                    on:click=move |_| navigate_to_crumb(idx)
                                                    class=if is_last {
                                                        "font-semibold text-teal-400 hover:underline max-w-[140px] truncate"
                                                    } else {
                                                        "text-slate-300 hover:text-white hover:underline max-w-[120px] truncate"
                                                    }
                                                >
                                                    {crumb.name}
                                                </button>
                                            </div>
                                        }
                                    }).collect_view()
                                }}
                            </div>

                            // Error notice
                            {move || error_msg.get().map(|err| view! {
                                <div class="mx-4 mt-3 p-2.5 rounded-lg bg-rose-950/60 border border-rose-800/80 text-rose-300 text-xs flex items-center justify-between">
                                    <span>{err}</span>
                                    <button on:click=move |_| set_error_msg.set(None) class="text-rose-400 hover:text-rose-200">"✕"</button>
                                </div>
                            })}

                            // Folder listing body
                            <div class="p-3 sm:p-4 overflow-y-auto flex-1 divide-y divide-slate-800/60 min-h-[200px]">
                                <Suspense fallback=move || view! {
                                    <div class="py-12 flex flex-col items-center justify-center gap-2 text-slate-400">
                                        <div class="w-6 h-6 border-2 border-teal-400 border-t-transparent rounded-full animate-spin"></div>
                                        <span class="text-xs">"Loading folders..."</span>
                                    </div>
                                }>
                                    {move || {
                                        folders_resource.get().map(|folders| {
                                            if folders.is_empty() {
                                                view! {
                                                    <div class="py-10 text-center text-slate-500 text-xs">
                                                        <span class="text-2xl block mb-1">"📂"</span>
                                                        "No subfolders found in this directory."
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <div class="space-y-1">
                                                        {folders.into_iter().map(|folder| {
                                                            let f_clone = folder.clone();
                                                            view! {
                                                                <button
                                                                    on:click=move |_| drill_into(f_clone.clone())
                                                                    class="w-full text-left px-3 py-2.5 rounded-xl hover:bg-slate-800/80 active:bg-slate-800 transition flex items-center justify-between group"
                                                                >
                                                                    <div class="flex items-center gap-2.5 min-w-0 pr-2">
                                                                        <span class="text-base text-amber-400 shrink-0">"📁"</span>
                                                                        <span class="text-sm font-medium text-slate-200 group-hover:text-teal-300 transition truncate">
                                                                            {folder.name}
                                                                        </span>
                                                                    </div>
                                                                    <span class="text-xs text-slate-500 group-hover:text-slate-300 shrink-0">
                                                                        "Open →"
                                                                    </span>
                                                                </button>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                }.into_view()
                                            }
                                        })
                                    }}
                                </Suspense>
                            </div>

                            // New Folder input block
                            {move || if show_new_folder_input.get() {
                                view! {
                                    <div class="px-4 py-3 bg-slate-950/60 border-t border-slate-800 flex items-center gap-2">
                                        <input
                                            type="text"
                                            placeholder="Folder name..."
                                            prop:value=move || new_folder_name.get()
                                            on:input=move |ev| set_new_folder_name.set(event_target_value(&ev))
                                            on:keydown=move |ev| {
                                                if ev.key() == "Enter" {
                                                    create_folder_action();
                                                }
                                            }
                                            class="flex-1 bg-slate-800 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-teal-400"
                                        />
                                        <button
                                            on:click=move |_| create_folder_action()
                                            disabled=move || is_creating_folder.get() || new_folder_name.get().trim().is_empty()
                                            class="px-3 py-1.5 rounded-lg bg-teal-400 text-slate-950 hover:bg-teal-300 disabled:opacity-50 text-xs font-semibold"
                                        >
                                            {if is_creating_folder.get() { "Creating..." } else { "Create" }}
                                        </button>
                                        <button
                                            on:click=move |_| set_show_new_folder_input.set(false)
                                            class="px-2.5 py-1.5 rounded-lg text-slate-400 hover:text-white text-xs"
                                        >
                                            "Cancel"
                                        </button>
                                    </div>
                                }.into_view()
                            } else {
                                view! { <div></div> }.into_view()
                            }}

                            // Footer actions
                            <div class="px-4 sm:px-5 py-3 border-t border-slate-800 bg-slate-900/90 flex flex-wrap items-center justify-between gap-2.5">
                                <button
                                    on:click=move |_| set_show_new_folder_input.update(|show| *show = !*show)
                                    class="text-xs text-teal-400 hover:text-teal-300 font-medium flex items-center gap-1.5 px-2 py-1.5 rounded-lg hover:bg-slate-800 transition"
                                >
                                    <span>"➕"</span>
                                    <span>"New Folder"</span>
                                </button>

                                <div class="flex items-center gap-2 ml-auto">
                                    <button
                                        on:click=close_modal
                                        class="px-3.5 py-2 text-xs font-medium text-slate-400 hover:text-white rounded-lg hover:bg-slate-800 transition"
                                    >
                                        "Cancel"
                                    </button>
                                    <button
                                        on:click=select_current_folder
                                        disabled=move || is_saving.get()
                                        class="px-4 py-2 text-xs font-bold rounded-lg bg-teal-400 hover:bg-teal-300 text-slate-950 transition shadow-sm disabled:opacity-50 flex items-center gap-1.5"
                                    >
                                        <span>"✓"</span>
                                        <span>
                                            {move || if is_saving.get() {
                                                "Selecting...".to_string()
                                            } else {
                                                format!("Select \"{}\"", current_dir().name)
                                            }}
                                        </span>
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { <div></div> }.into_view()
            }}
        </div>
    }
}
