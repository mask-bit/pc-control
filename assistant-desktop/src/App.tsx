import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  Clipboard,
  Clock,
  FileText,
  FolderOpen,
  Globe,
  Headphones,
  History,
  Key,
  ListMusic,
  Lock,
  MessageSquare,
  Monitor,
  Music,
  Pause,
  Play,
  Plug,
  Plus,
  RefreshCw,
  Save,
  Search,
  Send,
  Settings,
  ShieldCheck,
  SkipBack,
  SkipForward,
  Sparkles,
  Volume2,
  Workflow,
} from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { api } from "./lib/tauri";
import type {
  ActionSpec,
  AppView,
  AssistantSettings,
  ChatMessage,
  ChatMode,
  IntegrationStatus,
  LogEntry,
  Routine,
  SpotifyPlaylist,
  SpotifyTrack,
  TriggerKind,
} from "./types";

const viewItems: Array<{ id: AppView; label: string; icon: typeof Bot }> = [
  { id: "chat", label: "Inicio e chat", icon: Bot },
  { id: "spotify", label: "Spotify", icon: Music },
  { id: "routines", label: "Rotinas", icon: Workflow },
  { id: "integrations", label: "Integracoes", icon: Plug },
  { id: "settings", label: "Configuracoes", icon: Settings },
  { id: "logs", label: "Logs", icon: FileText },
];

const quickPrompts = [
  "Abre o Chrome e toca minha playlist de foco",
  "Cria um modo estudo com volume baixo e Spotify ambiente",
  "Resume esse texto e depois coloca lo-fi",
  "Quando eu abrir o Word, toca musica instrumental",
];

const defaultSettings: AssistantSettings = {
  model: "gpt-5",
  language: "pt-BR",
  main_hotkey: "Ctrl+Shift+A",
  start_with_windows: false,
  require_confirmation: true,
  safe_mode: true,
  spotify_client_id: "",
  spotify_redirect_uri: "http://127.0.0.1:8765/callback",
};

const nowIso = () => new Date().toISOString();

function makeRoutineFromActions(actions: ActionSpec[]): Routine {
  return {
    id: crypto.randomUUID(),
    name: "Rotina criada pela IA",
    enabled: true,
    trigger: { kind: "manual", label: "Manual" },
    actions,
    created_at: nowIso(),
    updated_at: nowIso(),
  };
}

