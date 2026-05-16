use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub target: String,
    #[serde(default)]
    pub args: serde_json::Value,
    pub risk: ActionRisk,
    #[serde(default)]
    pub requires_confirmation: bool,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: String,
    pub label: String,
    pub value: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    pub id: String,
    pub name: String,
    pub trigger: Trigger,
    pub actions: Vec<ActionSpec>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub tool_calls: Option<Vec<ActionSpec>>,
    #[serde(default)]
    pub status: Option<String>,
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
pub struct MemoryEntry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: String,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicFavorite {
    pub id: String,
    pub name: String,
    pub uri: Option<String>,
    pub query: Option<String>,
    pub kind: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    pub pc_control: bool,
    pub app_launch: bool,
    pub spotify_app: bool,
    pub spotify_api: bool,
    pub spotify_desktop_automation: bool,
    pub local_ai: bool,
    pub local_voice: bool,
    pub brightness_control: bool,
    pub window_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub mode: String,
    pub capabilities: RuntimeCapabilities,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceState {
    pub state: String,
    pub wake_word: String,
    pub engine_ready: bool,
    pub microphone_ready: bool,
    pub transcript: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskState {
    pub objective: Option<String>,
    pub completed_steps: Vec<String>,
    pub next_step: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicCommandResult {
    pub app_opened: bool,
    pub playback_started: bool,
    pub method_used: String,
    pub message: String,
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub level: String,
    pub module: String,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleProfile {
    pub name: String,
    pub email: String,
    pub picture: Option<String>,
    pub last_verified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStatus {
    pub google_connected: bool,
    pub profile: Option<GoogleProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSettings {
    pub apps: bool,
    pub windows: bool,
    pub clipboard: bool,
    pub screenshots: bool,
    pub processes: bool,
    pub power: bool,
    pub voice: bool,
}

impl Default for PermissionSettings {
    fn default() -> Self {
        Self {
            apps: true,
            windows: true,
            clipboard: true,
            screenshots: true,
            processes: true,
            power: true,
            voice: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantSettings {
    pub assistant_name: String,
    pub theme: String,
    pub language: String,
    pub start_with_windows: bool,
    pub minimize_to_tray: bool,
    pub operation_mode: String,
    pub local_ai_provider: String,
    pub local_ai_path: Option<String>,
    pub voice_enabled: bool,
    pub wake_word: String,
    pub speech_reply_enabled: bool,
    #[serde(default)]
    pub permissions: PermissionSettings,
    #[serde(default)]
    pub spotify_client_id: String,
    #[serde(default)]
    pub spotify_redirect_uri: String,
    #[serde(default)]
    pub system_volume: u8,
}

impl Default for AssistantSettings {
    fn default() -> Self {
        Self {
            assistant_name: "PC Control AI".to_string(),
            theme: "dark".to_string(),
            language: "pt-BR".to_string(),
            start_with_windows: false,
            minimize_to_tray: true,
            operation_mode: "auto_confirm_dangerous".to_string(),
            local_ai_provider: "qwen_embedded".to_string(),
            local_ai_path: None,
            voice_enabled: false,
            wake_word: "assistente".to_string(),
            speech_reply_enabled: false,
            permissions: PermissionSettings::default(),
            spotify_client_id: String::new(),
            spotify_redirect_uri: "http://127.0.0.1:8765/spotify/callback".to_string(),
            system_volume: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAiStatus {
    pub ready: bool,
    pub provider: String,
    pub model: String,
    pub state: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub onboarding_complete: bool,
    pub local_ai_ready: bool,
    pub local_ai_provider: String,
    pub local_ai_model: String,
    pub local_ai_state: String,
    pub local_ai_error: Option<String>,
    pub microphone_ready: bool,
    pub spotify_connected: bool,
    pub spotify_device_active: bool,
    pub system_status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    pub cpu: f32,
    pub memory: u64,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app: String,
    pub focused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub command: String,
    pub favorite: bool,
    pub recent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcControlState {
    pub volume: u8,
    pub brightness: u8,
    pub clipboard_preview: String,
    pub processes: Vec<ProcessInfo>,
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub id: String,
    pub command: String,
    pub actions: Vec<ActionSpec>,
    pub assistant_reply: String,
    pub result_summary: String,
    pub status: String,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub message: ChatMessage,
    pub proposed_actions: Vec<ActionSpec>,
}
