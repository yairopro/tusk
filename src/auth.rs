use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct UserSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
    pub selected_folder_id: Option<String>,
    pub selected_folder_name: Option<String>,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use super::*;
    use axum::{
        extract::Query,
        response::{IntoResponse, Redirect},
    };
    use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};
    use reqwest::Client;
    use std::env;

    pub const SESSION_COOKIE_NAME: &str = "tusk_session";

    #[derive(Deserialize)]
    pub struct CallbackQuery {
        pub code: Option<String>,
        pub error: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct TokenResponse {
        pub access_token: String,
        pub refresh_token: Option<String>,
        pub expires_in: i64,
        pub token_type: String,
    }

    pub fn get_oauth_config() -> (String, String, String) {
        let client_id = env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
        let client_secret = env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
        let redirect_uri = env::var("GOOGLE_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:3000/api/auth/callback".to_string());
        (client_id, client_secret, redirect_uri)
    }

    pub async fn login_handler() -> impl IntoResponse {
        let (client_id, _, redirect_uri) = get_oauth_config();
        let scope = "https://www.googleapis.com/auth/drive";
        let url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(scope)
        );
        Redirect::temporary(&url)
    }

    pub async fn logout_handler(jar: PrivateCookieJar) -> (PrivateCookieJar, impl IntoResponse) {
        let mut cookie = Cookie::new(SESSION_COOKIE_NAME, "");
        cookie.set_path("/");
        cookie.set_max_age(time::Duration::seconds(0));
        let jar = jar.remove(cookie);
        (jar, Redirect::to("/"))
    }

    pub async fn callback_handler(
        Query(query): Query<CallbackQuery>,
        jar: PrivateCookieJar,
    ) -> Result<(PrivateCookieJar, Redirect), String> {
        let code = query
            .code
            .ok_or_else(|| query.error.unwrap_or_else(|| "Missing code".to_string()))?;

        let (client_id, client_secret, redirect_uri) = get_oauth_config();
        let client = Client::new();

        let params = [
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ];

        let res = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Token exchange failed: {}", err_body));
        }

        let token_data: TokenResponse = res.json().await.map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();

        let mut session = get_session_from_jar(&jar).unwrap_or_default();
        session.access_token = token_data.access_token;
        if let Some(rf) = token_data.refresh_token {
            session.refresh_token = Some(rf);
        }
        session.expires_at = now + token_data.expires_in;

        let jar = save_session_to_jar(jar, &session)?;
        Ok((jar, Redirect::to("/")))
    }

    pub fn get_session_from_jar(jar: &PrivateCookieJar) -> Option<UserSession> {
        let cookie = jar.get(SESSION_COOKIE_NAME)?;
        serde_json::from_str(cookie.value()).ok()
    }

    pub fn save_session_to_jar(
        jar: PrivateCookieJar,
        session: &UserSession,
    ) -> Result<PrivateCookieJar, String> {
        let json = serde_json::to_string(session).map_err(|e| e.to_string())?;
        let mut cookie = Cookie::new(SESSION_COOKIE_NAME, json);
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_same_site(SameSite::Lax);
        cookie.set_max_age(time::Duration::days(30));
        Ok(jar.add(cookie))
    }

    pub async fn ensure_valid_access_token(
        session: &mut UserSession,
    ) -> Result<String, String> {
        let now = chrono::Utc::now().timestamp();
        if now < session.expires_at - 60 {
            return Ok(session.access_token.clone());
        }

        let refresh_token = session
            .refresh_token
            .as_ref()
            .ok_or_else(|| "No refresh token available, re-auth required".to_string())?;

        let (client_id, client_secret, _) = get_oauth_config();
        let client = Client::new();

        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let res = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("Refresh token failed: {}", err_body));
        }

        let token_data: TokenResponse = res.json().await.map_err(|e| e.to_string())?;
        session.access_token = token_data.access_token.clone();
        if let Some(rf) = token_data.refresh_token {
            session.refresh_token = Some(rf);
        }
        session.expires_at = now + token_data.expires_in;

        Ok(session.access_token.clone())
    }
}
