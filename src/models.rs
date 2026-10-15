use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Frontmatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub created: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub title: String,
    pub tags: Vec<String>,
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSummary {
    pub id: String,
    pub name: String,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub selected_folder_id: Option<String>,
    pub selected_folder_name: Option<String>,
    pub google_api_key: String,
    pub google_client_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType", default)]
    pub mime_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveFileListResponse {
    #[serde(default)]
    pub files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveCardRequest {
    pub card_id: Option<String>,
    pub name: String,
    pub title: String,
    pub tags: Vec<String>,
    pub question: String,
    pub answer: String,
}

impl Card {
    pub fn parse_markdown(id: String, name: String, raw_content: &str) -> Self {
        let trimmed = raw_content.trim();
        let (frontmatter, body) = if trimmed.starts_with("---") {
            let rest = &trimmed[3..];
            if let Some(end_idx) = rest.find("\n---") {
                let fm_str = &rest[..end_idx];
                let body_str = rest[end_idx + 4..].trim_start_matches(['\r', '\n']);
                let fm: Frontmatter = serde_yaml::from_str(fm_str).unwrap_or_default();
                (fm, body_str)
            } else {
                (Frontmatter::default(), trimmed)
            }
        } else {
            (Frontmatter::default(), trimmed)
        };

        let (question, answer) = if let Some(sep_idx) = body.find("\n---") {
            let q = body[..sep_idx].trim();
            let a = body[sep_idx + 4..].trim_start_matches(['\r', '\n']).trim();
            (q.to_string(), a.to_string())
        } else {
            (body.to_string(), String::new())
        };

        let default_name = name.trim_end_matches(".md").to_string();
        let title = frontmatter
            .title
            .unwrap_or_else(|| default_name.clone());
        let tags = frontmatter.tags.unwrap_or_default();

        Card {
            id,
            name: default_name,
            title,
            tags,
            question,
            answer,
        }
    }

    pub fn to_markdown(&self) -> String {
        let fm = Frontmatter {
            title: Some(self.title.clone()),
            tags: Some(self.tags.clone()),
            created: Some(chrono::Utc::now().to_rfc3339()),
        };
        let fm_yaml = serde_yaml::to_string(&fm).unwrap_or_default();
        format!(
            "---\n{}---\n{}\n---\n{}\n",
            fm_yaml,
            self.question.trim(),
            self.answer.trim()
        )
    }

    pub fn rewrite_media_urls(&mut self, folder_id: &str) {
        self.question = rewrite_urls(&self.question, folder_id);
        self.answer = rewrite_urls(&self.answer, folder_id);
    }
}

fn rewrite_urls(text: &str, folder_id: &str) -> String {
    let mut result = text.to_string();
    let prefixes = ["![", "<img src=\"", "<source src=\"", "<audio src=\"", "<video src=\""];
    for prefix in prefixes {
        let mut search_from = 0;
        while let Some(start_idx) = result[search_from..].find(prefix) {
            let abs_start = search_from + start_idx + prefix.len();
            let end_char = if prefix.ends_with('"') { '"' } else { ')' };
            let inner_start = if prefix == "![" {
                if let Some(bracket_end) = result[abs_start..].find("](") {
                    abs_start + bracket_end + 2
                } else {
                    search_from = abs_start;
                    continue;
                }
            } else {
                abs_start
            };

            if let Some(end_rel) = result[inner_start..].find(end_char) {
                let url = &result[inner_start..inner_start + end_rel];
                if !url.starts_with("http://")
                    && !url.starts_with("https://")
                    && !url.starts_with("/api/media/")
                    && !url.starts_with("data:")
                {
                    let filename = url.trim_start_matches("./");
                    let replacement = format!("/api/media/{}/{}", folder_id, filename);
                    result.replace_range(inner_start..inner_start + end_rel, &replacement);
                    search_from = inner_start + replacement.len() + 1;
                } else {
                    search_from = inner_start + end_rel + 1;
                }
            } else {
                break;
            }
        }
    }
    result
}