function App() {
  const [view, setView] = useState<AppView>("chat");
  const [status, setStatus] = useState<IntegrationStatus>({
    openai_connected: false,
    spotify_connected: false,
    spotify_device_active: false,
  });
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [routines, setRoutines] = useState<Routine[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [settings, setSettings] = useState<AssistantSettings>(defaultSettings);
  const [loading, setLoading] = useState(true);

  async function refreshAll() {
    const [nextStatus, nextMessages, nextRoutines, nextLogs, nextSettings] = await Promise.all([
      api.getIntegrationStatus(),
      api.listMessages(),
      api.listRoutines(),
      api.listLogs(),
      api.getSettings(),
    ]);
    setStatus(nextStatus);
    setMessages(nextMessages);
    setRoutines(nextRoutines);
    setLogs(nextLogs);
    setSettings(nextSettings);
  }

  useEffect(() => {
    refreshAll()
      .catch((error) => {
        setStatus((current) => ({ ...current, last_error: String(error) }));
      })
      .finally(() => setLoading(false));
  }, []);

  const activeActions = useMemo(
    () => messages.flatMap((message) => message.tool_calls ?? []).slice(-4),
    [messages],
  );

  async function handleLog(entry: LogEntry) {
    setLogs((current) => [entry, ...current]);
    await refreshAll().catch(() => undefined);
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <Sparkles size={22} />
          </div>
          <div>
            <strong>Assistente</strong>
            <span>IA local + Spotify</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="Navegacao principal">
          {viewItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                className={view === item.id ? "nav-item active" : "nav-item"}
                onClick={() => setView(item.id)}
              >
                <Icon size={18} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="status-stack">
          <StatusPill ok={status.openai_connected} label="OpenAI" />
          <StatusPill ok={status.spotify_connected} label="Spotify" />
          <StatusPill ok={status.spotify_device_active} label="Dispositivo" />
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">Assistente inteligente de desktop</p>
            <h1>{titleForView(view)}</h1>
          </div>
          <div className="topbar-actions">
            <button className="icon-button" onClick={() => refreshAll()}>
              <RefreshCw size={17} />
              <span>Atualizar</span>
            </button>
          </div>
        </header>

        {status.last_error && <InlineNotice tone="warn" text={status.last_error} />}

        {loading ? (
          <div className="loading-panel">Carregando dados locais...</div>
        ) : (
          <>
            {view === "chat" && (
              <ChatView
                messages={messages}
                setMessages={setMessages}
                activeActions={activeActions}
                onLog={handleLog}
                onSaveRoutine={async (actions) => {
                  const saved = await api.saveRoutine(makeRoutineFromActions(actions));
                  setRoutines((current) => [saved, ...current]);
                  setView("routines");
                }}
              />
            )}
            {view === "spotify" && <SpotifyView onLog={handleLog} />}
            {view === "routines" && (
              <RoutinesView
                routines={routines}
                setRoutines={setRoutines}
                onLog={handleLog}
              />
            )}
            {view === "integrations" && (
              <IntegrationsView
                status={status}
                settings={settings}
                setSettings={setSettings}
                onRefresh={refreshAll}
              />
            )}
            {view === "settings" && (
              <SettingsView
                settings={settings}
                setSettings={setSettings}
                onSaved={refreshAll}
              />
            )}
            {view === "logs" && <LogsView logs={logs} />}
          </>
        )}
      </main>
    </div>
  );
}

function titleForView(view: AppView) {
  switch (view) {
    case "chat":
      return "Inicio e chat";
    case "spotify":
      return "Spotify";
    case "routines":
      return "Rotinas";
    case "integrations":
      return "Integracoes";
    case "settings":
      return "Configuracoes";
    case "logs":
      return "Logs";
  }
}

function StatusPill({ ok, label }: { ok: boolean; label: string }) {
  return (
    <div className={ok ? "status-pill ok" : "status-pill"}>
      {ok ? <CheckCircle2 size={15} /> : <AlertTriangle size={15} />}
      <span>{label}</span>
    </div>
  );
}

function InlineNotice({ tone, text }: { tone: "warn" | "ok"; text: string }) {
  return (
    <div className={`inline-notice ${tone}`}>
      {tone === "warn" ? <AlertTriangle size={18} /> : <CheckCircle2 size={18} />}
      <span>{text}</span>
    </div>
  );
}

function ChatView({
  messages,
  setMessages,
  activeActions,
  onLog,
  onSaveRoutine,
}: {
  messages: ChatMessage[];
  setMessages: (messages: ChatMessage[]) => void;
  activeActions: ActionSpec[];
  onLog: (entry: LogEntry) => Promise<void>;
  onSaveRoutine: (actions: ActionSpec[]) => Promise<void>;
}) {
  const [mode, setMode] = useState<ChatMode>("assistente");
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);

  async function submitMessage(event?: FormEvent) {
    event?.preventDefault();
    const content = input.trim();
    if (!content || sending) return;

    const localUser: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      content,
      created_at: nowIso(),
    };
    setMessages([...messages, localUser]);
    setInput("");
    setSending(true);

    try {
      const result = await api.sendMessage(content, mode);
      setMessages([...(await api.listMessages())]);
      if (result.proposed_actions.length) {
        await onLog({
          id: crypto.randomUUID(),
          module: "ai",
          level: "info",
          message: `IA propos ${result.proposed_actions.length} acao(oes).`,
          created_at: nowIso(),
        });
      }
    } finally {
      setSending(false);
    }
  }

  async function execute(action: ActionSpec) {
    const entry = await api.executeAction(action);
    await onLog(entry);
  }

  return (
    <section className="chat-layout">
      <div className="chat-main panel">
        <div className="mode-row" role="tablist" aria-label="Modo do assistente">
          {(["conversa", "comando", "assistente"] as ChatMode[]).map((item) => (
            <button
              key={item}
              className={mode === item ? "segmented active" : "segmented"}
              onClick={() => setMode(item)}
            >
              {item}
            </button>
          ))}
        </div>

        <div className="message-list">
          {messages.map((message) => (
            <div key={message.id} className={`message ${message.role}`}>
              <div className="message-avatar">
                {message.role === "assistant" ? <Bot size={17} /> : <MessageSquare size={17} />}
              </div>
              <div className="message-body">
                <p>{message.content}</p>
                {message.tool_calls?.length ? (
                  <div className="message-actions">
                    {message.tool_calls.map((action, index) => (
                      <ActionCard
                        key={`${action.type}-${index}`}
                        action={action}
                        onExecute={() => execute(action)}
                      />
                    ))}
                  </div>
                ) : null}
              </div>
            </div>
          ))}
        </div>

        <form className="composer" onSubmit={submitMessage}>
          <input
            value={input}
            onChange={(event) => setInput(event.target.value)}
            placeholder="Digite um pedido em portugues: abre o Chrome e toca minha playlist de foco"
          />
          <button type="submit" disabled={sending || !input.trim()}>
            <Send size={18} />
            <span>{sending ? "Enviando" : "Enviar"}</span>
          </button>
        </form>
      </div>

      <aside className="chat-side">
        <section className="panel side-panel">
          <h2>Acoes sugeridas</h2>
          {activeActions.length ? (
            <div className="stack">
              {activeActions.map((action, index) => (
                <ActionCard key={`${action.label}-${index}`} action={action} onExecute={() => execute(action)} />
              ))}
              <button className="secondary wide" onClick={() => onSaveRoutine(activeActions)}>
                <Save size={17} />
                <span>Salvar como rotina</span>
              </button>
            </div>
          ) : (
            <EmptyState icon={ShieldCheck} title="Nenhuma acao pendente" text="Quando a IA entender um comando, ela vai propor acoes seguras aqui." />
          )}
        </section>

        <section className="panel side-panel">
          <h2>Comandos rapidos</h2>
          <div className="prompt-list">
            {quickPrompts.map((prompt) => (
              <button key={prompt} onClick={() => setInput(prompt)}>
                {prompt}
              </button>
            ))}
          </div>
        </section>
      </aside>
    </section>
  );
}

