#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{
        extract::Path,
        routing::{get, post},
        Router,
    };
    use axum_extra::extract::cookie::Key;
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::env;
    use tower_http::services::ServeDir;
    use tusk::app::*;
    use tusk::server::*;

    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let session_secret = env::var("SESSION_SECRET")
        .unwrap_or_else(|_| "CHANGE_ME_32_BYTES_SESSION_KEY!!".to_string());
    let key_bytes = session_secret.as_bytes();
    let mut padded = [0u8; 64];
    let len = key_bytes.len().min(64);
    padded[..len].copy_from_slice(&key_bytes[..len]);
    let key = Key::from(&padded);

    let site_root = leptos_options.site_root.to_string();

    let app_state = AppState {
        leptos_options,
        cookie_key: key,
    };

    let app = Router::new()
        // Auth routes
        .route("/api/auth/login", get(tusk::auth::ssr::login_handler))
        .route("/api/auth/logout", get(tusk::auth::ssr::logout_handler))
        .route("/api/auth/callback", get(tusk::auth::ssr::callback_handler))
        // API routes
        .route("/api/auth/status", get(auth_status_handler))
        .route("/api/cards", get(list_cards_handler))
        .route("/api/cards/:card_id", get(get_card_handler).delete(delete_card_handler))
        .route("/api/cards/save", post(save_text_card_handler))
        .route("/api/cards/upload", post(upload_media_card_handler))
        .route("/api/folder/select", post(select_folder_handler))
        .route("/api/media/file/:file_id", get(media_proxy_handler))
        // Leptos routes
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let app_state = app_state.clone();
                move || provide_context(app_state.clone())
            },
            App,
        )
        .fallback_service(ServeDir::new(site_root))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("🐘 Tusk listening on http://{}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
