use leptos::*;
use leptos_router::*;

use crate::components::folder_picker::FolderPicker;
use crate::models::AuthStatus;

#[component]
pub fn Nav() -> impl IntoView {
    let auth_status = create_local_resource(
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

    view! {
        <header class="border-b border-slate-800 bg-slate-900/70 backdrop-blur sticky top-0 z-50">
            <div class="max-w-5xl mx-auto px-4 h-16 flex items-center justify-between">
                <div class="flex items-center gap-6">
                    <A href="/" class="flex items-center gap-2.5 text-xl font-bold tracking-tight text-white hover:text-teal-400 transition">
                        <span>"Tusk"</span>
                    </A>
                    <nav class="hidden md:flex items-center gap-1 text-sm font-medium text-slate-300">
                        <A href="/" class="px-3 py-1.5 rounded-lg hover:bg-slate-800 hover:text-white transition">
                            "Cards"
                        </A>
                        <A href="/study" class="px-3 py-1.5 rounded-lg hover:bg-slate-800 hover:text-white transition">
                            "Study"
                        </A>
                        <A href="/editor" class="px-3 py-1.5 rounded-lg hover:bg-slate-800 hover:text-white transition">
                            "+ New Card"
                        </A>
                    </nav>
                </div>

                <div class="flex items-center gap-4">
                    <Suspense fallback=move || view! { <span class="text-xs text-slate-500">"Loading..."</span> }>
                        {move || {
                            auth_status.get().map(|status_opt| {
                                match status_opt {
                                    Some(status) if status.is_authenticated => {
                                        view! {
                                            <div class="flex items-center gap-3">
                                                <FolderPicker
                                                    api_key=status.google_api_key.clone()
                                                    client_id=status.google_client_id.clone()
                                                    current_folder_name=status.selected_folder_name.clone()
                                                    on_folder_selected=Callback::new(move |_| {
                                                        auth_status.refetch();
                                                    })
                                                />
                                                <a
                                                    href="/api/auth/logout"
                                                    rel="external"
                                                    class="text-xs text-slate-400 hover:text-rose-400 transition px-2 py-1"
                                                >
                                                    "Sign Out"
                                                </a>
                                            </div>
                                        }.into_view()
                                    }
                                    _ => {
                                        view! {
                                            <a
                                                href="/api/auth/login"
                                                rel="external"
                                                class="flex items-center gap-2 px-4 py-2 text-sm font-semibold text-slate-950 bg-teal-400 hover:bg-teal-300 rounded-lg shadow-sm transition"
                                            >
                                                <span>"Sign in with Google"</span>
                                            </a>
                                        }.into_view()
                                    }
                                }
                            })
                        }}
                    </Suspense>
                </div>
            </div>
        </header>
    }
}