function ActionCard({ action, onExecute }: { action: ActionSpec; onExecute: () => void }) {
  const Icon = iconForAction(action.type);
  return (
    <div className={`action-card risk-${action.risk}`}>
      <div className="action-icon">
        <Icon size={18} />
      </div>
      <div>
        <strong>{action.label}</strong>
        <span>{action.requires_confirmation ? "Pede confirmacao" : "Execucao direta permitida"}</span>
      </div>
      <button className="small-button" onClick={onExecute}>
        Executar
      </button>
    </div>
  );
}

function iconForAction(type: ActionSpec["type"]) {
  switch (type) {
    case "open_app":
      return Monitor;
    case "open_url":
      return Globe;
    case "open_path":
      return FolderOpen;
    case "copy_text":
      return Clipboard;
    case "spotify_play":
    case "spotify_pause":
    case "spotify_next":
    case "spotify_previous":
    case "spotify_volume":
      return Music;
    case "run_routine":
      return Workflow;
  }
}

function SpotifyView({ onLog }: { onLog: (entry: LogEntry) => Promise<void> }) {
  const [current, setCurrent] = useState<SpotifyTrack | null>(null);
  const [playlists, setPlaylists] = useState<SpotifyPlaylist[]>([]);
  const [query, setQuery] = useState("");
  const [volume, setVolume] = useState(35);

  async function refreshSpotify() {
    const [track, items] = await Promise.all([api.spotifyCurrent(), api.spotifyPlaylists()]);
    setCurrent(track);
    setPlaylists(items);
  }

  useEffect(() => {
    refreshSpotify().catch(() => undefined);
  }, []);

  async function play(uri: string) {
    await api.spotifyPlay(uri);
    await onLog({
      id: crypto.randomUUID(),
      module: "spotify",
      level: "info",
      message: `Spotify recebeu play para ${uri}.`,
      created_at: nowIso(),
    });
    await refreshSpotify();
  }

  async function search(event: FormEvent) {
    event.preventDefault();
    const results = query.trim() ? await api.spotifySearch(query.trim()) : await api.spotifyPlaylists();
    setPlaylists(results);
  }

  return (
    <section className="spotify-layout">
      <div className="panel player-panel">
        <div className="album-art">
          {current?.image_url ? <img src={current.image_url} alt="" /> : <Headphones size={64} />}
        </div>
        <div className="track-copy">
          <p className="eyebrow">Tocando agora</p>
          <h2>{current?.name ?? "Nenhuma musica ativa"}</h2>
          <span>{current?.artist ?? "Conecte o Spotify para controlar a reproducao"}</span>
        </div>
        <div className="player-controls">
          <button className="round-button" onClick={() => api.spotifyPrevious()}>
            <SkipBack size={19} />
          </button>
          <button className="round-button primary" onClick={() => (current?.is_playing ? api.spotifyPause() : current?.uri && play(current.uri))}>
            {current?.is_playing ? <Pause size={22} /> : <Play size={22} />}
          </button>
          <button className="round-button" onClick={() => api.spotifyNext()}>
            <SkipForward size={19} />
          </button>
        </div>
        <label className="slider-row">
          <Volume2 size={18} />
          <input
            type="range"
            min="0"
            max="100"
            value={volume}
            onChange={(event) => {
              const value = Number(event.target.value);
              setVolume(value);
              api.spotifySetVolume(value).catch(() => undefined);
            }}
          />
          <span>{volume}%</span>
        </label>
      </div>

      <div className="panel library-panel">
        <div className="section-header">
          <div>
            <p className="eyebrow">Biblioteca</p>
            <h2>Playlists e busca</h2>
          </div>
          <button className="secondary" onClick={refreshSpotify}>
            <RefreshCw size={16} />
            <span>Sincronizar</span>
          </button>
        </div>
        <form className="search-row" onSubmit={search}>
          <Search size={18} />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Buscar playlist, album, artista ou faixa" />
          <button type="submit">Buscar</button>
        </form>
        <div className="playlist-grid">
          {playlists.map((playlist) => (
            <button key={playlist.id} className="playlist-item" onClick={() => play(playlist.uri)}>
              <div className="playlist-art">{playlist.image_url ? <img src={playlist.image_url} alt="" /> : <ListMusic size={24} />}</div>
              <strong>{playlist.name}</strong>
              <span>{playlist.tracks_total} faixas</span>
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}

function RoutinesView({
  routines,
  setRoutines,
  onLog,
}: {
  routines: Routine[];
  setRoutines: (routines: Routine[]) => void;
  onLog: (entry: LogEntry) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [triggerKind, setTriggerKind] = useState<TriggerKind>("manual");
  const [triggerValue, setTriggerValue] = useState("");

  async function createRoutine(event: FormEvent) {
    event.preventDefault();
    const routine: Routine = {
      id: crypto.randomUUID(),
      name: name.trim() || "Nova rotina",
      enabled: true,
      trigger: {
        kind: triggerKind,
        label: labelForTrigger(triggerKind, triggerValue),
        value: triggerValue.trim() || undefined,
      },
      actions: [],
      created_at: nowIso(),
      updated_at: nowIso(),
    };
    const saved = await api.saveRoutine(routine);
    setRoutines([saved, ...routines]);
    setName("");
    setTriggerValue("");
  }

  async function runRoutine(id: string) {
    const log = await api.runRoutine(id);
    await onLog(log);
  }

  async function importOld() {
    const imported = await api.importWorkspaceConfig();
    setRoutines(imported);
    await onLog({
      id: crypto.randomUUID(),
      module: "migration",
      level: "info",
      message: "Perfis do launcher importados como rotinas.",
      created_at: nowIso(),
    });
  }

  return (
    <section className="two-column">
      <div className="panel">
        <div className="section-header">
          <div>
            <p className="eyebrow">Automacoes</p>
            <h2>Rotinas salvas</h2>
          </div>
          <button className="secondary" onClick={importOld}>
            <Workflow size={17} />
            <span>Importar launcher</span>
          </button>
        </div>
        <div className="routine-list">
          {routines.map((routine) => (
            <article key={routine.id} className="routine-item">
              <div>
                <strong>{routine.name}</strong>
                <span>{routine.trigger.label} · {routine.actions.length} acoes</span>
              </div>
              <div className="row-actions">
                <span className={routine.enabled ? "badge enabled" : "badge"}>{routine.enabled ? "Ativa" : "Pausada"}</span>
                <button className="small-button" onClick={() => runRoutine(routine.id)}>
                  Executar
                </button>
              </div>
            </article>
          ))}
        </div>
      </div>

      <form className="panel form-panel" onSubmit={createRoutine}>
        <h2>Criar rotina</h2>
        <label>
          Nome
          <input value={name} onChange={(event) => setName(event.target.value)} placeholder="Modo estudo" />
        </label>
        <label>
          Gatilho
          <select value={triggerKind} onChange={(event) => setTriggerKind(event.target.value as TriggerKind)}>
            <option value="manual">Manual</option>
            <option value="startup">Ao iniciar o Windows</option>
            <option value="schedule">Horario</option>
            <option value="app_open">Ao abrir aplicativo</option>
          </select>
        </label>
        <label>
          Valor do gatilho
          <input value={triggerValue} onChange={(event) => setTriggerValue(event.target.value)} placeholder="08:00 ou WINWORD.EXE" />
        </label>
        <button className="primary-button" type="submit">
          <Plus size={18} />
          <span>Criar rotina</span>
        </button>
      </form>
    </section>
  );
}

function labelForTrigger(kind: TriggerKind, value: string) {
  if (kind === "startup") return "Ao iniciar o Windows";
  if (kind === "schedule") return `Horario ${value || "nao definido"}`;
  if (kind === "app_open") return `Ao abrir ${value || "app"}`;
  return "Manual";
}

function IntegrationsView({
  status,
  settings,
  setSettings,
  onRefresh,
}: {
  status: IntegrationStatus;
  settings: AssistantSettings;
  setSettings: (settings: AssistantSettings) => void;
  onRefresh: () => Promise<void>;
}) {
  const [apiKey, setApiKey] = useState("");
  const [callbackUrl, setCallbackUrl] = useState("");
  const [authUrl, setAuthUrl] = useState("");
  const [notice, setNotice] = useState("");

  async function saveOpenAi(event: FormEvent) {
    event.preventDefault();
    await api.saveOpenAiKey(apiKey);
    setApiKey("");
    setNotice("Chave OpenAI salva no armazenamento seguro local.");
    await onRefresh();
  }

  async function startSpotify() {
    const saved = await api.saveSettings(settings);
    setSettings(saved);
    const url = await api.spotifyBeginAuth(settings.spotify_client_id, settings.spotify_redirect_uri);
    setAuthUrl(url);
    window.open(url, "_blank", "noopener,noreferrer");
  }

  async function finishSpotify(event: FormEvent) {
    event.preventDefault();
    await api.spotifyFinishAuth(callbackUrl);
    setCallbackUrl("");
    setNotice("Spotify conectado com OAuth PKCE.");
    await onRefresh();
  }

  return (
    <section className="two-column">
      <div className="panel form-panel">
        <div className="section-header compact">
          <div>
            <p className="eyebrow">OpenAI</p>
            <h2>Chave da API</h2>
          </div>
          <StatusPill ok={status.openai_connected} label={status.openai_connected ? "Conectada" : "Desconectada"} />
        </div>
        <form onSubmit={saveOpenAi} className="stack">
          <label>
            OpenAI API key
            <input
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
              placeholder="sk-..."
            />
          </label>
          <button className="primary-button" type="submit">
            <Key size={18} />
            <span>Salvar chave segura</span>
          </button>
          <button className="secondary wide" type="button" onClick={() => api.testOpenAi().then(() => onRefresh())}>
            <ShieldCheck size={17} />
            <span>Testar conexao</span>
          </button>
        </form>
      </div>

      <div className="panel form-panel">
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Spotify</p>
            <h2>OAuth PKCE</h2>
          </div>
          <StatusPill ok={status.spotify_connected} label={status.spotify_connected ? "Conectado" : "Desconectado"} />
        </div>
        <label>
          Client ID
          <input
            value={settings.spotify_client_id}
            onChange={(event) => setSettings({ ...settings, spotify_client_id: event.target.value })}
            placeholder="Client ID do app Spotify"
          />
        </label>
        <label>
          Redirect URI
          <input
            value={settings.spotify_redirect_uri}
            onChange={(event) => setSettings({ ...settings, spotify_redirect_uri: event.target.value })}
          />
        </label>
        <button className="primary-button" type="button" onClick={startSpotify}>
          <Plug size={18} />
          <span>Conectar Spotify</span>
        </button>
        {authUrl && (
          <div className="auth-box">
            <strong>URL de autorizacao</strong>
            <span>{authUrl}</span>
          </div>
        )}
        <form onSubmit={finishSpotify} className="stack">
          <label>
            URL de retorno
            <input
              value={callbackUrl}
              onChange={(event) => setCallbackUrl(event.target.value)}
              placeholder="Cole aqui a URL final depois do login"
            />
          </label>
          <button className="secondary wide" type="submit">
            Finalizar login
          </button>
        </form>
        {notice && <InlineNotice tone="ok" text={notice} />}
      </div>
    </section>
  );
}

function SettingsView({
  settings,
  setSettings,
  onSaved,
}: {
  settings: AssistantSettings;
  setSettings: (settings: AssistantSettings) => void;
  onSaved: () => Promise<void>;
}) {
  async function save(event: FormEvent) {
    event.preventDefault();
    const saved = await api.saveSettings(settings);
    setSettings(saved);
    await onSaved();
  }

  return (
    <form className="panel settings-grid" onSubmit={save}>
      <label>
        Modelo OpenAI
        <input value={settings.model} onChange={(event) => setSettings({ ...settings, model: event.target.value })} />
      </label>
      <label>
        Hotkey principal
        <input value={settings.main_hotkey} onChange={(event) => setSettings({ ...settings, main_hotkey: event.target.value })} />
      </label>
      <label className="toggle-line">
        <input
          type="checkbox"
          checked={settings.start_with_windows}
          onChange={(event) => setSettings({ ...settings, start_with_windows: event.target.checked })}
        />
        Iniciar com Windows
      </label>
      <label className="toggle-line">
        <input
          type="checkbox"
          checked={settings.require_confirmation}
          onChange={(event) => setSettings({ ...settings, require_confirmation: event.target.checked })}
        />
        Confirmar acoes sensiveis
      </label>
      <label className="toggle-line">
        <input
          type="checkbox"
          checked={settings.safe_mode}
          onChange={(event) => setSettings({ ...settings, safe_mode: event.target.checked })}
        />
        Modo seguro sem shell livre
      </label>
      <button className="primary-button settings-save" type="submit">
        <Save size={18} />
        <span>Salvar configuracoes</span>
      </button>
    </form>
  );
}

function LogsView({ logs }: { logs: LogEntry[] }) {
  return (
    <section className="panel">
      <div className="section-header">
        <div>
          <p className="eyebrow">Auditoria local</p>
          <h2>Historico de execucao</h2>
        </div>
        <History size={22} />
      </div>
      <div className="log-list">
        {logs.map((log) => (
          <article key={log.id} className={`log-item ${log.level}`}>
            <Clock size={17} />
            <div>
              <strong>{log.module}</strong>
              <span>{log.message}</span>
            </div>
            <time>{new Date(log.created_at).toLocaleString("pt-BR")}</time>
          </article>
        ))}
      </div>
    </section>
  );
}

function EmptyState({
  icon: Icon,
  title,
  text,
}: {
  icon: typeof Bot;
  title: string;
  text: string;
}) {
  return (
    <div className="empty-state">
      <Icon size={28} />
      <strong>{title}</strong>
      <span>{text}</span>
    </div>
  );
}

export default App;
