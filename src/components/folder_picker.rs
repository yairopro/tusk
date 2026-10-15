use leptos::*;

#[component]
pub fn FolderPicker(
    api_key: String,
    client_id: String,
    current_folder_name: Option<String>,
    on_folder_selected: Callback<()>,
) -> impl IntoView {
    let api_key_clone = api_key.clone();
    let client_id_clone = client_id.clone();

    let open_picker = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let js_code = format!(
                r#"
                (function() {{
                    if (window.gapi && window.google && window.google.picker) {{
                        buildPicker();
                    }} else {{
                        loadGoogleApis();
                    }}

                    function loadGoogleApis() {{
                        const script1 = document.createElement('script');
                        script1.src = 'https://apis.google.com/js/api.js';
                        script1.onload = () => {{
                            gapi.load('picker', () => {{
                                const script2 = document.createElement('script');
                                script2.src = 'https://accounts.google.com/gsi/client';
                                script2.onload = () => buildPicker();
                                document.body.appendChild(script2);
                            }});
                        }};
                        document.body.appendChild(script1);
                    }}

                    function buildPicker() {{
                        const tokenClient = google.accounts.oauth2.initTokenClient({{
                            client_id: '{client_id}',
                            scope: 'https://www.googleapis.com/auth/drive',
                            callback: (response) => {{
                                if (response.error !== undefined) {{
                                    console.error(response);
                                    return;
                                }}
                                const view = new google.picker.DocsView(google.picker.ViewId.FOLDERS)
                                    .setMimeTypes('application/vnd.google-apps.folder')
                                    .setSelectFolderEnabled(true);

                                const picker = new google.picker.PickerBuilder()
                                    .addView(view)
                                    .setOAuthToken(response.access_token)
                                    .setDeveloperKey('{api_key}')
                                    .setCallback((data) => {{
                                        if (data[google.picker.Response.ACTION] == google.picker.Action.PICKED) {{
                                            const doc = data[google.picker.Response.DOCUMENTS][0];
                                            const folderId = doc[google.picker.Document.ID];
                                            const folderName = doc[google.picker.Document.NAME];
                                            fetch('/api/folder/select', {{
                                                method: 'POST',
                                                headers: {{ 'Content-Type': 'application/json' }},
                                                body: JSON.stringify({{ folder_id: folderId, folder_name: folderName }})
                                            }}).then(() => {{
                                                window.location.reload();
                                            }});
                                        }}
                                    }})
                                    .build();
                                picker.setVisible(true);
                            }},
                        }});
                        tokenClient.requestAccessToken({{ prompt: '' }});
                    }}
                }})();
                "#,
                client_id = client_id_clone,
                api_key = api_key_clone
            );
            let _ = js_sys::eval(&js_code);
        }
    };

    view! {
        <div class="flex items-center gap-3">
            {match current_folder_name {
                Some(name) => view! {
                    <span class="text-sm font-medium text-emerald-400 bg-emerald-950/60 border border-emerald-800/80 px-3 py-1.5 rounded-lg flex items-center gap-2">
                        <span>"📁"</span>
                        <span>{name}</span>
                    </span>
                }.into_view(),
                None => view! {
                    <span class="text-sm text-amber-400 bg-amber-950/50 border border-amber-800/60 px-3 py-1.5 rounded-lg">
                        "No folder selected"
                    </span>
                }.into_view(),
            }}
            <button
                on:click=open_picker
                class="px-3 py-1.5 text-xs font-semibold uppercase tracking-wider rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 transition"
            >
                "Choose Folder"
            </button>
        </div>
    }
}
