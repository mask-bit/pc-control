import { invoke } from "@tauri-apps/api/core";
import type {
  AccountStatus,
  ActionSpec,
  AppStatus,
  AssistantSettings,
  ChatMessage,
  ChatMode,
  ChatResult,
  CommandHistoryEntry,
  GoogleProfile,
  InstalledApp,
  LocalAiStatus,
  LogEntry,
  MemoryEntry,
  MusicCommandResult,
  MusicFavorite,
  PcControlState,
  ProcessInfo,
  RuntimeStatus,
  Routine,
  SpotifyPlaylist,
  SpotifyTrack,
  TaskState,
  VoiceState,
  WindowInfo,
} from "../types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const isTauri = () => typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
const nowIso = () => new Date().toISOString();

const defaultSettings: AssistantSettings = {
  assistant_name: "PC Control AI",
  theme: "dark",
  language: "pt-BR",
  start_with_windows: false,
  minimize_to_tray: true,
  operation_mode: "auto_confirm_dangerous",
  local_ai_provider: "qwen_embedded",
  local_ai_path: null,
  voice_enabled: false,
  wake_word: "assistente",
  speech_reply_enabled: false,
  permissions: {
    apps: true,
    windows: true,
    clipboard: true,
    screenshots: true,
    processes: true,
    power: true,
    voice: true,
  },
  spotify_client_id: "",
  spotify_redirect_uri: "http://127.0.0.1:8765/spotify/callback",
  system_volume: 50,
};

let browserSettings: AssistantSettings = { ...defaultSettings };
let browserGoogleProfile: GoogleProfile | undefined;

let browserMessages: ChatMessage[] = [
  {
    id: "welcome",
    role: "assistant",
    content:
      "Esta pagina no navegador serve apenas para visualizar a interface. Para controlar o PC de verdade, abra o aplicativo instalado.",
    created_at: nowIso(),
    status: "done",
  },
];

let browserLogs: LogEntry[] = [
  {
    id: "log-1",
    module: "app",
    level: "warn",
    message: "Interface aberta fora do app nativo; controle do PC indisponivel.",
    created_at: nowIso(),
  },
];

let browserHistory: CommandHistoryEntry[] = [];
let browserMemories: MemoryEntry[] = [];
let browserMusicFavorites: MusicFavorite[] = [];
let browserTaskState: TaskState = { completed_steps: [] };

const appCatalog: InstalledApp[] = [];
const browserProcesses: ProcessInfo[] = [];

let browserRoutines: Routine[] = [];

async function command<T>(name: string, args?: Record<string, unknown>, fallback?: () => T | Promise<T>): Promise<T> {
  if (isTauri()) {
    return invoke<T>(name, args);
  }
  if (!fallback) {
    throw new Error(`Comando ${name} indisponivel fora do Tauri.`);
  }
  return fallback();
}

function pushLog(module: string, message: string, level: LogEntry["level"] = "info") {
  const log: LogEntry = { id: crypto.randomUUID(), module, level, message, created_at: nowIso() };
  browserLogs = [log, ...browserLogs];
  browserHistory = [
    {
      id: log.id,
      command: message,
      actions: [],
      assistant_reply: "",
      result_summary: message,
      status: level,
      source: module,
      created_at: log.created_at,
    },
    ...browserHistory,
  ];
  return log;
}

