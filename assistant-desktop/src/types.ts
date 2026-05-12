export type AppView =
  | "chat"
  | "spotify"
  | "routines"
  | "integrations"
  | "settings"
  | "logs";

export type ChatMode = "conversa" | "comando" | "assistente";

export type ActionRisk = "low" | "medium" | "high";

export type ActionType =
  | "open_app"
  | "open_url"
  | "open_path"
  | "copy_text"
  | "spotify_play"
  | "spotify_pause"
  | "spotify_next"
  | "spotify_previous"
  | "spotify_volume"
  | "run_routine";

export interface ActionSpec {
  type: ActionType;
  label: string;
  args: Record<string, string | number | boolean | null>;
  risk: ActionRisk;
  requires_confirmation: boolean;
}

export type TriggerKind = "manual" | "startup" | "schedule" | "app_open";

export interface Trigger {
  kind: TriggerKind;
  label: string;
  value?: string;
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
  status?: "pending" | "done" | "error";
}

export interface IntegrationStatus {
  openai_connected: boolean;
  spotify_connected: boolean;
  spotify_device_active: boolean;
  last_error?: string;
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

export interface LogEntry {
  id: string;
  level: "info" | "warn" | "error";
  message: string;
  created_at: string;
  module: string;
}

export interface AssistantSettings {
  model: string;
  language: "pt-BR";
  main_hotkey: string;
  start_with_windows: boolean;
  require_confirmation: boolean;
  safe_mode: boolean;
  spotify_client_id: string;
  spotify_redirect_uri: string;
}

export interface ChatResult {
  message: ChatMessage;
  proposed_actions: ActionSpec[];
}
