#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::models::{Card, CardSummary, DriveFileListResponse};
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    use reqwest::Client;
    use serde_json::json;

    pub async fn list_markdown_files(
        client: &Client,
        access_token: &str,
        folder_id: &str,
    ) -> Result<Vec<CardSummary>, String> {
        let query = format!(
            "'{}' in parents and trashed = false and name contains '.md'",
            folder_id.replace('\'', "\\'")
        );
        let url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType)&pageSize=1000",
            urlencoding::encode(&query)
        );

        let res = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Drive API list error: {}", err));
        }

        let list_data: DriveFileListResponse = res.json().await.map_err(|e| e.to_string())?;
        let summaries = list_data
            .files
            .into_iter()
            .map(|f| {
                let name = f.name.trim_end_matches(".md").to_string();
                CardSummary {
                    id: f.id,
                    name: name.clone(),
                    title: name,
                    tags: vec![],
                }
            })
            .collect();

        Ok(summaries)
    }

    pub async fn get_card_content(
        client: &Client,
        access_token: &str,
        file_id: &str,
        name: &str,
        folder_id: &str,
    ) -> Result<Card, String> {
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file_id
        );

        let res = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Drive API get media error: {}", err));
        }

        let raw_content = res.text().await.map_err(|e| e.to_string())?;
        let mut card = Card::parse_markdown(file_id.to_string(), name.to_string(), &raw_content);
        card.rewrite_media_urls(folder_id);
        Ok(card)
    }

    pub async fn save_card_file(
        client: &Client,
        access_token: &str,
        folder_id: &str,
        file_id: Option<&str>,
        card: &Card,
    ) -> Result<String, String> {
        let file_name = if card.name.ends_with(".md") {
            card.name.clone()
        } else {
            format!("{}.md", card.name)
        };
        let markdown = card.to_markdown();

        if let Some(id) = file_id {
            // Update existing file content
            let url = format!(
                "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
                id
            );
            let res = client
                .patch(&url)
                .bearer_auth(access_token)
                .header(CONTENT_TYPE, "text/markdown; charset=UTF-8")
                .body(markdown)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !res.status().is_success() {
                let err = res.text().await.unwrap_or_default();
                return Err(format!("Drive API update file error: {}", err));
            }
            Ok(id.to_string())
        } else {
            // Create new file with multipart upload
            let metadata = json!({
                "name": file_name,
                "parents": [folder_id],
                "mimeType": "text/markdown"
            });

            let boundary = "-------314159265358979323846";
            let delimiter = format!("\r\n--{}\r\n", boundary);
            let close_delim = format!("\r\n--{}--", boundary);

            let mut body = String::new();
            body.push_str(&delimiter);
            body.push_str("Content-Type: application/json; charset=UTF-8\r\n\r\n");
            body.push_str(&metadata.to_string());
            body.push_str(&delimiter);
            body.push_str("Content-Type: text/markdown; charset=UTF-8\r\n\r\n");
            body.push_str(&markdown);
            body.push_str(&close_delim);

            let url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart";
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
            );
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(&format!("multipart/related; boundary={}", boundary)).unwrap(),
            );

            let res = client
                .post(url)
                .headers(headers)
                .body(body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !res.status().is_success() {
                let err = res.text().await.unwrap_or_default();
                return Err(format!("Drive API create file error: {}", err));
            }

            let response_json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
            let created_id = response_json["id"]
                .as_str()
                .ok_or_else(|| "Missing id in Drive response".to_string())?
                .to_string();

            Ok(created_id)
        }
    }

    pub async fn fetch_media_stream(
        client: &Client,
        access_token: &str,
        folder_id: &str,
        filename: &str,
    ) -> Result<(String, reqwest::Response), String> {
        let query = format!(
            "'{}' in parents and trashed = false and name = '{}'",
            folder_id.replace('\'', "\\'"),
            filename.replace('\'', "\\'")
        );
        let search_url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType)",
            urlencoding::encode(&query)
        );

        let search_res = client
            .get(&search_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let list_data: DriveFileListResponse = search_res.json().await.map_err(|e| e.to_string())?;
        let file = list_data
            .files
            .into_iter()
            .next()
            .ok_or_else(|| format!("Media file '{}' not found in folder", filename))?;

        let media_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file.id
        );
        let media_res = client
            .get(&media_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !media_res.status().is_success() {
            return Err("Failed to download media file".to_string());
        }

        let mime = if !file.mime_type.is_empty() {
            file.mime_type
        } else {
            "application/octet-stream".to_string()
        };

        Ok((mime, media_res))
    }
}