function ruleActions(content: string): ActionSpec[] {
  const lower = content.toLowerCase();
  const actions: ActionSpec[] = [];
  if (/desliga|desligar|reinicia|reiniciar/.test(lower)) {
    actions.push({
      type: "power",
      label: lower.includes("reinic") ? "Reiniciar o PC" : "Desligar o PC",
      target: lower.includes("reinic") ? "restart" : "shutdown",
      args: { operation: lower.includes("reinic") ? "restart" : "shutdown" },
      risk: "high",
      requires_confirmation: true,
      reason: "Acao de energia precisa de confirmacao unica.",
    });
  }
  if (/chrome|edge|notepad|bloco de notas|calculadora|spotify|vscode|vs code/.test(lower) && /abre|abrir|inicia/.test(lower)) {
    const app = lower.includes("edge")
      ? "msedge"
      : lower.includes("notepad") || lower.includes("bloco")
        ? "notepad"
        : lower.includes("calculadora")
          ? "calc"
          : lower.includes("spotify")
            ? "spotify"
            : lower.includes("code")
              ? "code"
              : "chrome";
    actions.push({
      type: "open_app",
      label: `Abrir ${app}`,
      target: app,
      args: { app },
      risk: "low",
      requires_confirmation: false,
      reason: "Abrir aplicativo.",
    });
  }
  if (/copie|copiar|copia/.test(lower)) {
    actions.push({
      type: "copy_text",
      label: "Copiar texto",
      target: "clipboard",
      args: { text: content.replace(/copie|copiar|copia/gi, "").trim() || content },
      risk: "low",
      requires_confirmation: false,
      reason: "Copiar para area de transferencia.",
    });
  }
  if (/lembre que|lembra que|memoriza|salv[ae] na memoria|guarde na memoria/.test(lower)) {
    const memoryContent = contentAfter(content, [
      "lembre que",
      "lembra que",
      "memoriza",
      "salve na memoria",
      "salva na memoria",
      "guarde na memoria",
    ]);
    actions.push({
      type: "save_memory",
      label: "Salvar memoria",
      target: "memory",
      args: {
        title: memoryContent.split(/\s+/).slice(0, 7).join(" ") || "Memoria",
        content: memoryContent,
        category: "preferencia",
      },
      risk: "low",
      requires_confirmation: false,
      reason: "Guardar informacao para a IA usar depois.",
    });
  }
  if (/curtidas|musicas favoritas|minhas favoritas|liked songs/.test(lower)) {
    actions.push({
      type: "spotify_liked",
      label: "Tocar musicas curtidas",
      target: "spotify",
      args: {},
      risk: "low",
      requires_confirmation: false,
      reason: "Tocar musicas salvas no Spotify.",
    });
  } else if (/musica|playlist|toca|toque/.test(lower) || (lower.includes("spotify") && /tocar|toca|toque/.test(lower))) {
    const query = contentAfter(content, ["toca", "toque", "playlist", "spotify"]) || content;
    actions.push({
      type: "spotify_play",
      label: "Tocar no Spotify",
      target: "spotify",
      args: { query },
      risk: "low",
      requires_confirmation: false,
      reason: "Buscar e tocar musica/playlist.",
    });
  }
  if (/print|screenshot|captura/.test(lower)) {
    actions.push({
      type: "take_screenshot",
      label: "Tirar screenshot",
      target: "screen",
      args: {},
      risk: "low",
      requires_confirmation: false,
      reason: "Captura de tela.",
    });
  }
  return actions;
}

