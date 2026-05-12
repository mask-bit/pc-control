import { invoke } from "@tauri-apps/api/core";
import type {
  ActionSpec,
  AssistantSettings,
  ChatMessage,
  ChatMode,
  ChatResult,
  IntegrationStatus,
  LogEntry,
  Routine,
  SpotifyPlaylist,
  SpotifyTrack,
} from "../types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const isTauri = () => typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);

const nowIso = () => new Date().toISOString();

const demoActions: ActionSpec[] = [
  {
    type: "open_url",
    label: "Abrir Chrome com pesquisa de foco",
    args: { url: "https://www.google.com/search?q=tecnicas+de+foco" },
    risk: "low",
    requires_confirmation: false,
  },
  {
    type: "spotify_play",
    label: "Tocar playlist de foco no Spotify",
    args: { query: "playlist foco lo-fi" },
    risk: "low",
    requires_confirmation: false,
  },
];

let browserMessages: ChatMessage[] = [
  {
    id: "welcome",
    role: "assistant",
    content:
      "Oi. Eu sou seu assistente inteligente local. Posso conversar, montar rotinas, controlar Spotify e preparar acoes seguras para o PC.",
    created_at: nowIso(),
  },
];

let browserRoutines: Routine[] = [
  {
    id: "study-mode",
    name: "Modo estudo",
    enabled: true,
    trigger: { kind: "manual", label: "Manual" },
    actions: [
      {
        type: "spotify_play",
        label: "Tocar lo-fi ambiente",
        args: { query: "lo-fi focus" },
        risk: "low",
        requires_confirmation: false,
      },
      {
        type: "spotify_volume",
        label: "Volume do Spotify em 30%",
        args: { volume: 30 },
        risk: "low",
        requires_confirmation: false,
      },
    ],
    created_at: nowIso(),
    updated_at: nowIso(),
  },
];

let browserLogs: LogEntry[] = [
  {
    id: "log-1",
    module: "app",
    level: "info",
    message: "Interface carregada em modo demonstracao sem Tauri.",
    created_at: nowIso(),
  },
];

let browserSettings: AssistantSettings = {
  model: "gpt-5",
  language: "pt-BR",
  main_hotkey: "Ctrl+Shift+A",
  start_with_windows: false,
  require_confirmation: true,
  safe_mode: true,
  spotify_client_id: "",
  spotify_redirect_uri: "http://127.0.0.1:8765/callback",
};

const demoPlaylists: SpotifyPlaylist[] = [
  {
    id: "focus",
    name: "Foco profundo",
    description: "Ambiente calmo para estudo e trabalho.",
    tracks_total: 64,
    uri: "spotify:playlist:focus",
  },
  {
    id: "jazz",
    name: "Jazz leve",
    description: "Instrumental para fim de tarde.",
    tracks_total: 38,
    uri: "spotify:playlist:jazz",
  },
];

async function command<T>(name: string, args?: Record<string, unknown>, fallback?: () => T | Promise<T>): Promise<T> {
  if (isTauri()) {
    return invoke<T>(name, args);
  }
  if (!fallback) {
    throw new Error(`Comando ${name} indisponivel fora do Tauri.`);
  }
  return fallback();
}

