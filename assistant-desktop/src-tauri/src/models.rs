use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSpec {
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    #[serde(default)]
    pub args: Value,
    pub risk: ActionRisk,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: String,
    pub label: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger: Trigger,
    pub actions: Vec<ActionSpec>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub tool_calls: Option<Vec<ActionSpec>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    pub openai_connected: bool,
    pub spotify_connected: bool,
    pub spotify_device_active: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: Option<String>,
    pub image_url: Option<String>,
    pub uri: Option<String>,
    pub is_playing: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub tracks_total: u32,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub level: String,
    pub message: String,
    pub created_at: String,
    pub module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantSettings {
    pub model: String,
    pub language: String,
    pub main_hotkey: String,
    pub start_with_windows: bool,
    pub require_confirmation: bool,
    pub safe_mode: bool,
    pub spotify_client_id: String,
    pub spotify_redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub message: ChatMessage,
    pub proposed_actions: Vec<ActionSpec>,
}

impl Default for AssistantSettings {
    fn default() -> Self {
        Self {
            model: "gpt-5".to_string(),
            language: "pt-BR".to_string(),
            main_hotkey: "Ctrl+Shift+A".to_string(),
            start_with_windows: false,
            require_confirmation: true,
            safe_mode: true,
            spotify_client_id: String::new(),
            spotify_redirect_uri: "http://127.0.0.1:8765/callback".to_string(),
        }
    }
}