function contentAfter(content: string, needles: string[]) {
  const lower = content.toLowerCase();
  for (const needle of needles) {
    const index = lower.indexOf(needle);
    if (index >= 0) {
      return content
        .slice(index + needle.length)
        .trim()
        .replace(/^[:\-\s"']+/, "");
    }
  }
  return "";
}

export const api = {
  getRuntimeStatus: () =>
    command<RuntimeStatus>("get_runtime_status", undefined, () => ({
      mode: "web",
      capabilities: {
        pc_control: false,
        app_launch: false,
        spotify_app: false,
        spotify_api: false,
        spotify_desktop_automation: false,
        local_ai: false,
        local_voice: false,
        brightness_control: false,
        window_control: false,
      },
      last_error: "Controle real indisponivel fora do app nativo.",
    })),

  getAppStatus: () =>
    command<AppStatus>("get_app_status", undefined, () => ({
      onboarding_complete: localStorage.getItem("pc-control-onboarding") === "done",
      local_ai_ready: false,
      local_ai_provider: browserSettings.local_ai_provider,
      local_ai_model: "Qwen3-8B-Q5_0 embutido",
      local_ai_state: "unavailable_web",
      local_ai_error: "IA local indisponivel fora do app nativo.",
      microphone_ready: false,
      spotify_connected: false,
      spotify_device_active: false,
      system_status: "web",
      last_error: "Controle real indisponivel fora do app nativo.",
    })),

  completeOnboarding: (settings: AssistantSettings) =>
    command<AssistantSettings>("complete_onboarding", { settings }, () => {
      browserSettings = settings;
      localStorage.setItem("pc-control-onboarding", "done");
      pushLog("onboarding", "Primeiro acesso concluido.");
      return browserSettings;
    }),

  localAiStatus: () =>
    command<LocalAiStatus>("local_ai_status", undefined, () => ({
      ready: false,
      provider: browserSettings.local_ai_provider,
      model: "Qwen3-8B-Q5_0 embutido",
      state: "unavailable_web",
      error: "IA local indisponivel fora do app nativo.",
    })),

  setLocalAiPath: (path: string) =>
    command<LocalAiStatus>("set_local_ai_path", { path }, () => {
      browserSettings.local_ai_path = path || null;
      browserSettings.local_ai_provider = "qwen_embedded";
      return {
        ready: false,
        provider: browserSettings.local_ai_provider,
        model: "Qwen3-8B-Q5_0 embutido",
        state: "unavailable_web",
        error: "IA local indisponivel fora do app nativo.",
      };
    }),

  listMessages: () => command<ChatMessage[]>("list_messages", undefined, () => browserMessages),

  sendMessage: (content: string, mode: ChatMode) =>
    command<ChatResult>("send_chat_message", { content, mode }, () => {
      const user: ChatMessage = {
        id: crypto.randomUUID(),
        role: "user",
        content,
        created_at: nowIso(),
        status: "done",
      };
      const assistant: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        content:
          "Nao consigo controlar o PC nesta pagina do navegador. Abra o aplicativo instalado para executar comandos reais.",
        created_at: nowIso(),
        status: "blocked",
      };
      browserMessages = [...browserMessages, user, assistant];
      pushLog("runtime", "Comando recusado fora do app nativo.", "warn");
      return { message: assistant, proposed_actions: [] };
    }),

  executeAction: (action: ActionSpec) =>
    command<LogEntry>("execute_action", { action }, () => {
      throw new Error("Controle real indisponivel fora do app nativo.");
    }),

  listRoutines: () => command<Routine[]>("list_routines", undefined, () => browserRoutines),

  saveRoutine: (routine: Routine) =>
    command<Routine>("save_routine", { routine }, () => {
      browserRoutines = [routine, ...browserRoutines.filter((item) => item.id !== routine.id)];
      return routine;
    }),

  runRoutine: (id: string) =>
    command<LogEntry>("run_routine", { id }, () => {
      throw new Error(`Rotina ${id} indisponivel fora do app nativo.`);
    }),

  listMemories: () => command<MemoryEntry[]>("list_memories", undefined, () => browserMemories),

  saveMemory: (title: string, content: string, category: string, pinned: boolean) =>
    command<MemoryEntry>("save_memory", { title, content, category, pinned }, () => {
      const memory: MemoryEntry = {
        id: crypto.randomUUID(),
        title: title || content.split(/\s+/).slice(0, 7).join(" ") || "Memoria",
        content,
        category: category || "geral",
        pinned,
        created_at: nowIso(),
        updated_at: nowIso(),
      };
      browserMemories = [memory, ...browserMemories];
      pushLog("memory", `Memoria salva: ${memory.title}`);
      return memory;
    }),

  deleteMemory: (id: string) =>
    command<boolean>("delete_memory", { id }, () => {
      browserMemories = browserMemories.filter((memory) => memory.id !== id);
      pushLog("memory", "Memoria removida.", "warn");
      return true;
    }),

  getSettings: () => command<AssistantSettings>("get_settings", undefined, () => browserSettings),

  saveSettings: (settings: AssistantSettings) =>
    command<AssistantSettings>("save_settings", { settings }, () => {
      browserSettings = settings;
      pushLog("settings", "Configuracoes salvas.");
      return browserSettings;
    }),

  getPcControlState: () =>
    command<PcControlState>("get_pc_control_state", undefined, () => ({
      volume: 0,
      brightness: 0,
      clipboard_preview: "",
      processes: browserProcesses,
      windows: [],
    })),

  listProcesses: () => command<ProcessInfo[]>("list_processes", undefined, () => browserProcesses),

  closeProcess: (pid: number) =>
    command<boolean>("close_process", { pid }, () => {
      throw new Error(`Nao e possivel encerrar o processo ${pid} fora do app nativo.`);
    }),

  getVolume: () =>
    command<number>("get_volume", undefined, () => {
      throw new Error("Volume real indisponivel fora do app nativo.");
    }),

  setVolume: (level: number) =>
    command<number>("set_volume", { level }, () => {
      throw new Error(`Nao e possivel ajustar o volume para ${level}% fora do app nativo.`);
    }),

  getBrightness: () =>
    command<number>("get_brightness", undefined, () => {
      throw new Error("Brilho real indisponivel fora do app nativo.");
    }),

  setBrightness: (level: number) =>
    command<number>("set_brightness", { level }, () => {
      throw new Error(`Nao e possivel ajustar o brilho para ${level}% fora do app nativo.`);
    }),

  takeScreenshot: () =>
    command<string>("take_screenshot", undefined, () => {
      throw new Error("Screenshot indisponivel fora do app nativo.");
    }),

  listInstalledApps: () => command<InstalledApp[]>("list_installed_apps", undefined, () => appCatalog),

  openApp: (app: string) =>
    command<boolean>("open_app", { app }, () => {
      throw new Error(`Nao e possivel abrir ${app} fora do app nativo.`);
    }),

  closeApp: (app: string) =>
    command<boolean>("close_app", { app }, () => {
      throw new Error(`Nao e possivel fechar ${app} fora do app nativo.`);
    }),

  listWindows: () => command<WindowInfo[]>("list_windows", undefined, () => []),

  focusWindow: (id: string) =>
    command<boolean>("focus_window", { id }, () => {
      throw new Error(`Nao e possivel focar a janela ${id} fora do app nativo.`);
    }),

  minimizeWindow: (id: string) =>
    command<boolean>("minimize_window", { id }, () => {
      throw new Error(`Nao e possivel minimizar a janela ${id} fora do app nativo.`);
    }),

  maximizeWindow: (id: string) =>
    command<boolean>("maximize_window", { id }, () => {
      throw new Error(`Nao e possivel maximizar a janela ${id} fora do app nativo.`);
    }),

  getVoiceState: () =>
    command<VoiceState>("get_voice_state", undefined, () => ({
      state: "error",
      wake_word: browserSettings.wake_word,
      engine_ready: false,
      microphone_ready: false,
      transcript: null,
      error: "Escuta local indisponivel fora do app nativo.",
    })),

  startListening: () =>
    command<VoiceState>("start_listening", undefined, () => {
      throw new Error("Escuta local indisponivel fora do app nativo.");
    }),

  stopListening: () =>
    command<VoiceState>("stop_listening", undefined, () => ({
      state: "idle",
      wake_word: browserSettings.wake_word,
      engine_ready: false,
      microphone_ready: false,
      transcript: null,
      error: "Escuta local indisponivel fora do app nativo.",
    })),

  testMicrophone: () =>
    command<{ available: boolean; level: number; message: string }>("test_microphone", undefined, () => ({
      available: false,
      level: 0,
      message: "Teste de microfone indisponivel fora do app nativo.",
    })),

  listHistory: () => command<CommandHistoryEntry[]>("list_history", undefined, () => browserHistory),

  clearHistory: () =>
    command<boolean>("clear_history", undefined, () => {
      browserHistory = [];
      browserLogs = [];
      pushLog("history", "Historico limpo.", "warn");
      return true;
    }),

  getTaskState: () => command<TaskState>("get_task_state", undefined, () => browserTaskState),

  clearTaskState: () =>
    command<boolean>("clear_task_state", undefined, () => {
      browserTaskState = { completed_steps: [] };
      return true;
    }),

  googleLogin: () =>
    command<GoogleProfile>("google_login", undefined, () => {
      throw new Error("Login Google indisponivel fora do app nativo.");
    }),

  googleLogout: () =>
    command<boolean>("google_logout", undefined, () => {
      browserGoogleProfile = undefined;
      return true;
    }),

  getAccountStatus: () =>
    command<AccountStatus>("get_account_status", undefined, () => ({
      google_connected: Boolean(browserGoogleProfile),
      profile: browserGoogleProfile,
    })),

  spotifyBeginAuth: (clientId: string, redirectUri: string) =>
    command<string>(
      "spotify_begin_auth",
      { clientId, redirectUri },
      () => {
        throw new Error("Spotify API indisponivel fora do app nativo.");
      },
    ),

  spotifyFinishAuth: (callbackUrl: string) =>
    command<boolean>("spotify_finish_auth", { callbackUrl }, () => Boolean(callbackUrl)),

  spotifyCurrent: () =>
    command<SpotifyTrack | null>("spotify_current_track", undefined, () => null),

  spotifyPlaylists: () => command<SpotifyPlaylist[]>("spotify_list_playlists", undefined, () => []),

  spotifySearch: (query: string) =>
    command<SpotifyPlaylist[]>("spotify_search_playlists", { query }, () => []),

  spotifyPlay: (uri: string) =>
    command<boolean>("spotify_play_uri", { uri }, () => {
      throw new Error("Spotify indisponivel fora do app nativo.");
    }),

  spotifyPlayLiked: () =>
    command<boolean>("spotify_play_liked_tracks", undefined, () => {
      throw new Error("Spotify indisponivel fora do app nativo.");
    }),

  spotifyPause: () =>
    command<boolean>("spotify_pause", undefined, () => {
      throw new Error("Spotify indisponivel fora do app nativo.");
    }),

  spotifyNext: () =>
    command<boolean>("spotify_next", undefined, () => {
      throw new Error("Spotify indisponivel fora do app nativo.");
    }),

  spotifyPrevious: () =>
    command<boolean>("spotify_previous", undefined, () => {
      throw new Error("Spotify indisponivel fora do app nativo.");
    }),

  spotifySetVolume: (volume: number) =>
    command<boolean>("spotify_set_volume", { volume }, () => {
      throw new Error("Spotify indisponivel fora do app nativo.");
    }),

  playMusic: (query: string) =>
    command<MusicCommandResult>("play_music", { query }, () => {
      throw new Error("Musica indisponivel fora do app nativo.");
    }),

  playLikedMusic: () =>
    command<MusicCommandResult>("play_liked_music", undefined, () => {
      throw new Error("Musica indisponivel fora do app nativo.");
    }),

  pauseMusic: () =>
    command<MusicCommandResult>("pause_music", undefined, () => {
      throw new Error("Musica indisponivel fora do app nativo.");
    }),

  nextTrack: () =>
    command<MusicCommandResult>("next_track", undefined, () => {
      throw new Error("Musica indisponivel fora do app nativo.");
    }),

  previousTrack: () =>
    command<MusicCommandResult>("previous_track", undefined, () => {
      throw new Error("Musica indisponivel fora do app nativo.");
    }),

  listMusicFavorites: () =>
    command<MusicFavorite[]>("list_music_favorites", undefined, () => browserMusicFavorites),

  saveMusicFavorite: (name: string, uri?: string | null, query?: string | null, kind = "playlist") =>
    command<MusicFavorite>("save_music_favorite", { name, uri, query, kind }, () => {
      const favorite: MusicFavorite = {
        id: crypto.randomUUID(),
        name,
        uri,
        query,
        kind,
        created_at: nowIso(),
      };
      browserMusicFavorites = [favorite, ...browserMusicFavorites];
      pushLog("spotify", `Favorito musical salvo: ${favorite.name}`);
      return favorite;
    }),

  deleteMusicFavorite: (id: string) =>
    command<boolean>("delete_music_favorite", { id }, () => {
      browserMusicFavorites = browserMusicFavorites.filter((favorite) => favorite.id !== id);
      pushLog("spotify", "Favorito musical removido.", "warn");
      return true;
    }),

  listLogs: () => command<LogEntry[]>("list_logs", undefined, () => browserLogs),
};

export { defaultSettings };
