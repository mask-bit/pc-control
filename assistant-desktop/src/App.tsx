import {
  Activity,
  AppWindow,
  Bot,
  Camera,
  CheckCircle2,
  Clock,
  Cpu,
  Database,
  Gauge,
  Headphones,
  History,
  Home,
  Layers,
  ListChecks,
  LogIn,
  LogOut,
  Mic,
  MicOff,
  Monitor,
  Music,
  Pause,
  Play,
  Radar,
  RefreshCw,
  Save,
  Search,
  Send,
  Settings,
  ShieldCheck,
  SkipBack,
  SkipForward,
  Sparkles,
  Star,
  Trash2,
  UserCircle,
  Volume2,
  Workflow,
  XCircle,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import LottieImport from "lottie-react";
import * as THREE from "three";
import {
  type ButtonHTMLAttributes,
  type CSSProperties,
  type FormEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import neuralPulse from "./animations/neural-pulse.json";
import { api, defaultSettings } from "./lib/tauri";
import type {
  AccountStatus,
  ActionSpec,
  AppStatus,
  AppView,
  AssistantSettings,
  ChatMessage,
  CommandHistoryEntry,
  InstalledApp,
  LogEntry,
  MemoryEntry,
  MusicFavorite,
  PcControlState,
  ProcessInfo,
  Routine,
  RuntimeStatus,
  SpotifyPlaylist,
  SpotifyTrack,
  TaskState,
  VoiceState,
} from "./types";
import "./styles.css";

const navItems: Array<{ view: AppView; label: string; signal: string; icon: LucideIcon }> = [
  { view: "home", label: "Nucleo", signal: "AI-00", icon: Home },
  { view: "pc_control", label: "Comando", signal: "OPS-12", icon: Monitor },
  { view: "apps", label: "Apps", signal: "APP-31", icon: AppWindow },
  { view: "music", label: "Musica", signal: "MUS-22", icon: Music },
  { view: "voice", label: "Voz", signal: "VOX-08", icon: Mic },
  { view: "automations", label: "Rotinas", signal: "AUTO-19", icon: Workflow },
  { view: "history", label: "Memoria", signal: "MEM-27", icon: History },
  { view: "settings", label: "Sistema", signal: "SYS-01", icon: Settings },
];

const promptSuggestions = [
  "abre o Chrome",
  "toca minhas curtidas",
  "copie este texto para a area de transferencia",
  "tire um print da tela",
  "coloque o volume em 40%",
  "desliga o PC",
];

const viewTitles: Record<AppView, string> = {
  home: "Neural Core",
  pc_control: "Operational Command",
  apps: "Application Matrix",
  music: "Sonic Reactor",
  voice: "Voice Interface",
  automations: "Routine Engine",
  history: "Neural Memory",
  settings: "System Protocols",
};

const waveformBars = [32, 58, 44, 76, 52, 86, 62, 40, 70, 48, 82, 56, 36, 68, 46, 78, 54, 90];
const Lottie = ((LottieImport as unknown as { default?: typeof LottieImport }).default ?? LottieImport) as typeof LottieImport;

type AssistantUiState = "idle" | "listening" | "thinking" | "executing" | "success" | "error" | "speaking";

type AssistantActivity = {
  detail?: string;
  state: Exclude<AssistantUiState, "idle" | "listening" | "speaking">;
};

type AssistantStateCopy = {
  detail: string;
  orbLabel: string;
  title: string;
};

function App() {
  const [activeView, setActiveView] = useState<AppView>("home");
  const [runtime, setRuntime] = useState<RuntimeStatus | null>(null);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [settings, setSettings] = useState<AssistantSettings>(defaultSettings);
  const [account, setAccount] = useState<AccountStatus | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [history, setHistory] = useState<CommandHistoryEntry[]>([]);
  const [memories, setMemories] = useState<MemoryEntry[]>([]);
  const [musicFavorites, setMusicFavorites] = useState<MusicFavorite[]>([]);
  const [taskState, setTaskState] = useState<TaskState>({ completed_steps: [] });
  const [pcState, setPcState] = useState<PcControlState | null>(null);
  const [apps, setApps] = useState<InstalledApp[]>([]);
  const [track, setTrack] = useState<SpotifyTrack | null>(null);
  const [routines, setRoutines] = useState<Routine[]>([]);
  const [playlists, setPlaylists] = useState<SpotifyPlaylist[]>([]);
  const [musicSearch, setMusicSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [assistantActivity, setAssistantActivity] = useState<AssistantActivity | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [chatInput, setChatInput] = useState("");
  const [voiceState, setVoiceState] = useState<VoiceState | null>(null);
  const assistantResetRef = useRef<number | null>(null);
  const voicePollRef = useRef<number | null>(null);

  const refreshMusic = useCallback(async () => {
    const [nextTrack, nextFavorites] = await Promise.all([
      api.spotifyCurrent().catch(() => null),
      api.listMusicFavorites(),
    ]);
    setTrack(nextTrack);
    setMusicFavorites(nextFavorites);
  }, []);

  const refreshAll = useCallback(async () => {
    setLoading(true);
    try {
      const [
        nextRuntime,
        nextStatus,
        nextSettings,
        nextAccount,
        nextMessages,
        nextLogs,
        nextHistory,
        nextMemories,
        nextMusicFavorites,
        nextTaskState,
        nextPcState,
        nextApps,
        nextVoiceState,
        nextTrack,
        nextRoutines,
        nextPlaylists,
      ] = await Promise.all([
        api.getRuntimeStatus(),
        api.getAppStatus(),
        api.getSettings(),
        api.getAccountStatus(),
        api.listMessages(),
        api.listLogs(),
        api.listHistory(),
        api.listMemories(),
        api.listMusicFavorites(),
        api.getTaskState(),
        api.getPcControlState(),
        api.listInstalledApps(),
        api.getVoiceState(),
        api.spotifyCurrent().catch(() => null),
        api.listRoutines(),
        api.spotifyPlaylists().catch(() => []),
      ]);
      setRuntime(nextRuntime);
      setStatus(nextStatus);
      setSettings(nextSettings);
      setAccount(nextAccount);
      setMessages(nextMessages);
      setLogs(nextLogs);
      setHistory(nextHistory);
      setMemories(nextMemories);
      setMusicFavorites(nextMusicFavorites);
      setTaskState(nextTaskState);
      setPcState(nextPcState);
      setApps(nextApps);
      setVoiceState(nextVoiceState);
      setTrack(nextTrack);
      setRoutines(nextRoutines);
      setPlaylists(nextPlaylists);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  const recentApps = useMemo(() => apps.filter((app) => app.recent).slice(0, 5), [apps]);
  const favoriteApps = useMemo(() => apps.filter((app) => app.favorite), [apps]);
  const voiceActive = isVoiceActive(voiceState);
  const musicActive = Boolean(track?.is_playing);
  const clearAssistantActivityReset = useCallback(() => {
    if (assistantResetRef.current) {
      window.clearTimeout(assistantResetRef.current);
      assistantResetRef.current = null;
    }
  }, []);

  const clearVoicePoll = useCallback(() => {
    if (voicePollRef.current) {
      window.clearTimeout(voicePollRef.current);
      voicePollRef.current = null;
    }
  }, []);

  const setAssistantActivityState = useCallback(
    (activity: AssistantActivity | null, resetAfterMs?: number) => {
      clearAssistantActivityReset();
      setAssistantActivity(activity);
      if (activity && resetAfterMs) {
        assistantResetRef.current = window.setTimeout(() => {
          setAssistantActivity(null);
          assistantResetRef.current = null;
        }, resetAfterMs);
      }
    },
    [clearAssistantActivityReset],
  );

  useEffect(
    () => () => {
      clearAssistantActivityReset();
      clearVoicePoll();
    },
    [clearAssistantActivityReset, clearVoicePoll],
  );

  const assistantUiState = useMemo(
    () => deriveAssistantUiState(voiceState, assistantActivity, runtime),
    [assistantActivity, runtime, voiceState],
  );
  const assistantCopy = useMemo(
    () => getAssistantStateCopy(assistantUiState, voiceState, status, runtime, assistantActivity),
    [assistantActivity, assistantUiState, runtime, status, voiceState],
  );

  async function handleSend(event?: FormEvent, override?: string) {
    event?.preventDefault();
    const content = (override ?? chatInput).trim();
    if (!content || busy) return;

    setBusy(true);
    setAssistantActivityState({ state: "thinking", detail: "Aguardando resposta real da IA local." });
    setChatInput("");
    setMessages((current) => [
      ...current,
      {
        id: crypto.randomUUID(),
        role: "user",
        content,
        created_at: new Date().toISOString(),
        status: "done",
      },
    ]);

    try {
      const result = await api.sendMessage(content, "texto");
      setMessages((current) => [...current, result.message]);
      const [nextStatus, nextHistory, nextPcState, nextLogs, nextMemories, nextTaskState, nextTrack] =
        await Promise.all([
          api.getAppStatus(),
          api.listHistory(),
          api.getPcControlState(),
          api.listLogs(),
          api.listMemories(),
          api.getTaskState(),
          api.spotifyCurrent().catch(() => null),
        ]);
      setStatus(nextStatus);
      setHistory(nextHistory);
      setPcState(nextPcState);
      setLogs(nextLogs);
      setMemories(nextMemories);
      setTaskState(nextTaskState);
      setTrack(nextTrack);
      setAssistantActivityState(
        result.message.status === "error"
          ? { state: "error", detail: result.message.content }
          : { state: "success", detail: "Resposta pronta." },
        2800,
      );
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setMessages((current) => [
        ...current,
        {
          id: crypto.randomUUID(),
          role: "assistant",
          content: message,
          created_at: new Date().toISOString(),
          status: "error",
        },
      ]);
      setAssistantActivityState({ state: "error", detail: message }, 4200);
    } finally {
      setBusy(false);
    }
  }

  async function confirmAction(action: ActionSpec) {
    const ok =
      action.risk === "high" || action.requires_confirmation
        ? window.confirm(`Confirmar acao: ${action.label}?`)
        : true;
    if (!ok) return;
    setBusy(true);
    setAssistantActivityState({ state: "executing", detail: action.label });
    try {
      const log = await api.executeAction(action);
      setNotice(log.message);
      const [nextHistory, nextPcState, nextLogs] = await Promise.all([
        api.listHistory(),
        api.getPcControlState(),
        api.listLogs(),
      ]);
      setHistory(nextHistory);
      setPcState(nextPcState);
      setLogs(nextLogs);
      await refreshMusic();
      setAssistantActivityState({ state: "success", detail: log.message }, 2800);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setNotice(message);
      setAssistantActivityState({ state: "error", detail: message }, 4200);
    } finally {
      setBusy(false);
    }
  }

  async function saveSettings(nextSettings = settings) {
    setBusy(true);
    try {
      const saved = await api.saveSettings(nextSettings);
      setSettings(saved);
      setStatus(await api.getAppStatus());
      setNotice("Protocolos do sistema salvos.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function updateVolume(level: number) {
    const nextLevel = Math.max(0, Math.min(100, level));
    setPcState((current) => (current ? { ...current, volume: nextLevel } : current));
    setSettings((current) => ({ ...current, system_volume: nextLevel }));
    try {
      const saved = await api.setVolume(nextLevel);
      setPcState((current) => (current ? { ...current, volume: saved } : current));
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function updateBrightness(level: number) {
    const nextLevel = Math.max(0, Math.min(100, level));
    setPcState((current) => (current ? { ...current, brightness: nextLevel } : current));
    try {
      const saved = await api.setBrightness(nextLevel);
      setPcState((current) => (current ? { ...current, brightness: saved } : current));
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function refreshPc() {
    const [nextPcState, nextProcesses] = await Promise.all([api.getPcControlState(), api.listProcesses()]);
    setPcState({ ...nextPcState, processes: nextProcesses });
  }

  async function takeScreenshot() {
    setBusy(true);
    setAssistantActivityState({ state: "executing", detail: "Capturando tela." });
    try {
      const path = await api.takeScreenshot();
      setNotice(`Captura holografica salva: ${path}`);
      setHistory(await api.listHistory());
      setAssistantActivityState({ state: "success", detail: "Captura de tela concluida." }, 2800);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setNotice(message);
      setAssistantActivityState({ state: "error", detail: message }, 4200);
    } finally {
      setBusy(false);
    }
  }

  async function openConfiguredApp(app: InstalledApp) {
    await api.openApp(app.command);
    setHistory(await api.listHistory());
  }

  async function closeConfiguredApp(app: InstalledApp) {
    const ok = window.confirm(`Fechar ${app.name}?`);
    if (!ok) return;
    await api.closeApp(app.command);
    setHistory(await api.listHistory());
  }

  async function closeProcess(process: ProcessInfo) {
    const ok = process.critical
      ? window.confirm(`${process.name} parece critico. Confirmar mesmo assim?`)
      : window.confirm(`Encerrar ${process.name}?`);
    if (!ok) return;
    try {
      await api.closeProcess(process.pid);
      await refreshPc();
      setHistory(await api.listHistory());
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function controlWindow(id: string, operation: "focus" | "minimize" | "maximize") {
    try {
      if (operation === "focus") await api.focusWindow(id);
      if (operation === "minimize") await api.minimizeWindow(id);
      if (operation === "maximize") await api.maximizeWindow(id);
      await refreshPc();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  function pollVoiceTranscript(startedAt = Date.now()) {
    clearVoicePoll();
    voicePollRef.current = window.setTimeout(() => {
      void (async () => {
        try {
          const next = await api.getVoiceState();
          setVoiceState(next);
          const transcript = next.transcript?.trim();
          if (transcript) {
            clearVoicePoll();
            setAssistantActivityState({ state: "thinking", detail: "Voz transcrita. Enviando para a IA local." });
            await handleSend(undefined, transcript);
            return;
          }

          if (isVoiceActive(next) && Date.now() - startedAt < 14_000) {
            pollVoiceTranscript(startedAt);
            return;
          }

          if (next.state === "error" || next.error) {
            setAssistantActivityState({ state: "error", detail: next.error ?? "Nao consegui transcrever a fala." }, 4200);
          }
        } catch (error) {
          setAssistantActivityState(
            { state: "error", detail: error instanceof Error ? error.message : String(error) },
            4200,
          );
        }
      })();
    }, 900);
  }

  async function startVoiceListening() {
    clearVoicePoll();
    setAssistantActivityState({ state: "executing", detail: "Solicitando escuta local." });
    try {
      const next = await api.startListening();
      setVoiceState(next);
      setStatus(await api.getAppStatus());
      if (isVoiceActive(next)) {
        pollVoiceTranscript();
      }
      if (!isVoiceActive(next)) {
        setAssistantActivityState(
          next.state === "error" || next.error
            ? { state: "error", detail: next.error ?? "Escuta local indisponivel." }
            : { state: "success", detail: "Microfone em espera." },
          2600,
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setNotice(message);
      setAssistantActivityState({ state: "error", detail: message }, 4200);
      setVoiceState(await api.getVoiceState());
    }
  }

  async function stopVoiceListening() {
    clearVoicePoll();
    setAssistantActivityState({ state: "executing", detail: "Encerrando escuta local." });
    try {
      setVoiceState(await api.stopListening());
      setStatus(await api.getAppStatus());
      setAssistantActivityState({ state: "success", detail: "Escuta pausada." }, 2200);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setNotice(message);
      setAssistantActivityState({ state: "error", detail: message }, 4200);
    }
  }

  async function clearHistory() {
    const ok = window.confirm("Limpar historico de conversas, comandos e logs?");
    if (!ok) return;
    await api.clearHistory();
    setMessages([]);
    setHistory(await api.listHistory());
    setLogs(await api.listLogs());
  }

  async function connectGoogle() {
    try {
      await api.googleLogin();
      setAccount(await api.getAccountStatus());
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function logoutGoogle() {
    await api.googleLogout();
    setAccount(await api.getAccountStatus());
  }

  async function runRoutine(id: string) {
    setBusy(true);
    setAssistantActivityState({ state: "executing", detail: "Executando rotina salva." });
    try {
      const log = await api.runRoutine(id);
      setNotice(log.message);
      setHistory(await api.listHistory());
      setLogs(await api.listLogs());
      setAssistantActivityState({ state: "success", detail: log.message }, 2800);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setNotice(message);
      setAssistantActivityState({ state: "error", detail: message }, 4200);
    } finally {
      setBusy(false);
    }
  }

  async function saveRoutine(routine: Routine) {
    setBusy(true);
    try {
      const saved = await api.saveRoutine(routine);
      setRoutines((current) => [saved, ...current.filter((item) => item.id !== saved.id)]);
      setLogs(await api.listLogs());
      setNotice(`Rotina salva: ${saved.name}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  async function spotifyAuth() {
    if (!settings.spotify_client_id.trim()) {
      setNotice("Informe o Spotify Client ID em Sistema antes de conectar.");
      setActiveView("settings");
      return;
    }
    try {
      const authUrl = await api.spotifyBeginAuth(settings.spotify_client_id, settings.spotify_redirect_uri);
      window.open(authUrl, "_blank", "noopener,noreferrer");
      setNotice("Fluxo Spotify aberto. Finalize a autorizacao no navegador.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function searchSpotify() {
    const query = musicSearch.trim() || "focus";
    try {
      setPlaylists(await api.spotifySearch(query));
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function playUri(uri: string) {
    try {
      await api.spotifyPlay(uri);
      await refreshMusic();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function playLiked() {
    try {
      await api.spotifyPlayLiked();
      await refreshMusic();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function pauseMusic() {
    try {
      await api.spotifyPause();
      await refreshMusic();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function skipMusic(direction: "next" | "previous") {
    try {
      if (direction === "next") await api.spotifyNext();
      if (direction === "previous") await api.spotifyPrevious();
      await refreshMusic();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function favoritePlaylist(playlist: SpotifyPlaylist) {
    await api.saveMusicFavorite(playlist.name, playlist.uri, null, "playlist");
    setMusicFavorites(await api.listMusicFavorites());
  }

  async function playFavorite(favorite: MusicFavorite) {
    const target = favorite.uri ?? favorite.query ?? favorite.name;
    try {
      if (favorite.uri) {
        await api.spotifyPlay(target);
      } else {
        await api.playMusic(target);
      }
      await refreshMusic();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error));
    }
  }

  async function deleteMusicFavorite(id: string) {
    await api.deleteMusicFavorite(id);
    setMusicFavorites(await api.listMusicFavorites());
  }

  const activeScreen = (
    <AnimatePresence mode="wait">
      <motion.section
        animate={{ opacity: 1, y: 0, scale: 1 }}
        className="view-stage"
        exit={{ opacity: 0, y: 18, scale: 0.985 }}
        initial={{ opacity: 0, y: 18, scale: 0.985 }}
        key={activeView}
        transition={{ duration: 0.34, ease: [0.22, 1, 0.36, 1] }}
      >
        {activeView === "home" && (
          <QuickHomeView
            assistantCopy={assistantCopy}
            assistantState={assistantUiState}
            busy={busy}
            input={chatInput}
            messages={messages}
            runtime={runtime}
            status={status}
            voiceState={voiceState}
            onConfirmAction={confirmAction}
            onInput={setChatInput}
            onPrompt={(prompt) => void handleSend(undefined, prompt)}
            onSend={handleSend}
            onStartListening={startVoiceListening}
            onStopListening={stopVoiceListening}
          />
        )}

        {activeView === "pc_control" && (
          <PcControlView
            pcState={pcState}
            runtime={runtime}
            onBrightness={updateBrightness}
            onCloseProcess={closeProcess}
            onRefresh={refreshPc}
            onScreenshot={takeScreenshot}
            onVolume={updateVolume}
            onWindowControl={controlWindow}
          />
        )}

        {activeView === "apps" && (
          <AppsView apps={apps} favorites={favoriteApps} onClose={closeConfiguredApp} onOpen={openConfiguredApp} />
        )}

        {activeView === "music" && (
          <MusicView
            favorites={musicFavorites}
            playlists={playlists}
            search={musicSearch}
            settings={settings}
            track={track}
            onDeleteFavorite={deleteMusicFavorite}
            onFavoritePlaylist={favoritePlaylist}
            onPause={pauseMusic}
            onPlay={playUri}
            onPlayFavorite={playFavorite}
            onPlayLiked={playLiked}
            onSearch={searchSpotify}
            onSearchChange={setMusicSearch}
            onSettingsChange={setSettings}
            onSkipBack={() => void skipMusic("previous")}
            onSkipForward={() => void skipMusic("next")}
            onSpotifyAuth={spotifyAuth}
          />
        )}

        {activeView === "voice" && (
          <VoiceView
            runtime={runtime}
            settings={settings}
            voiceState={voiceState}
            onSave={() => saveSettings()}
            onSettingsChange={setSettings}
            onStart={startVoiceListening}
            onStop={stopVoiceListening}
          />
        )}

        {activeView === "automations" && <AutomationsView routines={routines} onRun={runRoutine} onSave={saveRoutine} />}

        {activeView === "history" && <HistoryView history={history} logs={logs} onClear={clearHistory} />}

        {activeView === "settings" && (
          <SettingsView
            account={account}
            memories={memories}
            settings={settings}
            status={status}
            onDeleteMemory={async (id) => {
              await api.deleteMemory(id);
              setMemories(await api.listMemories());
              setHistory(await api.listHistory());
            }}
            onGoogle={connectGoogle}
            onGoogleLogout={logoutGoogle}
            onSave={() => saveSettings()}
            onSaveMemory={async (title, content, category) => {
              await api.saveMemory(title, content, category, true);
              setMemories(await api.listMemories());
              setHistory(await api.listHistory());
            }}
            onSetLocalAiPath={async (path) => {
              const next = { ...settings, local_ai_path: path || null, local_ai_provider: "qwen_embedded" } as AssistantSettings;
              setSettings(next);
              await api.setLocalAiPath(path);
              await saveSettings(next);
            }}
            onSettingsChange={setSettings}
          />
        )}
      </motion.section>
    </AnimatePresence>
  );

  if (loading && !status) {
    return (
      <NeuralFrame activeView="home" musicActive={false} voiceActive={false}>
        <NeuralBootScreen />
      </NeuralFrame>
    );
  }

  if (status && !status.onboarding_complete) {
    return (
      <NeuralFrame activeView="settings" musicActive={musicActive} voiceActive={voiceActive}>
        <OnboardingGate
          initialSettings={settings}
          onGoogle={connectGoogle}
          onComplete={async (nextSettings) => {
            const saved = await api.completeOnboarding(nextSettings);
            setSettings(saved);
            await refreshAll();
          }}
        />
      </NeuralFrame>
    );
  }

  return (
    <NeuralFrame activeView={activeView} musicActive={musicActive} voiceActive={voiceActive}>
      <div className="app-shell neural-shell">
        <aside className="sidebar neural-sidebar">
          <div className="brand neural-brand">
            <div className="brand-mark neural-brand-mark">
              <Lottie animationData={neuralPulse} autoplay loop />
            </div>
            <div>
              <strong>PC Control AI</strong>
              <span>Neural OS online</span>
            </div>
          </div>

          <nav className="nav-list neural-nav" aria-label="Menu principal">
            {navItems.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  className={`nav-item neural-nav-item ${activeView === item.view ? "active" : ""}`}
                  key={item.view}
                  onClick={() => setActiveView(item.view)}
                  type="button"
                >
                  <Icon size={18} />
                  <span>{item.label}</span>
                  <em>{item.signal}</em>
                </button>
              );
            })}
          </nav>

          <div className="status-stack neural-status-stack">
            <StatusPill ok={Boolean(status?.local_ai_ready)} label={localAiStatusLabel(status)} />
            <StatusPill
              ok={Boolean(status?.microphone_ready)}
              label={status?.microphone_ready ? "Voz ativa" : voiceState?.engine_ready ? "Voz em espera" : "Voz offline"}
            />
          </div>
        </aside>

        <main className="workspace neural-workspace">
          <header className="topbar neural-topbar">
            <div>
              <p className="eyebrow">{settings.assistant_name}</p>
              <h1>{viewTitles[activeView]}</h1>
            </div>
            <div className="topbar-actions">
              <NeuralButton onClick={() => void refreshAll()} type="button">
                <RefreshCw size={16} />
                Sincronizar
              </NeuralButton>
              <NeuralButton onClick={() => setActiveView("settings")} type="button">
                <ShieldCheck size={16} />
                Protocolos
              </NeuralButton>
            </div>
          </header>

          {notice && (
            <motion.div animate={{ opacity: 1, y: 0 }} className="inline-notice neural-notice warn" initial={{ opacity: 0, y: -8 }}>
              <Activity size={16} />
              <span>{notice}</span>
              <NeuralButton className="small" onClick={() => setNotice(null)} type="button">
                Fechar
              </NeuralButton>
            </motion.div>
          )}

          {activeScreen}
        </main>
      </div>
    </NeuralFrame>
  );
}

function NeuralFrame({
  activeView,
  children,
  musicActive,
  voiceActive,
}: {
  activeView: AppView;
  children: ReactNode;
  musicActive: boolean;
  voiceActive: boolean;
}) {
  return (
    <div className="neural-frame">
      <AICoreScene activeView={activeView} musicActive={musicActive} voiceActive={voiceActive} />
      <div className="aurora-field" />
      <div className="scanline-layer" />
      {children}
    </div>
  );
}

function AICoreScene({
  activeView,
  musicActive,
  voiceActive,
}: {
  activeView: AppView;
  musicActive: boolean;
  voiceActive: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const reduceMotion = useReducedMotion();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(70, 1, 0.1, 100);
    camera.position.z = 6;

    const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, canvas });
    renderer.setClearAlpha(0);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.8));

    const count = reduceMotion ? 120 : 520;
    const positions = new Float32Array(count * 3);
    const seeds = new Float32Array(count);
    for (let index = 0; index < count; index += 1) {
      const radius = 1.8 + Math.random() * 5.2;
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      positions[index * 3] = radius * Math.sin(phi) * Math.cos(theta);
      positions[index * 3 + 1] = radius * Math.sin(phi) * Math.sin(theta);
      positions[index * 3 + 2] = radius * Math.cos(phi);
      seeds[index] = Math.random();
    }

    const particleGeometry = new THREE.BufferGeometry();
    particleGeometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    const color = new THREE.Color(voiceActive ? "#71f7ff" : musicActive ? "#b47cff" : "#43d6ff");
    const particleMaterial = new THREE.PointsMaterial({
      blending: THREE.AdditiveBlending,
      color,
      depthWrite: false,
      opacity: voiceActive || musicActive ? 0.52 : 0.34,
      size: voiceActive ? 0.021 : 0.016,
      transparent: true,
    });
    const particles = new THREE.Points(particleGeometry, particleMaterial);
    scene.add(particles);

    const ringMaterial = new THREE.MeshBasicMaterial({
      blending: THREE.AdditiveBlending,
      color: new THREE.Color(musicActive ? "#a26bff" : "#4fe7ff"),
      opacity: voiceActive || musicActive ? 0.28 : 0.16,
      transparent: true,
    });
    const ringOne = new THREE.Mesh(new THREE.TorusGeometry(1.35, 0.01, 10, 160), ringMaterial);
    const ringTwo = new THREE.Mesh(new THREE.TorusGeometry(2.12, 0.008, 10, 180), ringMaterial.clone());
    ringTwo.rotation.x = Math.PI / 2.8;
    ringTwo.rotation.y = Math.PI / 4;
    scene.add(ringOne, ringTwo);

    const resize = () => {
      renderer.setSize(window.innerWidth, window.innerHeight, false);
      camera.aspect = window.innerWidth / window.innerHeight;
      camera.updateProjectionMatrix();
    };
    resize();
    window.addEventListener("resize", resize);

    let frame = 0;
    const intensity = voiceActive ? 1.28 : musicActive ? 1.14 : activeView === "voice" ? 1.08 : 0.92;
    const animate = () => {
      frame = window.requestAnimationFrame(animate);
      const time = performance.now() * 0.001;
      if (!reduceMotion) {
        const position = particleGeometry.getAttribute("position") as THREE.BufferAttribute;
        for (let index = 0; index < count; index += 1) {
          const base = index * 3;
          const pulse = Math.sin(time * (0.8 + seeds[index] * 1.6) + seeds[index] * 12) * 0.0016 * intensity;
          positions[base + 1] += pulse;
        }
        position.needsUpdate = true;
        particles.rotation.y = time * 0.025 * intensity;
        particles.rotation.x = Math.sin(time * 0.18) * 0.08;
        ringOne.rotation.z = time * 0.28 * intensity;
        ringTwo.rotation.y = time * -0.18 * intensity;
        ringOne.scale.setScalar(1 + Math.sin(time * 1.4) * 0.025 * intensity);
        ringTwo.scale.setScalar(1 + Math.cos(time * 1.1) * 0.018 * intensity);
      }
      renderer.render(scene, camera);
    };
    animate();

    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
      particleGeometry.dispose();
      particleMaterial.dispose();
      ringMaterial.dispose();
      (ringTwo.material as THREE.Material).dispose();
      ringOne.geometry.dispose();
      ringTwo.geometry.dispose();
      renderer.dispose();
    };
  }, [activeView, musicActive, reduceMotion, voiceActive]);

  return <canvas aria-hidden className="ai-scene" ref={canvasRef} />;
}

function NeuralBootScreen() {
  const bootLines = ["NEURAL CORE", "LOCAL AI", "VOICE BUS", "PC CONTROL", "SONIC LINK"];
  return (
    <main className="login-screen neural-login">
      <motion.section
        animate={{ opacity: 1, scale: 1, y: 0 }}
        className="loading-panel neural-boot"
        initial={{ opacity: 0, scale: 0.96, y: 20 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
      >
        <div className="boot-orb-wrap">
          <NeuralOrb active label="BOOT" />
          <Lottie animationData={neuralPulse} autoplay loop />
        </div>
        <div className="boot-copy">
          <p className="eyebrow">Inicializacao cinematica</p>
          <h1>PC Control AI</h1>
          <span>Compilando consciencia operacional...</span>
        </div>
        <div className="boot-grid">
          {bootLines.map((line, index) => (
            <motion.div
              animate={{ opacity: 1, x: 0 }}
              className="boot-line"
              initial={{ opacity: 0, x: -18 }}
              key={line}
              transition={{ delay: index * 0.09, duration: 0.42 }}
            >
              <span>{line}</span>
              <strong>{index % 2 === 0 ? "ONLINE" : "SYNC"}</strong>
            </motion.div>
          ))}
        </div>
      </motion.section>
    </main>
  );
}

function OnboardingGate({
  initialSettings,
  onComplete,
  onGoogle,
}: {
  initialSettings: AssistantSettings;
  onComplete: (settings: AssistantSettings) => Promise<void>;
  onGoogle: () => Promise<void>;
}) {
  const [draft, setDraft] = useState<AssistantSettings>(initialSettings);
  const [saving, setSaving] = useState(false);

  return (
    <main className="login-screen neural-login">
      <motion.section
        animate={{ opacity: 1, y: 0 }}
        className="login-panel onboarding-panel neural-onboarding"
        initial={{ opacity: 0, y: 24 }}
        transition={{ duration: 0.55 }}
      >
        <div className="activation-header">
          <NeuralOrb active label="AI" />
          <div>
            <p className="eyebrow">Ativacao neural</p>
            <h1>Sincronizar PC Control AI</h1>
            <p>Configure a identidade, voz e permissoes para iniciar o sistema como uma IA local viva.</p>
          </div>
        </div>

        <div className="onboarding-grid">
          <label>
            Nome do assistente
            <input
              value={draft.assistant_name}
              onChange={(event) => setDraft({ ...draft, assistant_name: event.target.value })}
            />
          </label>
          <label>
            Palavra de ativacao
            <input value={draft.wake_word} onChange={(event) => setDraft({ ...draft, wake_word: event.target.value })} />
          </label>
          <label>
            Modo de funcionamento
            <select
              value={draft.operation_mode}
              onChange={(event) => setDraft({ ...draft, operation_mode: event.target.value })}
            >
              <option value="auto_confirm_dangerous">Automatico, confirmar perigosas</option>
            </select>
          </label>
        </div>

        <div className="permission-grid compact">
          {Object.entries(draft.permissions).map(([key, value]) => (
            <label className="toggle-line neural-toggle" key={key}>
              <input
                checked={value}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    permissions: { ...draft.permissions, [key]: event.target.checked },
                  })
                }
                type="checkbox"
              />
              <span>{permissionLabel(key)}</span>
            </label>
          ))}
        </div>

        <div className="button-row">
          <NeuralButton
            className="wide"
            disabled={saving}
            onClick={async () => {
              setSaving(true);
              await onComplete(draft);
              setSaving(false);
            }}
            type="button"
            variant="primary"
          >
            <LogIn size={17} />
            Ativar sem conta
          </NeuralButton>
          <NeuralButton className="wide" onClick={() => void onGoogle()} type="button">
            <UserCircle size={17} />
            Conectar Google opcional
          </NeuralButton>
        </div>
      </motion.section>
    </main>
  );
}

function QuickHomeView({
  assistantCopy,
  assistantState,
  busy,
  input,
  messages,
  runtime,
  status,
  voiceState,
  onConfirmAction,
  onInput,
  onPrompt,
  onSend,
  onStartListening,
  onStopListening,
}: {
  assistantCopy: AssistantStateCopy;
  assistantState: AssistantUiState;
  busy: boolean;
  input: string;
  messages: ChatMessage[];
  runtime: RuntimeStatus | null;
  status: AppStatus | null;
  voiceState: VoiceState | null;
  onConfirmAction: (action: ActionSpec) => Promise<void>;
  onInput: (value: string) => void;
  onPrompt: (prompt: string) => void;
  onSend: (event: FormEvent) => void;
  onStartListening: () => Promise<void>;
  onStopListening: () => Promise<void>;
}) {
  const voiceActive = isVoiceActive(voiceState);
  const canStartVoice = Boolean(voiceState?.engine_ready);
  const voiceButtonDisabled = voiceActive ? false : !canStartVoice;
  const voiceButtonLabel = voiceActive ? "Parar" : canStartVoice ? "Falar" : "Voz indisponivel";
  const waveActive = assistantState === "listening" || assistantState === "thinking" || assistantState === "executing";
  const visibleMessages = messages.slice(-7);

  return (
    <div className={`assistant-home neural-home clean-home state-${assistantState}`}>
      <HoloPanel className="assistant-console clean-assistant-console" glow={assistantState !== "idle"}>
        <div className="assistant-presence">
          <div className="assistant-orb-stage">
            <NeuralOrb
              active={assistantState !== "idle" || Boolean(status?.local_ai_ready)}
              label={assistantCopy.orbLabel}
              large
            />
            <AudioWaveform active={waveActive} large level={voiceState?.state === "transcribing" ? 68 : 42} />
          </div>

          <div className="assistant-state-copy">
            <p className="eyebrow">Assistente pessoal</p>
            <h2>{assistantCopy.title}</h2>
            <p className="muted-copy">{assistantCopy.detail}</p>
          </div>

          <div className="voice-primary-wrap">
            <button
              className={`voice-primary state-${assistantState}`}
              disabled={voiceButtonDisabled}
              onClick={() => void (voiceActive ? onStopListening() : onStartListening())}
              type="button"
            >
              {voiceActive ? <MicOff size={30} /> : <Mic size={30} />}
              <span>{voiceButtonLabel}</span>
            </button>
            <span className="voice-primary-hint">
              {voiceActive
                ? voiceStateLabel(voiceState)
                : canStartVoice
                  ? `Diga "${voiceState?.wake_word ?? "assistente"}" ou toque para falar.`
                  : "Microfone local nao esta ativo."}
            </span>
          </div>
        </div>

        <div className="assistant-status-rail">
          <StatusPill ok={Boolean(status?.local_ai_ready)} label={localAiShortLabel(status)} />
          <StatusPill ok={Boolean(voiceState?.engine_ready)} label={voiceStateLabel(voiceState)} />
          <StatusPill
            ok={runtime?.mode === "native"}
            label={runtime?.mode === "native" ? "App nativo" : "Modo visual"}
          />
        </div>

        <form className="composer neural-composer clean-composer primary-composer" onSubmit={onSend}>
          <input
            placeholder="Digite aqui seu comando..."
            value={input}
            onChange={(event) => onInput(event.target.value)}
          />
          <NeuralButton disabled={busy || !input.trim()} type="submit" variant="primary">
            <Send size={17} />
            Enviar
          </NeuralButton>
        </form>

        <div className="assistant-conversation">
          <div className="conversation-heading">
            <div>
              <p className="eyebrow">Conversa recente</p>
              <h3>Fale ou digite diretamente para a IA</h3>
            </div>
            <AudioWaveform active={busy || voiceActive} />
          </div>

          <div className="message-list clean-message-list">
            {visibleMessages.map((message) => (
              <ChatBubble message={message} onConfirmAction={onConfirmAction} key={message.id} />
            ))}
            {!visibleMessages.length && (
              <EmptyState
                icon={Bot}
                title="Conversa pronta"
                text="Nenhuma resposta foi gerada ainda. Envie um comando para iniciar."
              />
            )}
          </div>
        </div>

        <div className="prompt-strip clean-prompts" aria-label="Sugestoes de comandos">
          {promptSuggestions.slice(0, 5).map((prompt) => (
            <button key={prompt} onClick={() => onPrompt(prompt)} type="button">
              <Zap size={14} />
              {prompt}
            </button>
          ))}
        </div>
      </HoloPanel>
    </div>
  );
}

function HomeView({
  history,
  onOpenApp,
  onPrompt,
  pcState,
  recentApps,
  status,
  track,
}: {
  history: CommandHistoryEntry[];
  onOpenApp: (app: InstalledApp) => Promise<void>;
  onPrompt: (prompt: string) => void;
  pcState: PcControlState | null;
  recentApps: InstalledApp[];
  status: AppStatus | null;
  track: SpotifyTrack | null;
}) {
  return (
    <div className="dashboard-grid neural-dashboard">
      <HoloPanel className="hero-panel" glow>
        <p className="eyebrow">Sistema neural</p>
        <h2>Assistente pronto para comandos locais</h2>
        <div className="stat-grid">
          <Metric label="IA local" value={localAiShortLabel(status)} ok={Boolean(status?.local_ai_ready)} />
          <Metric label="Volume" value={`${pcState?.volume ?? 50}%`} ok />
          <Metric label="Musica" value={track?.name ?? "Idle"} ok={Boolean(track)} />
        </div>
      </HoloPanel>
      <HoloPanel>
        <h2>Acoes comuns</h2>
        <div className="quick-card-grid">
          {promptSuggestions.slice(0, 4).map((prompt) => (
            <button className="quick-card neural-card-button" key={prompt} onClick={() => onPrompt(prompt)} type="button">
              <Sparkles size={18} />
              <span>{prompt}</span>
            </button>
          ))}
        </div>
      </HoloPanel>
      <HoloPanel>
        <h2>Apps recentes</h2>
        <div className="card-list">
          {recentApps.map((app) => (
            <button className="list-button" key={app.id} onClick={() => void onOpenApp(app)} type="button">
              <AppWindow size={17} />
              <span>{app.name}</span>
            </button>
          ))}
        </div>
      </HoloPanel>
      <HoloPanel className="wide-panel">
        <h2>Historico curto</h2>
        <div className="log-list">
          {history.slice(0, 5).map((item) => (
            <HistoryRow item={item} key={item.id} />
          ))}
        </div>
      </HoloPanel>
    </div>
  );
}

function AssistantView({
  busy,
  input,
  memories,
  messages,
  onConfirmAction,
  onInput,
  onSend,
}: {
  busy: boolean;
  input: string;
  memories: MemoryEntry[];
  messages: ChatMessage[];
  onConfirmAction: (action: ActionSpec) => Promise<void>;
  onInput: (value: string) => void;
  onSend: (event: FormEvent) => void;
}) {
  return (
    <div className="chat-layout neural-assistant">
      <HoloPanel className="chat-main" glow>
        <div className="mode-row">
          <button className="segmented active" type="button">Texto</button>
          <button className="segmented" type="button">Voz push-to-talk</button>
          <span className="muted-copy">Nucleo local protegido</span>
        </div>
        <div className="message-list">
          {messages.map((message) => (
            <ChatBubble message={message} onConfirmAction={onConfirmAction} key={message.id} />
          ))}
        </div>
        <form className="composer neural-composer" onSubmit={onSend}>
          <input value={input} onChange={(event) => onInput(event.target.value)} placeholder="Digite um comando..." />
          <NeuralButton disabled={busy || !input.trim()} type="submit" variant="primary">
            <Send size={17} />
            Enviar
          </NeuralButton>
        </form>
      </HoloPanel>
      <aside className="chat-side">
        <HoloPanel className="side-panel">
          <h2>Memoria ativa</h2>
          <div className="memory-mini-list">
            {memories.slice(0, 5).map((memory) => (
              <div className="memory-chip" key={memory.id}>
                <strong>{memory.title}</strong>
                <span>{memory.content}</span>
              </div>
            ))}
          </div>
        </HoloPanel>
      </aside>
    </div>
  );
}

function PcControlView({
  onBrightness,
  onCloseProcess,
  onRefresh,
  onScreenshot,
  onVolume,
  onWindowControl,
  pcState,
  runtime,
}: {
  onBrightness: (level: number) => Promise<void>;
  onCloseProcess: (process: ProcessInfo) => Promise<void>;
  onRefresh: () => Promise<void>;
  onScreenshot: () => Promise<void>;
  onVolume: (level: number) => Promise<void>;
  onWindowControl: (id: string, operation: "focus" | "minimize" | "maximize") => Promise<void>;
  pcState: PcControlState | null;
  runtime: RuntimeStatus | null;
}) {
  if (!runtime?.capabilities.pc_control) {
    return (
      <HoloPanel>
        <EmptyState
          icon={Monitor}
          title="Controle operacional indisponivel"
          text="Abra o aplicativo instalado para ler volume, capturar tela e listar processos reais."
        />
      </HoloPanel>
    );
  }

  const processes = pcState?.processes ?? [];
  return (
    <div className="two-column command-grid">
      <HoloPanel className="control-panel" glow>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Central de comando</p>
            <h2>Windows HUD</h2>
          </div>
          <Monitor size={22} />
        </div>

        <div className="stat-grid compact-stats">
          <Metric label="Volume" value={`${pcState?.volume ?? 50}%`} ok />
          <Metric label="Brilho" value={`${pcState?.brightness ?? 0}%`} ok={Boolean(runtime.capabilities.brightness_control)} />
          <Metric label="Processos" value={`${processes.length}`} ok />
        </div>

        <label className="slider-row neural-slider">
          <Volume2 size={18} />
          <input
            max={100}
            min={0}
            onChange={(event) => void onVolume(Number(event.target.value))}
            type="range"
            value={pcState?.volume ?? 50}
          />
          <span>{pcState?.volume ?? 50}%</span>
        </label>

        {runtime.capabilities.brightness_control && (
          <label className="slider-row neural-slider">
            <Sparkles size={18} />
            <input
              max={100}
              min={0}
              onChange={(event) => void onBrightness(Number(event.target.value))}
              type="range"
              value={pcState?.brightness ?? 0}
            />
            <span>{pcState?.brightness ?? 0}%</span>
          </label>
        )}

        <div className="control-grid">
          <button className="quick-card neural-card-button" onClick={() => void onScreenshot()} type="button">
            <Camera size={18} />
            <span>Capturar tela</span>
          </button>
          <button className="quick-card neural-card-button" onClick={() => void onRefresh()} type="button">
            <RefreshCw size={18} />
            <span>Atualizar processos</span>
          </button>
        </div>

        <div className="auth-box neural-data-box">
          <strong>Area de transferencia</strong>
          <span>{pcState?.clipboard_preview || "Sem texto detectado."}</span>
        </div>
      </HoloPanel>

      <HoloPanel>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Processos</p>
            <h2>Malha operacional</h2>
          </div>
          <Gauge size={22} />
        </div>
        <div className="process-list">
          {processes.map((process) => (
            <div className="process-row neural-row" key={`${process.pid}-${process.name}`}>
              <div>
                <strong>{process.name}</strong>
                <span>PID {process.pid} - {formatBytes(process.memory)}</span>
              </div>
              <NeuralButton
                className={process.critical ? "danger small" : "small"}
                onClick={() => void onCloseProcess(process)}
                type="button"
              >
                Encerrar
              </NeuralButton>
            </div>
          ))}
        </div>
      </HoloPanel>

      {runtime.capabilities.window_control && (
        <HoloPanel className="wide-panel">
          <div className="section-header compact">
            <div>
              <p className="eyebrow">Janelas</p>
              <h2>Controle holografico</h2>
            </div>
          </div>
          <div className="process-list">
            {(pcState?.windows ?? []).map((window) => (
              <div className="process-row window-row neural-row" key={window.id}>
                <div>
                  <strong>{window.title}</strong>
                  <span>{window.app}{window.focused ? " - ativa" : ""}</span>
                </div>
                <div className="button-row compact-buttons">
                  <NeuralButton className="small" onClick={() => void onWindowControl(window.id, "focus")} type="button">Focar</NeuralButton>
                  <NeuralButton className="small" onClick={() => void onWindowControl(window.id, "minimize")} type="button">Minimizar</NeuralButton>
                  <NeuralButton className="small" onClick={() => void onWindowControl(window.id, "maximize")} type="button">Maximizar</NeuralButton>
                </div>
              </div>
            ))}
            {!pcState?.windows.length && <EmptyState icon={Monitor} title="Sem janelas" text="Janelas abertas aparecem aqui." />}
          </div>
        </HoloPanel>
      )}
    </div>
  );
}

function AppsView({
  apps,
  favorites,
  onClose,
  onOpen,
}: {
  apps: InstalledApp[];
  favorites: InstalledApp[];
  onClose: (app: InstalledApp) => Promise<void>;
  onOpen: (app: InstalledApp) => Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const filtered = apps.filter((app) => app.name.toLowerCase().includes(query.toLowerCase()));

  return (
    <div className="settings-layout app-matrix">
      <HoloPanel glow>
        <div className="search-row neural-search">
          <Search size={18} />
          <input placeholder="Buscar aplicativo na matriz" value={query} onChange={(event) => setQuery(event.target.value)} />
          <span className="badge enabled">{filtered.length}</span>
        </div>
        <div className="app-grid">
          {filtered.map((app) => (
            <article className="app-tile neural-tile" key={app.id}>
              <AppWindow size={24} />
              <strong>{app.name}</strong>
              <span>{app.command}</span>
              <div className="button-row">
                <NeuralButton className="small" onClick={() => void onOpen(app)} type="button" variant="primary">Abrir</NeuralButton>
                <NeuralButton className="small danger" onClick={() => void onClose(app)} type="button">Fechar</NeuralButton>
              </div>
            </article>
          ))}
          {!filtered.length && (
            <EmptyState
              icon={AppWindow}
              title="Nenhum aplicativo detectado"
              text="Quando o backend nativo encontrar apps instalados, eles aparecem aqui."
            />
          )}
        </div>
      </HoloPanel>

      <HoloPanel>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Favoritos</p>
            <h2>Acesso instantaneo</h2>
          </div>
        </div>
        <div className="button-row">
          {favorites.map((app) => (
            <NeuralButton key={app.id} onClick={() => void onOpen(app)} type="button">
              <AppWindow size={16} />
              {app.name}
            </NeuralButton>
          ))}
          {!favorites.length && <span className="muted-copy">Apps favoritos detectados aparecem aqui.</span>}
        </div>
      </HoloPanel>
    </div>
  );
}

function MusicView({
  favorites,
  onDeleteFavorite,
  onFavoritePlaylist,
  onPause,
  onPlay,
  onPlayFavorite,
  onPlayLiked,
  onSearch,
  onSearchChange,
  onSettingsChange,
  onSkipBack,
  onSkipForward,
  onSpotifyAuth,
  playlists,
  search,
  settings,
  track,
}: {
  favorites: MusicFavorite[];
  playlists: SpotifyPlaylist[];
  search: string;
  settings: AssistantSettings;
  track: SpotifyTrack | null;
  onDeleteFavorite: (id: string) => Promise<void>;
  onFavoritePlaylist: (playlist: SpotifyPlaylist) => Promise<void>;
  onPause: () => void;
  onPlay: (uri: string) => void;
  onPlayFavorite: (favorite: MusicFavorite) => void;
  onPlayLiked: () => void;
  onSearch: () => Promise<void>;
  onSearchChange: (value: string) => void;
  onSettingsChange: (settings: AssistantSettings) => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
  onSpotifyAuth: () => Promise<void>;
}) {
  const playing = Boolean(track?.is_playing);
  return (
    <div className="spotify-layout neural-music">
      <HoloPanel className="player-panel" glow>
        <div className={`album-art neural-album ${playing ? "is-playing" : ""}`}>
          {track?.image_url ? <img alt="" src={track.image_url} /> : <Music size={54} />}
          <div className="album-energy" />
        </div>
        <div className="track-copy">
          <p className="eyebrow">Sonic reactor</p>
          <h2>{track?.name ?? "Spotify nao conectado"}</h2>
          <span>{track ? `${track.artist} - ${track.album ?? "Spotify"}` : "Conecte uma conta Spotify opcional."}</span>
        </div>
        <AudioWaveform active={playing} large />
        <div className="player-controls">
          <button className="round-button" onClick={onSkipBack} type="button">
            <SkipBack size={18} />
          </button>
          <button className="round-button primary" onClick={track?.is_playing ? onPause : () => track?.uri && onPlay(track.uri)} type="button">
            {track?.is_playing ? <Pause size={18} /> : <Play size={18} />}
          </button>
          <button className="round-button" onClick={onSkipForward} type="button">
            <SkipForward size={18} />
          </button>
        </div>
        <div className="quick-buttons">
          <NeuralButton onClick={onPlayLiked} type="button">
            <Star size={16} />
            Curtidas
          </NeuralButton>
          {track?.uri && (
            <NeuralButton
              onClick={() => void onFavoritePlaylist({
                id: track.id,
                name: track.name,
                description: track.artist,
                image_url: track.image_url,
                tracks_total: 1,
                uri: track.uri ?? "",
              })}
              type="button"
            >
              <Star size={16} />
              Favoritar atual
            </NeuralButton>
          )}
        </div>

        <div className="settings-grid music-auth-grid">
          <label>
            Spotify Client ID
            <input
              value={settings.spotify_client_id}
              onChange={(event) => onSettingsChange({ ...settings, spotify_client_id: event.target.value })}
            />
          </label>
          <label>
            Redirect URI
            <input
              value={settings.spotify_redirect_uri}
              onChange={(event) => onSettingsChange({ ...settings, spotify_redirect_uri: event.target.value })}
            />
          </label>
        </div>
        <NeuralButton className="wide" onClick={() => void onSpotifyAuth()} type="button" variant="primary">
          <LogIn size={17} />
          Conectar Spotify
        </NeuralButton>
      </HoloPanel>

      <HoloPanel className="library-panel">
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Biblioteca holografica</p>
            <h2>Favoritos e playlists</h2>
          </div>
        </div>
        <div className="favorite-music-list">
          <button className="playlist-item liked-item neural-tile" onClick={onPlayLiked} type="button">
            <div className="playlist-art">
              <Star size={24} />
            </div>
            <strong>Musicas curtidas</strong>
            <span>Spotify salvas</span>
          </button>
          {favorites.map((favorite) => (
            <div className="favorite-row neural-row" key={favorite.id}>
              <button className="favorite-main" onClick={() => onPlayFavorite(favorite)} type="button">
                <Star size={17} />
                <span>{favorite.name}</span>
              </button>
              <NeuralButton className="small danger" onClick={() => void onDeleteFavorite(favorite.id)} type="button">
                Remover
              </NeuralButton>
            </div>
          ))}
        </div>

        <div className="search-row neural-search">
          <Search size={18} />
          <input placeholder="Buscar playlist" value={search} onChange={(event) => onSearchChange(event.target.value)} />
          <NeuralButton onClick={() => void onSearch()} type="button" variant="primary">Buscar</NeuralButton>
        </div>
        <div className="playlist-grid">
          {playlists.map((playlist) => (
            <article className="playlist-item neural-tile" key={playlist.id}>
              <div className="playlist-art">
                {playlist.image_url ? <img alt="" src={playlist.image_url} /> : <ListChecks size={24} />}
              </div>
              <strong>{playlist.name}</strong>
              <span>{playlist.tracks_total} faixas</span>
              <div className="button-row compact-buttons">
                <NeuralButton className="small" onClick={() => onPlay(playlist.uri)} type="button">Tocar</NeuralButton>
                <NeuralButton className="small" onClick={() => void onFavoritePlaylist(playlist)} type="button">
                  <Star size={14} />
                  Salvar
                </NeuralButton>
              </div>
            </article>
          ))}
          {!playlists.length && <EmptyState icon={Music} title="Sem playlists carregadas" text="Busque uma playlist para ativar a biblioteca holografica." />}
        </div>
      </HoloPanel>
    </div>
  );
}

function VoiceView({
  onSave,
  onSettingsChange,
  onStart,
  onStop,
  runtime,
  settings,
  voiceState,
}: {
  runtime: RuntimeStatus | null;
  settings: AssistantSettings;
  voiceState: VoiceState | null;
  onSettingsChange: (settings: AssistantSettings) => void;
  onStart: () => Promise<void>;
  onStop: () => Promise<void>;
  onSave: () => void;
}) {
  const running = isVoiceActive(voiceState);
  const listeningLabel = voiceStateLabel(voiceState);
  const transcript = voiceState?.transcript ?? "";
  const error = voiceState?.error ?? runtime?.last_error ?? null;
  const canStartVoice = Boolean(voiceState?.engine_ready);
  const [micLevel, setMicLevel] = useState(0);
  const [micTesting, setMicTesting] = useState(false);
  const [micError, setMicError] = useState<string | null>(null);
  const micCleanupRef = useRef<(() => void) | null>(null);

  useEffect(() => () => micCleanupRef.current?.(), []);

  async function testMicrophone() {
    micCleanupRef.current?.();
    setMicError(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const AudioContextClass = window.AudioContext ?? window.webkitAudioContext;
      if (!AudioContextClass) {
        throw new Error("AudioContext nao esta disponivel neste ambiente.");
      }
      const audioContext = new AudioContextClass();
      const source = audioContext.createMediaStreamSource(stream);
      const analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      source.connect(analyser);
      const data = new Uint8Array(analyser.frequencyBinCount);
      let frame = 0;
      setMicTesting(true);

      const render = () => {
        analyser.getByteTimeDomainData(data);
        const peak = data.reduce((max, value) => Math.max(max, Math.abs(value - 128)), 0);
        setMicLevel(Math.min(100, Math.round((peak / 128) * 100)));
        frame = window.requestAnimationFrame(render);
      };
      frame = window.requestAnimationFrame(render);

      const stop = () => {
        window.cancelAnimationFrame(frame);
        source.disconnect();
        stream.getTracks().forEach((track) => track.stop());
        void audioContext.close();
        setMicTesting(false);
      };
      micCleanupRef.current = stop;
      window.setTimeout(() => {
        if (micCleanupRef.current === stop) {
          stop();
          micCleanupRef.current = null;
        }
      }, 5000);
    } catch (caught) {
      setMicTesting(false);
      setMicLevel(0);
      setMicError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  return (
    <div className="two-column voice-grid">
      <HoloPanel className={`listening-panel neural-listening ${running ? "is-listening" : ""}`} glow>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Interface vocal</p>
            <h2>{listeningLabel}</h2>
          </div>
          {running ? <Mic size={24} /> : <MicOff size={24} />}
        </div>

        <NeuralOrb active={running} label={voiceState?.engine_ready ? "VOX" : "OFF"} large />
        <AudioWaveform active={running || micTesting} level={micLevel} large />
        <p className="muted-copy">
          Palavra de ativacao: <strong>{settings.wake_word || "assistente"}</strong>. O estado mostrado aqui vem do motor local.
        </p>
        <div className="button-row">
          <NeuralButton disabled={running || !canStartVoice} onClick={() => void onStart()} type="button" variant="primary">
            <Mic size={17} />
            Comecar escuta
          </NeuralButton>
          <NeuralButton disabled={!running} onClick={() => void onStop()} type="button">
            Parar
          </NeuralButton>
        </div>

        <div className="transcript-box neural-data-box">
          <span className="badge">{transcript ? "texto detectado" : "aguardando fala"}</span>
          <p>{transcript || "A transcricao local aparece aqui quando o motor estiver instalado e captar fala."}</p>
        </div>

        <div className="transcript-box neural-data-box">
          <span className="badge">{micTesting ? "testando microfone" : "nivel do microfone"}</span>
          <div className="level-meter large" aria-label="Nivel do microfone">
            <span style={{ width: `${micLevel}%` }} />
          </div>
          <div className="button-row">
            <NeuralButton onClick={() => void testMicrophone()} type="button">
              Testar microfone
            </NeuralButton>
          </div>
          {micError && <p>{micError}</p>}
        </div>

        {error && (
          <div className="inline-notice neural-notice warn">
            <Activity size={16} />
            <span>{error}</span>
          </div>
        )}
      </HoloPanel>

      <HoloPanel className="form-panel">
        <h2>Protocolos de voz</h2>
        <label className="toggle-line neural-toggle">
          <input
            checked={settings.voice_enabled}
            onChange={(event) => onSettingsChange({ ...settings, voice_enabled: event.target.checked })}
            type="checkbox"
          />
          <span>Ativar microfone</span>
        </label>
        <label>
          Palavra de ativacao
          <input value={settings.wake_word} onChange={(event) => onSettingsChange({ ...settings, wake_word: event.target.value })} />
        </label>
        <label className="toggle-line neural-toggle">
          <input
            checked={settings.speech_reply_enabled}
            onChange={(event) => onSettingsChange({ ...settings, speech_reply_enabled: event.target.checked })}
            type="checkbox"
          />
          <span>Resposta falada</span>
        </label>
        <NeuralButton className="settings-save" onClick={onSave} type="button" variant="primary">
          <Save size={17} />
          Salvar voz
        </NeuralButton>
      </HoloPanel>
    </div>
  );
}

function AutomationsView({
  onRun,
  onSave,
  routines,
}: {
  routines: Routine[];
  onRun: (id: string) => Promise<void>;
  onSave: (routine: Routine) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [triggerKind, setTriggerKind] = useState<"manual" | "schedule" | "app_open">("manual");
  const [triggerValue, setTriggerValue] = useState("");
  const [actionType, setActionType] = useState<ActionSpec["type"]>("open_app");
  const [actionTarget, setActionTarget] = useState("");

  const actionNeedsTarget = actionType !== "take_screenshot";
  const canSave =
    Boolean(name.trim()) &&
    (!actionNeedsTarget || Boolean(actionTarget.trim())) &&
    (triggerKind !== "schedule" || /^([01]\d|2[0-3]):[0-5]\d$/.test(triggerValue.trim())) &&
    (triggerKind !== "app_open" || Boolean(triggerValue.trim()));

  async function handleSaveRoutine() {
    if (!canSave) return;
    const now = new Date().toISOString();
    const routine: Routine = {
      id: crypto.randomUUID(),
      name: name.trim(),
      enabled: true,
      trigger: {
        kind: triggerKind,
        label: triggerLabel(triggerKind, triggerValue),
        value: triggerKind === "manual" ? undefined : triggerValue.trim(),
        aliases: [],
      },
      actions: [buildRoutineAction(actionType, actionTarget)],
      created_at: now,
      updated_at: now,
    };
    await onSave(routine);
    setName("");
    setTriggerKind("manual");
    setTriggerValue("");
    setActionType("open_app");
    setActionTarget("");
  }

  return (
    <div className="two-column automations-grid">
      <HoloPanel glow>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Motor de rotinas</p>
            <h2>Automacoes prontas</h2>
          </div>
          <Workflow size={22} />
        </div>
        <div className="routine-list">
          {routines.map((routine) => (
            <div className="routine-item neural-row" key={routine.id}>
              <div>
                <strong>{routine.name}</strong>
                <span>{routine.actions.length} acoes - {routine.enabled ? "ativa" : "pausada"}</span>
              </div>
              <NeuralButton className="small" onClick={() => void onRun(routine.id)} type="button" variant="primary">
                Executar
              </NeuralButton>
            </div>
          ))}
          {!routines.length && <EmptyState icon={Workflow} title="Nenhuma rotina criada" text="Rotinas salvas pela IA aparecem neste motor." />}
        </div>
      </HoloPanel>
      <HoloPanel className="form-panel">
        <h2>Criar rotina</h2>
        <p className="muted-copy">
          Rotinas salvas entram no motor local e respeitam as permissoes ativas antes de executar.
        </p>
        <div className="settings-grid">
          <label>
            Nome
            <input placeholder="Ex: abrir foco" value={name} onChange={(event) => setName(event.target.value)} />
          </label>
          <label>
            Gatilho
            <select value={triggerKind} onChange={(event) => setTriggerKind(event.target.value as "manual" | "schedule" | "app_open")}>
              <option value="manual">Manual</option>
              <option value="schedule">Horario</option>
              <option value="app_open">App aberto</option>
            </select>
          </label>
          {triggerKind !== "manual" && (
            <label>
              Valor do gatilho
              <input
                placeholder={triggerKind === "schedule" ? "20:30" : "chrome"}
                value={triggerValue}
                onChange={(event) => setTriggerValue(event.target.value)}
              />
            </label>
          )}
          <label>
            Acao
            <select value={actionType} onChange={(event) => setActionType(event.target.value as ActionSpec["type"])}>
              <option value="open_app">Abrir aplicativo</option>
              <option value="open_url">Abrir URL</option>
              <option value="copy_text">Copiar texto</option>
              <option value="system_volume">Ajustar volume</option>
              <option value="brightness">Ajustar brilho</option>
              <option value="take_screenshot">Capturar tela</option>
              <option value="play_music">Tocar musica</option>
            </select>
          </label>
          {actionNeedsTarget && (
            <label>
              Alvo
              <input
                placeholder={routineTargetPlaceholder(actionType)}
                value={actionTarget}
                onChange={(event) => setActionTarget(event.target.value)}
              />
            </label>
          )}
        </div>
        <NeuralButton disabled={!canSave} onClick={() => void handleSaveRoutine()} type="button" variant="primary">
          <Save size={17} />
          Salvar rotina
        </NeuralButton>
        <div className="neural-orbit-card">
          <Layers size={28} />
          <span>Sequencias protegidas</span>
          <strong>{routines.length}</strong>
        </div>
      </HoloPanel>
    </div>
  );
}

function triggerLabel(kind: Routine["trigger"]["kind"], value: string) {
  if (kind === "schedule") return `Horario ${value.trim()}`;
  if (kind === "app_open") return `Ao abrir ${value.trim()}`;
  return "Manual";
}

function buildRoutineAction(actionType: ActionSpec["type"], target: string): ActionSpec {
  const cleanTarget = target.trim();
  const numericTarget = Number(cleanTarget);
  const percent = Number.isFinite(numericTarget) ? Math.max(0, Math.min(100, numericTarget)) : 50;
  const labels: Partial<Record<ActionSpec["type"], string>> = {
    open_app: `Abrir ${cleanTarget}`,
    open_url: "Abrir URL",
    copy_text: "Copiar texto",
    system_volume: `Volume ${percent}%`,
    brightness: `Brilho ${percent}%`,
    take_screenshot: "Capturar tela",
    play_music: `Tocar ${cleanTarget}`,
  };
  const argsByType: Partial<Record<ActionSpec["type"], Record<string, string | number | boolean | null>>> = {
    open_app: { app: cleanTarget },
    open_url: { url: cleanTarget },
    copy_text: { text: cleanTarget },
    system_volume: { level: percent },
    brightness: { level: percent },
    take_screenshot: {},
    play_music: { query: cleanTarget },
  };

  return {
    type: actionType,
    label: labels[actionType] ?? actionType,
    target: actionType === "take_screenshot" ? "screen" : cleanTarget,
    args: argsByType[actionType] ?? {},
    risk: "low",
    requires_confirmation: false,
    reason: "Rotina criada pela interface.",
  };
}

function routineTargetPlaceholder(actionType: ActionSpec["type"]) {
  const placeholders: Partial<Record<ActionSpec["type"], string>> = {
    open_app: "chrome",
    open_url: "https://www.google.com",
    copy_text: "texto para copiar",
    system_volume: "40",
    brightness: "60",
    play_music: "playlist foco",
  };
  return placeholders[actionType] ?? "alvo";
}

function HistoryView({
  history,
  logs,
  onClear,
}: {
  history: CommandHistoryEntry[];
  logs: LogEntry[];
  onClear: () => Promise<void>;
}) {
  return (
    <div className="settings-layout neural-history">
      <HoloPanel glow>
        <div className="section-header">
          <div>
            <p className="eyebrow">Memoria neural</p>
            <h2>Timeline de comandos</h2>
          </div>
          <NeuralButton className="danger" onClick={() => void onClear()} type="button">
            <Trash2 size={16} />
            Limpar
          </NeuralButton>
        </div>
        <div className="log-list neural-timeline">
          {history.map((item) => (
            <HistoryRow item={item} key={item.id} />
          ))}
          {!history.length && <EmptyState icon={History} title="Memoria vazia" text="Use o assistente para gerar comandos." />}
        </div>
      </HoloPanel>

      <HoloPanel>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Logs tecnicos</p>
            <h2>Eventos do nucleo</h2>
          </div>
        </div>
        <div className="log-list">
          {logs.slice(0, 12).map((log) => (
            <div className={`log-item neural-row ${log.level}`} key={log.id}>
              <span className="badge">{log.level}</span>
              <div>
                <strong>{log.module}</strong>
                <span>{log.message}</span>
              </div>
              <time>{new Date(log.created_at).toLocaleString("pt-BR")}</time>
            </div>
          ))}
        </div>
      </HoloPanel>
    </div>
  );
}

function SettingsView({
  account,
  memories,
  onDeleteMemory,
  onGoogle,
  onGoogleLogout,
  onSave,
  onSaveMemory,
  onSetLocalAiPath,
  onSettingsChange,
  settings,
  status,
}: {
  account: AccountStatus | null;
  memories: MemoryEntry[];
  settings: AssistantSettings;
  status: AppStatus | null;
  onDeleteMemory: (id: string) => Promise<void>;
  onGoogle: () => Promise<void>;
  onGoogleLogout: () => Promise<void>;
  onSave: () => void;
  onSaveMemory: (title: string, content: string, category: string) => Promise<void>;
  onSettingsChange: (settings: AssistantSettings) => void;
  onSetLocalAiPath: (path: string) => Promise<void>;
}) {
  const [memoryDraft, setMemoryDraft] = useState("");
  return (
    <div className="settings-layout neural-settings">
      <HoloPanel className="settings-grid">
        <label>
          Nome do assistente
          <input value={settings.assistant_name} onChange={(event) => onSettingsChange({ ...settings, assistant_name: event.target.value })} />
        </label>
        <label className="toggle-line neural-toggle">
          <input
            checked={settings.start_with_windows}
            onChange={(event) => onSettingsChange({ ...settings, start_with_windows: event.target.checked })}
            type="checkbox"
          />
          <span>Iniciar com Windows</span>
        </label>
        <label className="toggle-line neural-toggle">
          <input
            checked={settings.minimize_to_tray}
            onChange={(event) => onSettingsChange({ ...settings, minimize_to_tray: event.target.checked })}
            type="checkbox"
          />
          <span>Minimizar para bandeja</span>
        </label>
      </HoloPanel>

      <HoloPanel glow>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">IA local</p>
            <h2>{status?.local_ai_model ?? "Qwen3-8B-Q5_0 embutido"}</h2>
          </div>
          <StatusPill ok={Boolean(status?.local_ai_ready)} label={localAiStatusLabel(status)} />
        </div>
        <div className="settings-grid">
          <label>
            Modelo local ativo
            <input readOnly value={settings.local_ai_path || "resources/models/Qwen3-8B-Q5_0.gguf"} />
          </label>
          <label>
            Modo avancado
            <select
              value={settings.local_ai_provider}
              onChange={(event) => onSettingsChange({ ...settings, local_ai_provider: event.target.value as AssistantSettings["local_ai_provider"] })}
            >
              <option value="qwen_embedded">Qwen local embutido</option>
              <option value="rules">Fallback de regras</option>
            </select>
          </label>
        </div>
        <label>
          Caminho do modelo GGUF
          <input
            placeholder="C:\\Users\\voce\\Downloads\\Qwen3-8B-Q5_0.gguf"
            value={settings.local_ai_path ?? ""}
            onChange={(event) => onSettingsChange({ ...settings, local_ai_path: event.target.value || null })}
          />
        </label>
        {status?.local_ai_error && <div className="inline-notice neural-notice warn">{status.local_ai_error}</div>}
        <div className="button-row">
          <NeuralButton onClick={onSave} type="button" variant="primary">
            <Save size={17} />
            Salvar IA local
          </NeuralButton>
          <NeuralButton onClick={() => onSettingsChange({ ...settings, local_ai_provider: "rules" })} type="button">
            Usar fallback de regras
          </NeuralButton>
          <NeuralButton onClick={() => void onSetLocalAiPath(settings.local_ai_path ?? "")} type="button">
            Aplicar caminho GGUF
          </NeuralButton>
        </div>
      </HoloPanel>

      <HoloPanel>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Permissoes salvas</p>
            <h2>Protocolos protegidos</h2>
          </div>
        </div>
        <div className="permission-grid">
          {Object.entries(settings.permissions).map(([key, value]) => (
            <label className="toggle-line neural-toggle" key={key}>
              <input
                checked={value}
                onChange={(event) =>
                  onSettingsChange({
                    ...settings,
                    permissions: { ...settings.permissions, [key]: event.target.checked },
                  })
                }
                type="checkbox"
              />
              <span>{permissionLabel(key)}</span>
            </label>
          ))}
        </div>
      </HoloPanel>

      <HoloPanel className="memory-panel">
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Memoria da IA</p>
            <h2>Preferencias persistentes</h2>
          </div>
          <span className="badge enabled">{memories.length}</span>
        </div>
        <div className="search-row neural-search">
          <Save size={18} />
          <input
            placeholder="Ex: gosto de playlist foco para estudar"
            value={memoryDraft}
            onChange={(event) => setMemoryDraft(event.target.value)}
          />
          <NeuralButton
            disabled={!memoryDraft.trim()}
            onClick={async () => {
              await onSaveMemory("", memoryDraft, "preferencia");
              setMemoryDraft("");
            }}
            type="button"
            variant="primary"
          >
            Salvar
          </NeuralButton>
        </div>
        <div className="memory-list">
          {memories.slice(0, 8).map((memory) => (
            <div className="memory-row neural-row" key={memory.id}>
              <div>
                <strong>{memory.title}</strong>
                <span>{memory.content}</span>
              </div>
              <NeuralButton className="small danger" onClick={() => void onDeleteMemory(memory.id)} type="button">
                Remover
              </NeuralButton>
            </div>
          ))}
          {!memories.length && <EmptyState icon={Database} title="Sem memorias" text="Salve preferencias para a IA usar nos comandos." />}
        </div>
      </HoloPanel>

      <HoloPanel>
        <div className="section-header compact">
          <div>
            <p className="eyebrow">Google opcional</p>
            <h2>Sincronizacao futura</h2>
          </div>
        </div>
        {account?.google_connected ? (
          <div className="account-card neural-row">
            {account.profile?.picture ? <img alt="" src={account.profile.picture} /> : <UserCircle size={36} />}
            <div>
              <strong>{account.profile?.name}</strong>
              <span>{account.profile?.email}</span>
            </div>
            <NeuralButton onClick={() => void onGoogleLogout()} type="button">
              <LogOut size={16} />
              Sair
            </NeuralButton>
          </div>
        ) : (
          <NeuralButton onClick={() => void onGoogle()} type="button">
            <UserCircle size={17} />
            Conectar Google
          </NeuralButton>
        )}
      </HoloPanel>

      <NeuralButton className="settings-save" onClick={onSave} type="button" variant="primary">
        <Save size={17} />
        Salvar configuracoes
      </NeuralButton>
    </div>
  );
}

function ChatBubble({
  message,
  onConfirmAction,
}: {
  message: ChatMessage;
  onConfirmAction: (action: ActionSpec) => Promise<void>;
}) {
  return (
    <article className={`message neural-message ${message.role}`}>
      <div className="message-avatar">{message.role === "user" ? <UserCircle size={18} /> : <Bot size={18} />}</div>
      <div className="message-body">
        <p>{message.content}</p>
        {!!message.tool_calls?.length && (
          <div className="message-actions">
            {message.tool_calls.map((action) => (
              <ActionCard action={action} key={`${message.id}-${action.label}`} onConfirm={onConfirmAction} />
            ))}
          </div>
        )}
      </div>
    </article>
  );
}

function HoloPanel({
  children,
  className = "",
  glow = false,
}: {
  children: ReactNode;
  className?: string;
  glow?: boolean;
}) {
  return (
    <motion.section
      className={`panel holo-panel ${glow ? "glow" : ""} ${className}`}
      whileHover={{ y: -2 }}
      transition={{ duration: 0.22 }}
    >
      {children}
    </motion.section>
  );
}

function NeuralButton({
  children,
  className = "",
  variant = "secondary",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" }) {
  return (
    <button className={`neural-button ${variant} ${className}`} {...props}>
      {children}
    </button>
  );
}

function NeuralOrb({
  active,
  label,
  large = false,
}: {
  active: boolean;
  label: string;
  large?: boolean;
}) {
  return (
    <div className={`neural-orb ${active ? "active" : ""} ${large ? "large" : ""}`}>
      <div className="orb-shell" />
      <div className="orb-core">
        <Sparkles size={large ? 38 : 24} />
        <span>{label}</span>
      </div>
      <div className="orb-ring ring-a" />
      <div className="orb-ring ring-b" />
      <div className="orb-ring ring-c" />
    </div>
  );
}

function AudioWaveform({ active, large = false, level = 48 }: { active: boolean; large?: boolean; level?: number }) {
  return (
    <div className={`audio-waveform ${active ? "active" : ""} ${large ? "large" : ""}`} aria-hidden>
      {waveformBars.map((height, index) => {
        const dynamicHeight = active ? Math.min(96, Math.max(18, height + level * 0.28)) : height * 0.38;
        return (
          <span
            key={`${height}-${index}`}
            style={{ "--bar-height": `${dynamicHeight}%`, "--bar-delay": `${index * 46}ms` } as CSSProperties}
          />
        );
      })}
    </div>
  );
}

function StatusPill({ ok, label }: { ok: boolean; label: string }) {
  return (
    <div className={`status-pill neural-status-pill ${ok ? "ok" : ""}`}>
      {ok ? <CheckCircle2 size={15} /> : <XCircle size={15} />}
      <span>{label}</span>
    </div>
  );
}

function Metric({ label, ok, value }: { label: string; ok: boolean; value: string }) {
  return (
    <div className={`metric-card hud-metric ${ok ? "ok" : ""}`}>
      <span>{label}</span>
      <strong>{value}</strong>
      <em>{ok ? "ONLINE" : "IDLE"}</em>
    </div>
  );
}

function ActionCard({ action, onConfirm }: { action: ActionSpec; onConfirm: (action: ActionSpec) => Promise<void> }) {
  return (
    <div className={`action-card neural-action ${action.risk}`}>
      <div className="action-icon">
        {action.risk === "high" ? <ShieldCheck size={18} /> : <CheckCircle2 size={18} />}
      </div>
      <div>
        <strong>{action.label}</strong>
        <span>{action.reason || action.target}</span>
      </div>
      <span className="badge">{riskLabel(action.risk)}</span>
      <NeuralButton className="small" onClick={() => void onConfirm(action)} type="button" variant="primary">
        Confirmar
      </NeuralButton>
    </div>
  );
}

function HistoryRow({ item }: { item: CommandHistoryEntry }) {
  return (
    <div className={`log-item history-row neural-row ${item.status}`}>
      <span className="badge">{item.status}</span>
      <div>
        <strong>Pedido: {item.command}</strong>
        <span>Decisao: {item.assistant_reply || "Sem resposta registrada."}</span>
        <span>Resultado: {item.result_summary || item.source}</span>
        {!!item.actions.length && (
          <div className="history-actions">
            {item.actions.map((action) => (
              <em key={`${item.id}-${action.label}`}>Acao: {action.label}</em>
            ))}
          </div>
        )}
      </div>
      <time>{new Date(item.created_at).toLocaleString("pt-BR")}</time>
    </div>
  );
}

function EmptyState({ icon: Icon, text, title }: { icon: LucideIcon; text: string; title: string }) {
  return (
    <div className="empty-state neural-empty">
      <div className="empty-orbit">
        <Icon size={30} />
      </div>
      <strong>{title}</strong>
      <span>{text}</span>
    </div>
  );
}

function deriveAssistantUiState(
  voiceState: VoiceState | null,
  activity: AssistantActivity | null,
  runtime: RuntimeStatus | null,
): AssistantUiState {
  const webVoiceFallback = runtime?.mode !== "native" && voiceState?.error;
  if ((voiceState?.state === "error" || voiceState?.error) && !webVoiceFallback) return "error";
  if (isVoiceActive(voiceState)) return "listening";
  return activity?.state ?? "idle";
}

function getAssistantStateCopy(
  state: AssistantUiState,
  voiceState: VoiceState | null,
  status: AppStatus | null,
  runtime: RuntimeStatus | null,
  activity: AssistantActivity | null,
): AssistantStateCopy {
  if (state === "listening") {
    const title = voiceState?.state === "transcribing" ? "Transcrevendo sua voz" : "Estou ouvindo";
    return {
      detail: voiceState?.transcript?.trim()
        ? `Transcricao parcial: ${voiceState.transcript}`
        : "Microfone ativo. A IA so mostra escuta quando o motor local confirma o estado.",
      orbLabel: voiceState?.state === "transcribing" ? "TEXT" : "LISTEN",
      title,
    };
  }

  if (state === "thinking") {
    return {
      detail: activity?.detail ?? "Mensagem enviada. Aguardando retorno real do modelo.",
      orbLabel: "THINK",
      title: "Pensando na resposta",
    };
  }

  if (state === "executing") {
    return {
      detail: activity?.detail ?? "Executando uma acao confirmada.",
      orbLabel: "ACT",
      title: "Executando comando",
    };
  }

  if (state === "success") {
    return {
      detail: activity?.detail ?? "Operacao concluida com retorno real do sistema.",
      orbLabel: "DONE",
      title: "Resposta pronta",
    };
  }

  if (state === "error") {
    return {
      detail:
        voiceState?.error ??
        activity?.detail ??
        status?.last_error ??
        runtime?.last_error ??
        "Um erro real foi capturado pelo sistema.",
      orbLabel: "ERR",
      title: "Preciso de atencao",
    };
  }

  if (state === "speaking") {
    return {
      detail: "Resposta falada confirmada pelo sistema de voz.",
      orbLabel: "TALK",
      title: "Falando agora",
    };
  }

  if (status?.local_ai_ready) {
    return {
      detail: voiceState?.engine_ready
        ? `Toque em Falar, diga "${voiceState.wake_word}" ou digite um comando natural.`
        : "A IA local esta pronta. A voz fica disponivel quando o microfone local estiver ativo.",
      orbLabel: "AI",
      title: "Pronta para conversar",
    };
  }

  return {
    detail:
      runtime?.mode === "native"
        ? "A interface esta pronta, mas a IA local ainda nao confirmou disponibilidade."
        : "Modo visual no navegador. Comandos reais dependem do app nativo.",
    orbLabel: "IDLE",
    title: "Assistente em espera",
  };
}

function permissionLabel(key: string) {
  const labels: Record<string, string> = {
    apps: "Aplicativos",
    windows: "Janelas",
    clipboard: "Area de transferencia",
    screenshots: "Capturas de tela",
    processes: "Processos",
    power: "Energia",
    voice: "Voz",
  };
  return labels[key] ?? key;
}

function localAiStatusLabel(status: AppStatus | null) {
  if (!status) return "IA carregando";
  if (status.local_ai_ready) return "Qwen online";
  const labels: Record<string, string> = {
    loading: "Qwen iniciando",
    stopped: "Qwen parado",
    model_missing: "Modelo ausente",
    runtime_missing: "Runtime ausente",
    error: "Falha na IA",
  };
  return labels[status.local_ai_state] ?? "IA offline";
}

function localAiShortLabel(status: AppStatus | null) {
  if (!status) return "Carregando";
  if (status.local_ai_ready) return "Qwen pronto";
  if (status.local_ai_state === "loading") return "Carregando";
  if (status.local_ai_state === "model_missing") return "Sem modelo";
  if (status.local_ai_state === "runtime_missing") return "Sem runtime";
  return "Offline";
}

function voiceStateLabel(voiceState: VoiceState | null) {
  if (!voiceState) return "Carregando voz";
  const labels: Record<string, string> = {
    idle: "Microfone em espera",
    waiting_wake_word: "Aguardando palavra de ativacao",
    listening: "Escutando agora",
    transcribing: "Transcrevendo",
    error: "Escuta indisponivel",
  };
  return labels[voiceState.state] ?? "Estado de voz desconhecido";
}

function riskLabel(risk: ActionSpec["risk"]) {
  const labels: Record<ActionSpec["risk"], string> = {
    low: "baixo",
    medium: "medio",
    high: "alto",
  };
  return labels[risk];
}

function isVoiceActive(voiceState: VoiceState | null) {
  return voiceState?.state === "waiting_wake_word" || voiceState?.state === "listening" || voiceState?.state === "transcribing";
}

function formatBytes(value: number) {
  if (!value) return "0 MB";
  return `${Math.max(1, Math.round(value / 1024 / 1024))} MB`;
}

declare global {
  interface Window {
    webkitAudioContext?: typeof AudioContext;
    SpeechRecognition?: SpeechRecognitionConstructor;
    webkitSpeechRecognition?: SpeechRecognitionConstructor;
  }
}

type SpeechRecognitionConstructor = new () => SpeechRecognitionLike;

interface SpeechRecognitionLike {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  onerror: ((event: { error?: string }) => void) | null;
  onresult: ((event: SpeechRecognitionEventLike) => void) | null;
  start: () => void;
  stop: () => void;
}

interface SpeechRecognitionEventLike {
  results: ArrayLike<ArrayLike<{ transcript: string }>>;
}

export default App;
