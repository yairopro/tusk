#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::models::{Card, CardSummary, CardType, DriveFileListResponse};
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, RANGE};
    use reqwest::Client;
    use serde_json::json;

    /// List all card subfolders inside the user's selected deck folder
    pub async fn list_flashcard_folders(
        client: &Client,
        access_token: &str,
        deck_folder_id: &str,
    ) -> Result<Vec<CardSummary>, String> {
        // Step 1: List all subfolders in deck folder
        let folder_query = format!(
            "'{}' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
            deck_folder_id.replace('\'', "\\'")
        );
        let folder_url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name)&pageSize=1000",
            urlencoding::encode(&folder_query)
        );

        let res = client
            .get(&folder_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Drive API list folders error: {}", err));
        }

        let folder_list: DriveFileListResponse = res.json().await.map_err(|e| e.to_string())?;
        if folder_list.files.is_empty() {
            return Ok(Vec::new());
        }

        // Step 2: For each folder, look for index.* file
        // To be fast, query files in these folders
        let mut summaries = Vec::new();

        // Check each folder for index.*
        for folder in folder_list.files {
            let file_query = format!(
                "'{}' in parents and trashed = false and name contains 'index.'",
                folder.id.replace('\'', "\\'")
            );
            let file_url = format!(
                "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType)",
                urlencoding::encode(&file_query)
            );

            let Ok(res) = client.get(&file_url).bearer_auth(access_token).send().await else {
                continue;
            };

            if let Ok(file_list) = res.json::<DriveFileListResponse>().await {
                // Find matching index.<ext>
                for f in file_list.files {
                    if let Some(ext) = f.name.strip_prefix("index.") {
                        if let Some(card_type) = CardType::from_extension(ext) {
                            summaries.push(CardSummary {
                                id: folder.id.clone(),
                                name: folder.name.clone(),
                                title: folder.name.clone(),
                                card_type,
                                file_id: f.id,
                                filename: f.name,
                            });
                            break; // 1 index file per folder
                        }
                    }
                }
            }
        }

        summaries.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        Ok(summaries)
    }

    /// Get details of a single flashcard from its folder
    pub async fn get_card(
        client: &Client,
        access_token: &str,
        folder_id: &str,
    ) -> Result<Card, String> {
        // Get folder metadata for title
        let folder_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id,name",
            folder_id
        );
        let folder_res = client
            .get(&folder_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !folder_res.status().is_success() {
            return Err("Card folder not found in Drive".to_string());
        }
        let folder_meta: serde_json::Value =
            folder_res.json().await.map_err(|e| e.to_string())?;
        let title = folder_meta["name"].as_str().unwrap_or("Untitled").to_string();

        // Find index.<ext> inside folder
        let file_query = format!(
            "'{}' in parents and trashed = false and name contains 'index.'",
            folder_id.replace('\'', "\\'")
        );
        let file_url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name,mimeType)",
            urlencoding::encode(&file_query)
        );

        let files_res = client
            .get(&file_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let file_list: DriveFileListResponse = files_res.json().await.map_err(|e| e.to_string())?;

        let mut matched = None;
        for f in file_list.files {
            if let Some(ext) = f.name.strip_prefix("index.") {
                if let Some(ctype) = CardType::from_extension(ext) {
                    matched = Some((f, ctype));
                    break;
                }
            }
        }

        let (file, card_type) = matched.ok_or_else(|| {
            format!("Folder '{}' does not contain a valid index.<ext> file", title)
        })?;

        let content = if card_type == CardType::Text {
            // Fetch markdown content
            let media_url = format!(
                "https://www.googleapis.com/drive/v3/files/{}?alt=media",
                file.id
            );
            let content_res = client
                .get(&media_url)
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            content_res.text().await.unwrap_or_default()
        } else {
            // Media proxy URL
            format!("/api/media/file/{}", file.id)
        };

        Ok(Card {
            id: folder_id.to_string(),
            name: title.clone(),
            title,
            card_type,
            file_id: file.id,
            filename: file.name,
            content,
        })
    }

    /// Create or update a text note card (`index.md`)
    pub async fn save_text_card(
        client: &Client,
        access_token: &str,
        deck_folder_id: &str,
        folder_id: Option<&str>,
        title: &str,
        content: &str,
    ) -> Result<String, String> {
        let card_folder_id = match folder_id {
            Some(id) if !id.is_empty() => {
                // Rename folder if title changed
                let rename_url = format!("https://www.googleapis.com/drive/v3/files/{}", id);
                let _ = client
                    .patch(&rename_url)
                    .bearer_auth(access_token)
                    .json(&json!({ "name": title }))
                    .send()
                    .await;
                id.to_string()
            }
            _ => {
                // Create card folder
                create_folder(client, access_token, deck_folder_id, title).await?
            }
        };

        // Check if index.md exists in card_folder_id
        let file_query = format!(
            "'{}' in parents and trashed = false and name = 'index.md'",
            card_folder_id.replace('\'', "\\'")
        );
        let file_url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id)",
            urlencoding::encode(&file_query)
        );

        let res = client
            .get(&file_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let list: DriveFileListResponse = res.json().await.map_err(|e| e.to_string())?;

        if let Some(existing) = list.files.into_iter().next() {
            // Update existing file content
            let update_url = format!(
                "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
                existing.id
            );
            client
                .patch(&update_url)
                .bearer_auth(access_token)
                .header(CONTENT_TYPE, "text/markdown; charset=UTF-8")
                .body(content.to_string())
                .send()
                .await
                .map_err(|e| e.to_string())?;
        } else {
            // Upload new index.md
            upload_raw_file(
                client,
                access_token,
                &card_folder_id,
                "index.md",
                "text/markdown",
                content.as_bytes(),
            )
            .await?;
        }

        Ok(card_folder_id)
    }

    /// Create or update a media card (`index.<ext>`)
    pub async fn save_media_card(
        client: &Client,
        access_token: &str,
        deck_folder_id: &str,
        folder_id: Option<&str>,
        title: &str,
        original_filename: &str,
        mime_type: &str,
        data: &[u8],
    ) -> Result<String, String> {
        let ext = original_filename
            .split('.')
            .last()
            .unwrap_or("bin")
            .to_lowercase();
        let target_filename = format!("index.{}", ext);

        let card_folder_id = match folder_id {
            Some(id) if !id.is_empty() => {
                // Rename folder if needed
                let rename_url = format!("https://www.googleapis.com/drive/v3/files/{}", id);
                let _ = client
                    .patch(&rename_url)
                    .bearer_auth(access_token)
                    .json(&json!({ "name": title }))
                    .send()
                    .await;
                id.to_string()
            }
            _ => create_folder(client, access_token, deck_folder_id, title).await?,
        };

        // Look for existing index.* files
        let file_query = format!(
            "'{}' in parents and trashed = false and name contains 'index.'",
            card_folder_id.replace('\'', "\\'")
        );
        let file_url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name)",
            urlencoding::encode(&file_query)
        );

        let res = client
            .get(&file_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let list: DriveFileListResponse = res.json().await.map_err(|e| e.to_string())?;

        // If an existing file has the exact same name, overwrite it; otherwise delete old and upload new
        let mut overwritten = false;
        for f in list.files {
            if f.name == target_filename {
                let update_url = format!(
                    "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
                    f.id
                );
                client
                    .patch(&update_url)
                    .bearer_auth(access_token)
                    .header(CONTENT_TYPE, mime_type)
                    .body(data.to_vec())
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                overwritten = true;
            } else {
                // Trash previous index file
                let del_url = format!("https://www.googleapis.com/drive/v3/files/{}", f.id);
                let _ = client.delete(&del_url).bearer_auth(access_token).send().await;
            }
        }

        if !overwritten {
            upload_raw_file(
                client,
                access_token,
                &card_folder_id,
                &target_filename,
                mime_type,
                data,
            )
            .await?;
        }

        Ok(card_folder_id)
    }

    /// Trash a card folder
    pub async fn delete_card_folder(
        client: &Client,
        access_token: &str,
        folder_id: &str,
    ) -> Result<(), String> {
        let url = format!("https://www.googleapis.com/drive/v3/files/{}", folder_id);
        let res = client
            .patch(&url)
            .bearer_auth(access_token)
            .json(&json!({ "trashed": true }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Drive delete folder error: {}", err));
        }

        Ok(())
    }

    /// Stream media by file_id with Range support
    pub async fn fetch_media_by_id(
        client: &Client,
        access_token: &str,
        file_id: &str,
        range_header: Option<&str>,
    ) -> Result<(String, Option<String>, Option<String>, reqwest::Response), String> {
        let meta_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,size",
            file_id
        );
        let meta_res = client
            .get(&meta_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !meta_res.status().is_success() {
            return Err("File not found in Drive".to_string());
        }

        let meta: serde_json::Value = meta_res.json().await.map_err(|e| e.to_string())?;
        let mime_type = meta["mimeType"]
            .as_str()
            .unwrap_or("application/octet-stream")
            .to_string();

        let media_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file_id
        );
        let mut req = client.get(&media_url).bearer_auth(access_token);
        if let Some(range) = range_header {
            req = req.header(RANGE, range);
        }

        let media_res = req.send().await.map_err(|e| e.to_string())?;
        if !media_res.status().is_success() {
            return Err("Drive media download failed".to_string());
        }

        let content_range = media_res
            .headers()
            .get("Content-Range")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let content_length = media_res
            .headers()
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok((mime_type, content_range, content_length, media_res))
    }

    // --- Helpers ---

    async fn create_folder(
        client: &Client,
        access_token: &str,
        parent_id: &str,
        name: &str,
    ) -> Result<String, String> {
        let url = "https://www.googleapis.com/drive/v3/files";
        let body = json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder",
            "parents": [parent_id]
        });

        let res = client
            .post(url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!("Failed to create folder: {}", err));
        }

        let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let id = data["id"]
            .as_str()
            .ok_or_else(|| "Missing folder id in response".to_string())?;
        Ok(id.to_string())
    }

    async fn upload_raw_file(
        client: &Client,
        access_token: &str,
        parent_id: &str,
        name: &str,
        mime_type: &str,
        data: &[u8],
    ) -> Result<String, String> {
        let metadata = json!({
            "name": name,
            "parents": [parent_id],
            "mimeType": mime_type
        });

        let boundary = "-------314159265358979323846";
        let delimiter = format!("\r\n--{}\r\n", boundary);
        let close_delim = format!("\r\n--{}--", boundary);

        let mut body = Vec::new();
        body.extend_from_slice(delimiter.as_bytes());
        body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
        body.extend_from_slice(metadata.to_string().as_bytes());
        body.extend_from_slice(delimiter.as_bytes());
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", mime_type).as_bytes());
        body.extend_from_slice(data);
        body.extend_from_slice(close_delim.as_bytes());

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
            return Err(format!("Drive API upload error: {}", err));
        }

        let response_json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let id = response_json["id"]
            .as_str()
            .ok_or_else(|| "Missing id in Drive response".to_string())?;
        Ok(id.to_string())
    }
}
