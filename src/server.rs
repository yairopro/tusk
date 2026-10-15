#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use axum::{
        body::Body,
        extract::{Path, State},
        http::{header, StatusCode},
        response::{IntoResponse, Json},
    };
    use axum_extra::extract::cookie::{Key, PrivateCookieJar};
    use leptos::LeptosOptions;
    use reqwest::Client;
    use serde::Deserialize;
    use std::env;

    use crate::auth::ssr::*;
    use crate::auth::UserSession;
    use crate::drive::ssr::*;
    use crate::models::*;

    #[derive(Clone, axum::extract::FromRef)]
    pub struct AppState {
        pub leptos_options: LeptosOptions,
        pub cookie_key: Key,
    }

    fn get_session(jar: &PrivateCookieJar) -> Result<UserSession, (StatusCode, String)> {
        get_session_from_jar(jar).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Not authenticated".to_string(),
            )
        })
    }

    pub async fn auth_status_handler(
        jar: PrivateCookieJar,
    ) -> impl IntoResponse {
        let api_key = env::var("GOOGLE_API_KEY").unwrap_or_default();
        let client_id = env::var("GOOGLE_CLIENT_ID").unwrap_or_default();

        match get_session_from_jar(&jar) {
            Some(session) => Json(AuthStatus {
                is_authenticated: true,
                selected_folder_id: session.selected_folder_id,
                selected_folder_name: session.selected_folder_name,
                google_api_key: api_key,
                google_client_id: client_id,
            }),
            None => Json(AuthStatus {
                is_authenticated: false,
                selected_folder_id: None,
                selected_folder_name: None,
                google_api_key: api_key,
                google_client_id: client_id,
            }),
        }
    }

    pub async fn list_cards_handler(
        jar: PrivateCookieJar,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let folder_id = session
            .selected_folder_id
            .as_ref()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "No folder selected".to_string()))?
            .clone();

        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let cards = list_markdown_files(&client, &access_token, &folder_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(cards))
    }

    pub async fn get_card_handler(
        jar: PrivateCookieJar,
        Path(card_id): Path<String>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let folder_id = session
            .selected_folder_id
            .as_ref()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "No folder selected".to_string()))?
            .clone();

        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let card = get_card_content(&client, &access_token, &card_id, &card_id, &folder_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(card))
    }

    pub async fn save_card_handler(
        jar: PrivateCookieJar,
        Json(req): Json<SaveCardRequest>,
    ) -> Result<(PrivateCookieJar, impl IntoResponse), (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let folder_id = session
            .selected_folder_id
            .as_ref()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "No folder selected".to_string()))?
            .clone();

        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let card = Card {
            id: req.card_id.clone().unwrap_or_default(),
            name: req.name.clone(),
            title: req.title,
            tags: req.tags,
            question: req.question,
            answer: req.answer,
        };

        let client = Client::new();
        let saved_id = save_card_file(
            &client,
            &access_token,
            &folder_id,
            req.card_id.as_deref(),
            &card,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let jar = save_session_to_jar(jar, &session)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok((jar, Json(serde_json::json!({ "id": saved_id }))))
    }

    #[derive(Deserialize)]
    pub struct SelectFolderRequest {
        pub folder_id: String,
        pub folder_name: String,
    }

    pub async fn select_folder_handler(
        jar: PrivateCookieJar,
        Json(req): Json<SelectFolderRequest>,
    ) -> Result<(PrivateCookieJar, impl IntoResponse), (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        session.selected_folder_id = Some(req.folder_id);
        session.selected_folder_name = Some(req.folder_name);

        let jar = save_session_to_jar(jar, &session)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok((jar, Json(serde_json::json!({ "ok": true }))))
    }

    pub async fn media_proxy_handler(
        jar: PrivateCookieJar,
        Path((folder_id, filepath)): Path<(String, String)>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let filename = filepath.split('/').last().unwrap_or(&filepath);
        let client = Client::new();
        let (mime_type, response) =
            fetch_media_stream(&client, &access_token, &folder_id, filename)
                .await
                .map_err(|e| (StatusCode::NOT_FOUND, e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok((
            [
                (header::CONTENT_TYPE, mime_type),
                (
                    header::CACHE_CONTROL,
                    "public, max-age=3600".to_string(),
                ),
            ],
            bytes,
        ))
    }
}
