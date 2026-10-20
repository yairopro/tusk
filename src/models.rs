use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardType {
    Text,
    Audio,
    Video,
    Image,
}

impl CardType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "md" => Some(CardType::Text),
            "mp3" | "wav" | "ogg" | "m4a" | "aac" => Some(CardType::Audio),
            "mp4" | "webm" | "mov" | "mkv" => Some(CardType::Video),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => Some(CardType::Image),
            _ => None,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CardType::Text => "📝",
            CardType::Audio => "🎵",
            CardType::Video => "🎬",
            CardType::Image => "🖼️",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CardType::Text => "Text Note",
            CardType::Audio => "Audio",
            CardType::Video => "Video",
            CardType::Image => "Image",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: String,          // Folder ID
    pub name: String,        // Folder name (card title)
    pub title: String,       // Folder name
    pub card_type: CardType,
    pub file_id: String,     // index.<ext> file id
    pub filename: String,    // e.g. "index.md" or "index.mp3"
    pub content: String,     // Text markdown if Text, or media URL if Media
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSummary {
    pub id: String,          // Folder ID
    pub name: String,        // Folder name
    pub title: String,       // Folder name
    pub card_type: CardType,
    pub file_id: String,     // index.<ext> file id
    pub filename: String,    // e.g. "index.md"
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
pub struct SaveTextCardRequest {
    pub folder_id: Option<String>,
    pub title: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderItem {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateFolderRequest {
    pub parent_id: Option<String>,
    pub name: String,
}
