#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use axum::{
        body::Body,
        extract::Path,
        http::{header, HeaderMap, HeaderValue, StatusCode},
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
        let folder_id = match &session.selected_folder_id {
            Some(f) => f.clone(),
            None => return Ok(Json(Vec::<CardSummary>::new())),
        };

        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let cards = list_flashcard_folders(&client, &access_token, &folder_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(cards))
    }

    pub async fn get_card_handler(
        jar: PrivateCookieJar,
        Path(card_id): Path<String>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let card = get_card(&client, &access_token, &card_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(card))
    }

    pub async fn save_text_card_handler(
        jar: PrivateCookieJar,
        Json(req): Json<SaveTextCardRequest>,
    ) -> Result<(PrivateCookieJar, impl IntoResponse), (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let deck_folder_id = session
            .selected_folder_id
            .as_ref()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "No folder selected".to_string()))?
            .clone();

        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let saved_id = save_text_card(
            &client,
            &access_token,
            &deck_folder_id,
            req.folder_id.as_deref(),
            &req.title,
            &req.content,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let jar = save_session_to_jar(jar, &session)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok((jar, Json(serde_json::json!({ "id": saved_id }))))
    }

    pub async fn upload_media_card_handler(
        jar: PrivateCookieJar,
        mut multipart: axum::extract::Multipart,
    ) -> Result<(PrivateCookieJar, impl IntoResponse), (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let deck_folder_id = session
            .selected_folder_id
            .as_ref()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "No folder selected".to_string()))?
            .clone();

        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let mut folder_id: Option<String> = None;
        let mut title = String::new();
        let mut filename = String::new();
        let mut mime_type = String::from("application/octet-stream");
        let mut file_bytes = Vec::new();

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
        {
            let name = field.name().unwrap_or("").to_string();
            match name.as_str() {
                "folder_id" => {
                    let val = field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                    if !val.is_empty() {
                        folder_id = Some(val);
                    }
                }
                "title" => {
                    title = field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                }
                "file" => {
                    if let Some(fname) = field.file_name() {
                        filename = fname.to_string();
                    }
                    if let Some(content_type) = field.content_type() {
                        mime_type = content_type.to_string();
                    }
                    file_bytes = field
                        .bytes()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                        .to_vec();
                }
                _ => {}
            }
        }

        if title.trim().is_empty() {
            title = filename
                .split('.')
                .next()
                .unwrap_or("media-card")
                .to_string();
        }

        if file_bytes.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()));
        }

        let client = Client::new();
        let saved_folder_id = save_media_card(
            &client,
            &access_token,
            &deck_folder_id,
            folder_id.as_deref(),
            &title,
            &filename,
            &mime_type,
            &file_bytes,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let jar = save_session_to_jar(jar, &session)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok((jar, Json(serde_json::json!({ "id": saved_folder_id }))))
    }

    pub async fn delete_card_handler(
        jar: PrivateCookieJar,
        Path(card_id): Path<String>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        delete_card_folder(&client, &access_token, &card_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(serde_json::json!({ "ok": true })))
    }

    #[derive(Deserialize)]
    pub struct SelectFolderRequest {
        pub folder_id: String,
        pub folder_name: String,
    }

    #[derive(Deserialize)]
    pub struct ListFoldersQuery {
        pub parent_id: Option<String>,
    }

    pub async fn list_folders_handler(
        jar: PrivateCookieJar,
        axum::extract::Query(query): axum::extract::Query<ListFoldersQuery>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let parent = query.parent_id.as_deref().unwrap_or("root");
        let folders = list_folders(&client, &access_token, parent)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(folders))
    }

    pub async fn create_folder_handler(
        jar: PrivateCookieJar,
        Json(req): Json<CreateFolderRequest>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let client = Client::new();
        let folder = create_subfolder(&client, &access_token, req.parent_id.as_deref(), &req.name)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Json(folder))
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
        Path(file_id): Path<String>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let mut session = get_session(&jar)?;
        let access_token = ensure_valid_access_token(&mut session)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e))?;

        let range_header = headers
            .get(header::RANGE)
            .and_then(|h| h.to_str().ok());

        let client = Client::new();
        let (mime_type, content_range, content_length, media_res) =
            fetch_media_by_id(&client, &access_token, &file_id, range_header)
                .await
                .map_err(|e| (StatusCode::NOT_FOUND, e))?;

        let status = if content_range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        };

        let mut res_headers = HeaderMap::new();
        res_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&mime_type)
                .unwrap_or(HeaderValue::from_static("application/octet-stream")),
        );
        res_headers.insert(
            header::ACCEPT_RANGES,
            HeaderValue::from_static("bytes"),
        );
        res_headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=3600"),
        );
        if let Some(range_val) = content_range {
            if let Ok(val) = HeaderValue::from_str(&range_val) {
                res_headers.insert(header::CONTENT_RANGE, val);
            }
        }
        if let Some(len_val) = content_length {
            if let Ok(val) = HeaderValue::from_str(&len_val) {
                res_headers.insert(header::CONTENT_LENGTH, val);
            }
        }

        let body = Body::from_stream(media_res.bytes_stream());
        Ok((status, res_headers, body))
    }
}
