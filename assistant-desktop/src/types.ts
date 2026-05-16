export type AppView =
  | "home"
  | "pc_control"
  | "apps"
  | "music"
  | "voice"
  | "automations"
  | "history"
  | "settings";

export type ChatMode = "texto" | "voz" | "comando";

export type ActionRisk = "low" | "medium" | "high";

export type ActionType =
  | "open_app"
  | "close_app"
  | "open_url"
  | "open_path"
  | "copy_text"
  | "system_volume"
  | "brightness"
  | "take_screenshot"
  | "list_processes"
  | "close_process"
  | "power"
  | "window_minimize"
  | "window_maximize"
  | "window_focus"
  | "spotify_play"
  | "spotify_pause"
  | "spotify_next"
  | "spotify_previous"
  | "spotify_volume"
  | "spotify_liked"
  | "play_music"
  | "play_liked_music"
  | "pause_music"
  | "next_track"
  | "previous_track"
  | "save_memory"
  | "save_music_favorite"
  | "run_routine";

export interface ActionSpec {
  type: ActionType;
  label: string;
  target: string;
  args: Record<string, string | number | boolean | null>;
  risk: ActionRisk;
  requires_confirmation: boolean;
  reason: string;
}

export interface Trigger {
  kind: string;
  label: string;
  value?: string;
  aliases?: string[];
}

export interface Routine {
  id: string;
  name: string;
  enabled: boolean;
  trigger: Trigger;
  actions: ActionSpec[];
  created_at: string;
  updated_at: string;
}

export type ChatRole = "user" | "assistant" | "system";

export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
  created_at: string;
  tool_calls?: ActionSpec[];
  status?: "pending" | "done" | "blocked" | "error";
}

export interface LocalAiStatus {
  ready: boolean;
  provider: "qwen_embedded" | "rules" | string;
  model: string;
  state: string;
  error?: string | null;
}

export interface AppStatus {
  onboarding_complete: boolean;
  local_ai_ready: boolean;
  local_ai_provider: string;
  local_ai_model: string;
  local_ai_state: string;
  local_ai_error?: string | null;
  microphone_ready: boolean;
  spotify_connected: boolean;
  spotify_device_active: boolean;
  system_status: string;
  last_error?: string | null;
}

export interface PermissionSettings {
  apps: boolean;
  windows: boolean;
  clipboard: boolean;
  screenshots: boolean;
  processes: boolean;
  power: boolean;
  voice: boolean;
}

export interface AssistantSettings {
  assistant_name: string;
  theme: "dark" | "light" | string;
  language: "pt-BR" | string;
  start_with_windows: boolean;
  minimize_to_tray: boolean;
  operation_mode: "auto_confirm_dangerous" | string;
  local_ai_provider: "qwen_embedded" | "rules";
  local_ai_path?: string | null;
  voice_enabled: boolean;
  wake_word: string;
  speech_reply_enabled: boolean;
  permissions: PermissionSettings;
  spotify_client_id: string;
  spotify_redirect_uri: string;
  system_volume: number;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  exe?: string | null;
  cpu: number;
  memory: number;
  critical: boolean;
}

export interface WindowInfo {
  id: string;
  title: string;
  app: string;
  focused: boolean;
}

export interface InstalledApp {
  id: string;
  name: string;
  command: string;
  favorite: boolean;
  recent: boolean;
}

export interface PcControlState {
  volume: number;
  brightness: number;
  clipboard_preview: string;
  processes: ProcessInfo[];
  windows: WindowInfo[];
}

export interface CommandHistoryEntry {
  id: string;
  command: string;
  actions: ActionSpec[];
  assistant_reply: string;
  result_summary: string;
  status: string;
  source: string;
  created_at: string;
}

export interface SpotifyTrack {
  id: string;
  name: string;
  artist: string;
  album?: string;
  image_url?: string;
  uri?: string;
  is_playing?: boolean;
}

export interface SpotifyPlaylist {
  id: string;
  name: string;
  description?: string;
  image_url?: string;
  tracks_total: number;
  uri: string;
}

export interface MemoryEntry {
  id: string;
  title: string;
  content: string;
  category: string;
  pinned: boolean;
  created_at: string;
  updated_at: string;
}

export interface MusicFavorite {
  id: string;
  name: string;
  uri?: string | null;
  query?: string | null;
  kind: string;
  created_at: string;
}

export interface RuntimeCapabilities {
  pc_control: boolean;
  app_launch: boolean;
  spotify_app: boolean;
  spotify_api: boolean;
  spotify_desktop_automation: boolean;
  local_ai: boolean;
  local_voice: boolean;
  brightness_control: boolean;
  window_control: boolean;
}

export interface RuntimeStatus {
  mode: "native" | "web" | string;
  capabilities: RuntimeCapabilities;
  last_error?: string | null;
}

export interface VoiceState {
  state: "idle" | "waiting_wake_word" | "listening" | "transcribing" | "error" | string;
  wake_word: string;
  engine_ready: boolean;
  microphone_ready: boolean;
  transcript?: string | null;
  error?: string | null;
}

export interface TaskState {
  objective?: string | null;
  completed_steps: string[];
  next_step?: string | null;
  updated_at?: string | null;
}

export interface MusicCommandResult {
  app_opened: boolean;
  playback_started: boolean;
  method_used: string;
  message: string;
  limitation?: string | null;
}

export interface LogEntry {
  id: string;
  level: "info" | "warn" | "error" | string;
  message: string;
  created_at: string;
  module: string;
}

export interface ChatResult {
  message: ChatMessage;
  proposed_actions: ActionSpec[];
}

export interface GoogleProfile {
  email: string;
  name: string;
  picture?: string;
  last_verified_at: string;
}

export interface AccountStatus {
  google_connected: boolean;
  profile?: GoogleProfile | null;
}