export const api = {
  getIntegrationStatus: () =>
    command<IntegrationStatus>("get_integration_status", undefined, () => ({
      openai_connected: false,
      spotify_connected: false,
      spotify_device_active: false,
      last_error: "Modo demonstracao: rode via Tauri para usar credenciais reais.",
    })),

  listMessages: () =>
    command<ChatMessage[]>("list_messages", undefined, () => browserMessages),

  sendMessage: (content: string, mode: ChatMode) =>
    command<ChatResult>(
      "send_chat_message",
      { content, mode },
      () => {
        const user: ChatMessage = {
          id: crypto.randomUUID(),
          role: "user",
          content,
          created_at: nowIso(),
        };
        const wantsAction = /abre|abrir|toca|spotify|rotina|modo|playlist|chrome/i.test(content);
        const proposed_actions = wantsAction ? demoActions : [];
        const assistant: ChatMessage = {
          id: crypto.randomUUID(),
          role: "assistant",
          content: wantsAction
            ? "Entendi. Montei uma proposta segura de acoes para voce revisar antes de executar."
            : "Posso te ajudar com isso. Se quiser, tambem posso transformar a resposta em uma rotina ou acao local.",
          created_at: nowIso(),
          tool_calls: proposed_actions,
          status: proposed_actions.length ? "pending" : "done",
        };
        browserMessages = [...browserMessages, user, assistant];
        return { message: assistant, proposed_actions };
      },
    ),

  executeAction: (action: ActionSpec) =>
    command<LogEntry>(
      "execute_action",
      { action },
      () => {
        const log: LogEntry = {
          id: crypto.randomUUID(),
          module: "executor",
          level: "info",
          message: `Acao simulada: ${action.label}`,
          created_at: nowIso(),
        };
        browserLogs = [log, ...browserLogs];
        return log;
      },
    ),

  listRoutines: () =>
    command<Routine[]>("list_routines", undefined, () => browserRoutines),

  saveRoutine: (routine: Routine) =>
    command<Routine>(
      "save_routine",
      { routine },
      () => {
        browserRoutines = [routine, ...browserRoutines.filter((item) => item.id !== routine.id)];
        return routine;
      },
    ),

  runRoutine: (id: string) =>
    command<LogEntry>("run_routine", { id }, () => {
      const routine = browserRoutines.find((item) => item.id === id);
      const log: LogEntry = {
        id: crypto.randomUUID(),
        module: "routines",
        level: "info",
        message: `Rotina simulada: ${routine?.name ?? id}`,
        created_at: nowIso(),
      };
      browserLogs = [log, ...browserLogs];
      return log;
    }),

  importWorkspaceConfig: () =>
    command<Routine[]>("import_workspace_config", undefined, () => browserRoutines),

  getSettings: () =>
    command<AssistantSettings>("get_settings", undefined, () => browserSettings),

  saveSettings: (settings: AssistantSettings) =>
    command<AssistantSettings>(
      "save_settings",
      { settings },
      () => {
        browserSettings = settings;
        return browserSettings;
      },
    ),

  saveOpenAiKey: (apiKey: string) =>
    command<boolean>("save_openai_key", { apiKey }, () => Boolean(apiKey.trim())),

  testOpenAi: () =>
    command<boolean>("test_openai", undefined, () => false),

  spotifyBeginAuth: (clientId: string, redirectUri: string) =>
    command<string>(
      "spotify_begin_auth",
      { clientId, redirectUri },
      () =>
        `https://accounts.spotify.com/authorize?client_id=${encodeURIComponent(
          clientId || "demo",
        )}&response_type=code&redirect_uri=${encodeURIComponent(redirectUri)}&scope=user-read-playback-state%20user-modify-playback-state%20playlist-read-private`,
    ),

  spotifyFinishAuth: (callbackUrl: string) =>
    command<boolean>("spotify_finish_auth", { callbackUrl }, () => Boolean(callbackUrl)),

  spotifyCurrent: () =>
    command<SpotifyTrack | null>("spotify_current_track", undefined, () => ({
      id: "demo-track",
      name: "Faixa demonstracao",
      artist: "Assistente",
      album: "Modo local",
      is_playing: true,
    })),

  spotifyPlaylists: () =>
    command<SpotifyPlaylist[]>("spotify_list_playlists", undefined, () => demoPlaylists),

  spotifySearch: (query: string) =>
    command<SpotifyPlaylist[]>("spotify_search_playlists", { query }, () =>
      demoPlaylists.filter((playlist) => playlist.name.toLowerCase().includes(query.toLowerCase())),
    ),

  spotifyPlay: (uri: string) =>
    command<boolean>("spotify_play_uri", { uri }, () => Boolean(uri)),

  spotifyPause: () =>
    command<boolean>("spotify_pause", undefined, () => true),

  spotifyNext: () =>
    command<boolean>("spotify_next", undefined, () => true),

  spotifyPrevious: () =>
    command<boolean>("spotify_previous", undefined, () => true),

  spotifySetVolume: (volume: number) =>
    command<boolean>("spotify_set_volume", { volume }, () => volume >= 0),

  listLogs: () =>
    command<LogEntry[]>("list_logs", undefined, () => browserLogs),
};
