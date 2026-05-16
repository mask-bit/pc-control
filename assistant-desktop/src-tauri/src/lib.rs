mod actions;
mod automation;
mod google;
mod local_ai;
mod music;
mod models;
mod secrets;
mod spotify;
mod storage;
mod system_control;
mod voice;

use crate::models::{
    AccountStatus, ActionSpec, AppStatus, AssistantSettings, ChatMessage, ChatResult,
    CommandHistoryEntry, GoogleProfile, InstalledApp, LocalAiStatus, LogEntry, PcControlState,
    MemoryEntry, MusicCommandResult, MusicFavorite, ProcessInfo, Routine, RuntimeCapabilities,
    RuntimeStatus, SpotifyPlaylist, SpotifyTrack, TaskState, VoiceState,
};
use crate::spotify::SpotifyAuthState;
use crate::storage::Database;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

pub struct AppState {
    db: Arc<Database>,
    spotify_auth: SpotifyAuthState,
    local_ai_runtime: local_ai::LlamaRuntimeState,
    voice_runtime: voice::VoiceRuntimeState,
}

#[tauri::command]
async fn get_app_status(app: AppHandle, state: State<'_, AppState>) -> Result<AppStatus, String> {
    let settings = state.db.get_settings()?;
    let ai = local_ai::status(&app, &state.local_ai_runtime, &settings, true).await;
    let voice = voice::get_state(&app, &state.voice_runtime, &settings);
    let spotify_connected = spotify::has_refresh_token();
    let last_error = state
        .db
        .list_logs()?
        .into_iter()
        .find(|log| log.level == "error")
        .map(|log| log.message);

    Ok(AppStatus {
        onboarding_complete: state.db.is_onboarding_complete()?,
        local_ai_ready: ai.ready,
        local_ai_provider: ai.provider,
        local_ai_model: ai.model,
        local_ai_state: ai.state,
        local_ai_error: ai.error,
        microphone_ready: voice.engine_ready && voice.microphone_ready,
        spotify_connected,
        spotify_device_active: spotify_connected,
        system_status: "online".to_string(),
        last_error,
    })
}

#[tauri::command]
async fn get_runtime_status(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let settings = state.db.get_settings()?;
    let ai = local_ai::status(&app, &state.local_ai_runtime, &settings, false).await;
    let voice = voice::get_state(&app, &state.voice_runtime, &settings);
    let last_error = state
        .db
        .list_logs()?
        .into_iter()
        .find(|log| log.level == "error")
        .map(|log| log.message);
    Ok(RuntimeStatus {
        mode: "native".to_string(),
        capabilities: RuntimeCapabilities {
            pc_control: true,
            app_launch: true,
            spotify_app: system_control::spotify_installed(),
            spotify_api: spotify::has_refresh_token(),
            spotify_desktop_automation: system_control::spotify_installed(),
            local_ai: ai.ready,
            local_voice: voice.engine_ready,
            brightness_control: system_control::brightness_supported(),
            window_control: system_control::window_control_supported(),
        },
        last_error,
    })
}

#[tauri::command]
fn complete_onboarding(
    state: State<AppState>,
    settings: AssistantSettings,
) -> Result<AssistantSettings, String> {
    state.db.save_settings(&settings)?;
    state.db.set_onboarding_complete(true)?;
    let log = make_log("onboarding", "Primeiro acesso concluido.", "info");
    state.db.add_log(&log)?;
    Ok(settings)
}

#[tauri::command]
async fn local_ai_status(app: AppHandle, state: State<'_, AppState>) -> Result<LocalAiStatus, String> {
    let settings = state.db.get_settings()?;
    Ok(local_ai::status(&app, &state.local_ai_runtime, &settings, true).await)
}

#[tauri::command]
fn set_local_ai_path(state: State<AppState>, path: String) -> Result<LocalAiStatus, String> {
    let mut settings = state.db.get_settings()?;
    settings.local_ai_provider = "qwen_embedded".to_string();
    settings.local_ai_path = if path.trim().is_empty() {
        None
    } else {
        Some(path.trim().to_string())
    };
    state.db.save_settings(&settings)?;
    Ok(LocalAiStatus {
        ready: true,
        provider: settings.local_ai_provider,
        model: "Qwen3-8B-Q5_0 embutido".to_string(),
        state: "settings_saved".to_string(),
        error: None,
    })
}

#[tauri::command]
fn list_messages(state: State<AppState>) -> Result<Vec<ChatMessage>, String> {
    state.db.list_messages()
}

#[tauri::command]
async fn send_chat_message(
    app: AppHandle,
    state: State<'_, AppState>,
    content: String,
    mode: String,
) -> Result<ChatResult, String> {
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: content.clone(),
        created_at: Utc::now().to_rfc3339(),
        tool_calls: None,
        status: Some("done".to_string()),
    };
    state.db.save_message(&user_message)?;

    let settings = state.db.get_settings()?;
    let history = state.db.list_messages()?;
    let memories = state.db.list_memories()?;
    let task_state = state.db.get_task_state()?;
    let local_result = match local_ai::send_chat(
        &app,
        &state.local_ai_runtime,
        settings.clone(),
        history,
        memories,
        task_state,
        content.clone(),
        mode,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            let log = make_log("local_ai", &format!("Falha na IA local: {error}"), "error");
            state.db.add_log(&log)?;
            local_ai::LocalAiOutput {
                assistant_reply: format!("A IA local configurada falhou: {error}. Volte para o provedor de regras ou confira o arquivo externo."),
                actions: Vec::new(),
            }
        }
    };

    let mut executed = Vec::new();
    let mut blocked = Vec::new();
    let mut failed = Vec::new();
    let mut blocked_actions = Vec::new();

    for action in local_result.actions.clone() {
        let action = actions::sanitize_action(action, true);
        if let Some(reason) = actions::permission_denial(&action, &settings) {
            blocked.push(format!("{} ({reason})", action.label));
            blocked_actions.push(action);
            continue;
        }
        if let Some(reason) = actions::protected_reason(&action) {
            blocked.push(format!("{} ({reason})", action.label));
            blocked_actions.push(action);
            continue;
        }

        match execute_action_impl(&app, &state, action.clone()).await {
            Ok(log) => {
                state.db.add_log(&log)?;
                executed.push(log.message);
            }
            Err(error) => {
                let log = make_log("executor", &format!("Falha em {}: {error}", action.label), "error");
                state.db.add_log(&log)?;
                failed.push(format!("{} ({error})", action.label));
            }
        }
    }

    update_task_state(&state, &content, &executed, &blocked, &failed)?;
    let content = compose_autonomous_reply(&local_result.assistant_reply, &executed, &blocked, &failed);
    let assistant_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content,
        created_at: Utc::now().to_rfc3339(),
        tool_calls: if blocked_actions.is_empty() {
            None
        } else {
            Some(blocked_actions.clone())
        },
        status: Some(if blocked_actions.is_empty() && failed.is_empty() {
            "done".to_string()
        } else {
            "blocked".to_string()
        }),
    };
    state.db.save_message(&assistant_message)?;
    state.db.save_history_entry(&CommandHistoryEntry {
        id: Uuid::new_v4().to_string(),
        command: user_message.content.clone(),
        actions: local_result.actions.clone(),
        assistant_reply: assistant_message.content.clone(),
        result_summary: compose_result_summary(&executed, &blocked, &failed),
        status: if failed.is_empty() && blocked.is_empty() {
            "done".to_string()
        } else if !failed.is_empty() {
            "error".to_string()
        } else {
            "blocked".to_string()
        },
        source: "chat".to_string(),
        created_at: Utc::now().to_rfc3339(),
    })?;
    if settings.speech_reply_enabled {
        if let Err(error) = system_control::speak_text(&assistant_message.content) {
            state
                .db
                .add_log(&make_log("speech", &format!("Falha ao falar resposta: {error}"), "warn"))?;
        }
    }

    Ok(ChatResult {
        message: assistant_message,
        proposed_actions: blocked_actions,
    })
}

#[tauri::command]
async fn execute_action(
    app: AppHandle,
    state: State<'_, AppState>,
    action: ActionSpec,
) -> Result<LogEntry, String> {
    let settings = state.db.get_settings()?;
    if let Some(reason) = actions::permission_denial(&action, &settings) {
        return Err(format!("Acao bloqueada por permissao: {reason}."));
    }
    let log = execute_action_impl(&app, &state, action).await?;
    state.db.add_log(&log)?;
    Ok(log)
}

async fn execute_action_impl(
    app: &AppHandle,
    state: &State<'_, AppState>,
    action: ActionSpec,
) -> Result<LogEntry, String> {
    let maybe_log = match action.action_type.as_str() {
        "play_music" | "spotify_play" => {
            let uri = action
                .args
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| action.args.get("query").and_then(Value::as_str).map(str::to_string))
                .ok_or("URI ou busca Spotify ausente.")?;
            let result = if is_liked_music_query(&uri) {
                music::play_liked_music(&state.db).await?
            } else if let Some(favorite) = state.db.find_music_favorite(&uri)? {
                if let Some(favorite_uri) = favorite.uri {
                    music::play_music(&state.db, &favorite_uri).await?
                } else if let Some(query) = favorite.query {
                    music::play_music(&state.db, &query).await?
                } else {
                    return Err("Favorito musical sem URI ou busca.".to_string());
                }
            } else {
                music::play_music(&state.db, &uri).await?
            };
            Some(make_music_log("spotify", &result))
        }
        "play_liked_music" | "spotify_liked" => {
            let result = music::play_liked_music(&state.db).await?;
            Some(make_music_log("spotify", &result))
        }
        "pause_music" | "spotify_pause" => {
            let result = music::pause_music(&state.db).await?;
            Some(make_music_log("spotify", &result))
        }
        "next_track" | "spotify_next" => {
            let result = music::next_track(&state.db).await?;
            Some(make_music_log("spotify", &result))
        }
        "previous_track" | "spotify_previous" => {
            let result = music::previous_track(&state.db).await?;
            Some(make_music_log("spotify", &result))
        }
        "spotify_volume" => {
            let volume = action.args.get("volume").and_then(Value::as_u64).unwrap_or(35) as u8;
            spotify::set_volume(&state.db, volume).await?;
            Some(make_log("spotify", &format!("Volume Spotify em {}%", volume), "info"))
        }
        "run_routine" => {
            let id = action
                .args
                .get("id")
                .and_then(Value::as_str)
                .ok_or("ID de rotina ausente.")?;
            Some(run_routine_inner(app, state, id).await?)
        }
        "save_memory" => {
            let content = action
                .args
                .get("content")
                .and_then(Value::as_str)
                .or_else(|| action.args.get("text").and_then(Value::as_str))
                .unwrap_or(&action.target)
                .trim()
                .to_string();
            if content.is_empty() {
                return Err("Memoria vazia.".to_string());
            }
            let title = action
                .args
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| memory_title(&content));
            let category = action
                .args
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("geral")
                .to_string();
            let now = Utc::now().to_rfc3339();
            let memory = MemoryEntry {
                id: Uuid::new_v4().to_string(),
                title,
                content,
                category,
                pinned: true,
                created_at: now.clone(),
                updated_at: now,
            };
            state.db.save_memory(&memory)?;
            Some(make_log("memory", &format!("Memoria salva: {}", memory.title), "info"))
        }
        "save_music_favorite" => {
            let name = action
                .args
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&action.target)
                .trim()
                .to_string();
            let uri = action
                .args
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_string);
            let query = action
                .args
                .get("query")
                .and_then(Value::as_str)
                .map(str::to_string);
            let kind = action
                .args
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("playlist")
                .to_string();
            if name.is_empty() || (uri.is_none() && query.is_none()) {
                return Err("Favorito musical incompleto.".to_string());
            }
            let favorite = MusicFavorite {
                id: Uuid::new_v4().to_string(),
                name,
                uri,
                query,
                kind,
                created_at: Utc::now().to_rfc3339(),
            };
            state.db.save_music_favorite(&favorite)?;
            persist_music_memory(&state.db, &favorite)?;
            Some(make_log(
                "spotify",
                &format!("Favorito musical salvo: {}", favorite.name),
                "info",
            ))
        }
        "system_volume" => {
            let level = action.args.get("level").and_then(Value::as_u64).unwrap_or(50).min(100) as u8;
            system_control::set_system_volume(level)?;
            let mut settings = state.db.get_settings()?;
            settings.system_volume = level;
            state.db.save_settings(&settings)?;
            Some(make_log("pc_control", &format!("Volume do Windows ajustado para {}%.", level), "info"))
        }
        "brightness" => {
            let level = action.args.get("level").and_then(Value::as_u64).unwrap_or(50).min(100) as u8;
            system_control::set_brightness(level)?;
            Some(make_log("pc_control", &format!("Brilho ajustado para {}%.", level), "info"))
        }
        "take_screenshot" => {
            let path = system_control::take_screenshot(app)?;
            Some(make_log("pc_control", &format!("Screenshot salvo em {path}"), "info"))
        }
        "list_processes" => Some(make_log("pc_control", "Processos atualizados na tela Controle do PC.", "info")),
        "close_process" => {
            let pid = action
                .args
                .get("pid")
                .and_then(Value::as_u64)
                .ok_or("PID ausente.")? as u32;
            system_control::close_process(pid)?;
            Some(make_log("pc_control", &format!("Processo {pid} encerrado."), "info"))
        }
        "power" => {
            let operation = action
                .args
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or(&action.target);
            system_control::power(operation)?;
            Some(make_log("pc_control", &format!("Energia: {operation} solicitado."), "warn"))
        }
        "window_focus" => {
            let id = action
                .args
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(&action.target);
            system_control::focus_window(id)?;
            Some(make_log("windows", &format!("Janela focada: {}", action.label), "info"))
        }
        "window_minimize" => {
            let id = action
                .args
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(&action.target);
            system_control::minimize_window(id)?;
            Some(make_log("windows", &format!("Janela minimizada: {}", action.label), "info"))
        }
        "window_maximize" => {
            let id = action
                .args
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(&action.target);
            system_control::maximize_window(id)?;
            Some(make_log("windows", &format!("Janela maximizada: {}", action.label), "info"))
        }
        "open_app" => {
            let target = action
                .args
                .get("app")
                .and_then(Value::as_str)
                .unwrap_or(&action.target);
            system_control::open_app(target)?;
            Some(make_log("apps", &format!("Aplicativo aberto: {}", action.label), "info"))
        }
        "close_app" => {
            let target = action
                .args
                .get("app")
                .and_then(Value::as_str)
                .unwrap_or(&action.target);
            system_control::close_app(target)?;
            Some(make_log("apps", &format!("Aplicativo fechado: {}", action.label), "info"))
        }
        _ => actions::execute_basic_action(&action)?,
    };

    Ok(maybe_log.unwrap_or_else(|| make_log("executor", "Acao nao implementada na V1.", "warn")))
}

#[tauri::command]
fn list_routines(state: State<AppState>) -> Result<Vec<Routine>, String> {
    state.db.list_routines()
}

#[tauri::command]
fn save_routine(state: State<AppState>, routine: Routine) -> Result<Routine, String> {
    state.db.save_routine(&routine)?;
    Ok(routine)
}

#[tauri::command]
async fn run_routine(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<LogEntry, String> {
    let log = run_routine_inner(&app, &state, &id).await?;
    state.db.add_log(&log)?;
    Ok(log)
}

async fn run_routine_inner(
    app: &AppHandle,
    state: &State<'_, AppState>,
    id: &str,
) -> Result<LogEntry, String> {
    let routines = state.db.list_routines()?;
    let routine = routines
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("Rotina nao encontrada.")?;

    for action in routine.actions {
        let settings = state.db.get_settings()?;
        if let Some(reason) = actions::permission_denial(&action, &settings) {
            return Err(format!("Rotina bloqueada por permissao: {reason}."));
        }
        let log = Box::pin(execute_action_impl(app, state, action)).await?;
        state.db.add_log(&log)?;
    }

    Ok(make_log(
        "automations",
        &format!("Rotina executada: {}", routine.name),
        "info",
    ))
}

#[tauri::command]
fn list_memories(state: State<AppState>) -> Result<Vec<MemoryEntry>, String> {
    state.db.list_memories()
}

#[tauri::command]
fn save_memory(
    state: State<AppState>,
    title: String,
    content: String,
    category: String,
    pinned: bool,
) -> Result<MemoryEntry, String> {
    let now = Utc::now().to_rfc3339();
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("Memoria vazia.".to_string());
    }
    let memory = MemoryEntry {
        id: Uuid::new_v4().to_string(),
        title: if title.trim().is_empty() {
            memory_title(&content)
        } else {
            title.trim().to_string()
        },
        content,
        category: if category.trim().is_empty() {
            "geral".to_string()
        } else {
            category.trim().to_string()
        },
        pinned,
        created_at: now.clone(),
        updated_at: now,
    };
    state.db.save_memory(&memory)?;
    state.db.add_log(&make_log("memory", &format!("Memoria salva: {}", memory.title), "info"))?;
    Ok(memory)
}

#[tauri::command]
fn delete_memory(state: State<AppState>, id: String) -> Result<bool, String> {
    let deleted = state.db.delete_memory(&id)?;
    if deleted {
        state.db.add_log(&make_log("memory", "Memoria removida.", "warn"))?;
    }
    Ok(deleted)
}

#[tauri::command]
fn list_music_favorites(state: State<AppState>) -> Result<Vec<MusicFavorite>, String> {
    state.db.list_music_favorites()
}

#[tauri::command]
fn save_music_favorite(
    state: State<AppState>,
    name: String,
    uri: Option<String>,
    query: Option<String>,
    kind: String,
) -> Result<MusicFavorite, String> {
    let favorite = MusicFavorite {
        id: Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        uri: uri.filter(|value| !value.trim().is_empty()),
        query: query.filter(|value| !value.trim().is_empty()),
        kind: if kind.trim().is_empty() {
            "playlist".to_string()
        } else {
            kind.trim().to_string()
        },
        created_at: Utc::now().to_rfc3339(),
    };
    if favorite.name.is_empty() {
        return Err("Nome do favorito ausente.".to_string());
    }
    state.db.save_music_favorite(&favorite)?;
    persist_music_memory(&state.db, &favorite)?;
    state.db.add_log(&make_log("spotify", &format!("Favorito musical salvo: {}", favorite.name), "info"))?;
    Ok(favorite)
}

#[tauri::command]
fn delete_music_favorite(state: State<AppState>, id: String) -> Result<bool, String> {
    let deleted = state.db.delete_music_favorite(&id)?;
    if deleted {
        let _ = state.db.delete_memory(&format!("music-favorite-{id}"));
        state.db.add_log(&make_log("spotify", "Favorito musical removido.", "warn"))?;
    }
    Ok(deleted)
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<AssistantSettings, String> {
    state.db.get_settings()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: AssistantSettings) -> Result<AssistantSettings, String> {
    system_control::set_start_with_windows(settings.start_with_windows)?;
    state.db.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
fn get_pc_control_state(state: State<AppState>) -> Result<PcControlState, String> {
    let settings = state.db.get_settings()?;
    let mut pc_state = system_control::get_pc_state(settings.system_volume);
    if !settings.permissions.processes {
        pc_state.processes.clear();
    }
    if !settings.permissions.windows {
        pc_state.windows.clear();
    }
    if !settings.permissions.clipboard {
        pc_state.clipboard_preview.clear();
    }
    Ok(pc_state)
}

#[tauri::command]
fn list_processes(state: State<AppState>) -> Result<Vec<ProcessInfo>, String> {
    ensure_permission(state.db.get_settings()?.permissions.processes, "processos")?;
    Ok(system_control::list_processes())
}

#[tauri::command]
fn close_process(state: State<AppState>, pid: u32) -> Result<bool, String> {
    ensure_permission(state.db.get_settings()?.permissions.processes, "processos")?;
    let result = system_control::close_process(pid)?;
    let log = make_log("pc_control", &format!("Processo {pid} encerrado."), "warn");
    state.db.add_log(&log)?;
    Ok(result)
}

#[tauri::command]
fn get_volume(state: State<AppState>) -> Result<u8, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    let actual = system_control::get_system_volume()?;
    let mut settings = state.db.get_settings()?;
    settings.system_volume = actual;
    state.db.save_settings(&settings)?;
    Ok(actual)
}

#[tauri::command]
fn set_volume(state: State<AppState>, level: u8) -> Result<u8, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    let level = level.min(100);
    system_control::set_system_volume(level)?;
    let mut settings = state.db.get_settings()?;
    settings.system_volume = level;
    state.db.save_settings(&settings)?;
    state.db.add_log(&make_log(
        "pc_control",
        &format!("Volume do Windows ajustado para {}%.", level),
        "info",
    ))?;
    Ok(system_control::get_system_volume().unwrap_or(level))
}

#[tauri::command]
fn get_brightness(state: State<AppState>) -> Result<u8, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    system_control::get_brightness()
}

#[tauri::command]
fn set_brightness(state: State<AppState>, level: u8) -> Result<u8, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    let level = level.min(100);
    system_control::set_brightness(level)?;
    state
        .db
        .add_log(&make_log("pc_control", &format!("Brilho ajustado para {}%.", level), "info"))?;
    Ok(system_control::get_brightness().unwrap_or(level))
}

#[tauri::command]
fn take_screenshot(app: AppHandle, state: State<AppState>) -> Result<String, String> {
    ensure_permission(state.db.get_settings()?.permissions.screenshots, "capturas de tela")?;
    let path = system_control::take_screenshot(&app)?;
    state.db.add_log(&make_log("pc_control", &format!("Screenshot salvo em {path}"), "info"))?;
    Ok(path)
}

#[tauri::command]
fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    Ok(system_control::list_installed_apps())
}

#[tauri::command]
fn open_app(state: State<AppState>, app: String) -> Result<bool, String> {
    ensure_permission(state.db.get_settings()?.permissions.apps, "aplicativos")?;
    let result = system_control::open_app(&app)?;
    state.db.add_log(&make_log("apps", &format!("Aplicativo aberto: {app}"), "info"))?;
    Ok(result)
}

#[tauri::command]
fn close_app(state: State<AppState>, app: String) -> Result<bool, String> {
    ensure_permission(state.db.get_settings()?.permissions.apps, "aplicativos")?;
    let result = system_control::close_app(&app)?;
    state.db.add_log(&make_log("apps", &format!("Aplicativo fechado: {app}"), "warn"))?;
    Ok(result)
}

#[tauri::command]
fn list_windows(state: State<AppState>) -> Result<Vec<crate::models::WindowInfo>, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    Ok(system_control::list_windows())
}

#[tauri::command]
fn focus_window(state: State<AppState>, id: String) -> Result<bool, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    let result = system_control::focus_window(&id)?;
    state.db.add_log(&make_log("windows", &format!("Janela focada: {id}"), "info"))?;
    Ok(result)
}

#[tauri::command]
fn minimize_window(state: State<AppState>, id: String) -> Result<bool, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    let result = system_control::minimize_window(&id)?;
    state.db.add_log(&make_log("windows", &format!("Janela minimizada: {id}"), "info"))?;
    Ok(result)
}

#[tauri::command]
fn maximize_window(state: State<AppState>, id: String) -> Result<bool, String> {
    ensure_permission(state.db.get_settings()?.permissions.windows, "controle do Windows")?;
    let result = system_control::maximize_window(&id)?;
    state.db.add_log(&make_log("windows", &format!("Janela maximizada: {id}"), "info"))?;
    Ok(result)
}

#[tauri::command]
fn get_voice_state(
    app: AppHandle,
    state: State<AppState>,
) -> Result<VoiceState, String> {
    let settings = state.db.get_settings()?;
    Ok(voice::get_state(&app, &state.voice_runtime, &settings))
}

#[tauri::command]
fn start_listening(
    app: AppHandle,
    state: State<AppState>,
) -> Result<VoiceState, String> {
    let mut settings = state.db.get_settings()?;
    ensure_permission(settings.permissions.voice, "voz")?;
    settings.voice_enabled = true;
    state.db.save_settings(&settings)?;
    let voice_state = voice::start_listening(&app, &state.voice_runtime, &settings)?;
    state.db.add_log(&make_log("voice", "Escuta local iniciada.", "info"))?;
    Ok(voice_state)
}

#[tauri::command]
fn stop_listening(state: State<AppState>) -> Result<VoiceState, String> {
    let settings = state.db.get_settings()?;
    let voice_state = voice::stop_listening(&state.voice_runtime, &settings);
    state.db.add_log(&make_log("voice", "Escuta local finalizada.", "info"))?;
    Ok(voice_state)
}

#[tauri::command]
fn test_microphone() -> Result<Value, String> {
    Ok(json!({
        "available": false,
        "level": 0.0,
        "message": "Use o teste real da tela Voz; o backend nao inventa disponibilidade de microfone."
    }))
}

#[tauri::command]
fn get_task_state(state: State<AppState>) -> Result<TaskState, String> {
    state.db.get_task_state()
}

#[tauri::command]
fn clear_task_state(state: State<AppState>) -> Result<bool, String> {
    state.db.clear_task_state()?;
    Ok(true)
}

#[tauri::command]
fn list_history(state: State<AppState>) -> Result<Vec<CommandHistoryEntry>, String> {
    state.db.list_history_entries()
}

#[tauri::command]
fn clear_history(state: State<AppState>) -> Result<bool, String> {
    state.db.clear_history()?;
    state.db.add_log(&make_log("history", "Historico limpo.", "warn"))?;
    Ok(true)
}

#[tauri::command]
async fn google_login(state: State<'_, AppState>) -> Result<GoogleProfile, String> {
    google::login(&state.db).await
}

#[tauri::command]
fn google_logout(state: State<AppState>) -> Result<bool, String> {
    google::logout(&state.db)
}

#[tauri::command]
fn get_account_status(state: State<AppState>) -> Result<AccountStatus, String> {
    google::account_status(&state.db)
}

#[tauri::command]
fn spotify_begin_auth(
    state: State<AppState>,
    client_id: String,
    redirect_uri: String,
) -> Result<String, String> {
    spotify::begin_auth_with_callback(state.db.clone(), &state.spotify_auth, client_id, redirect_uri)
}

#[tauri::command]
async fn spotify_finish_auth(
    state: State<'_, AppState>,
    callback_url: String,
) -> Result<bool, String> {
    spotify::finish_auth(&state.db, &state.spotify_auth, callback_url).await
}

#[tauri::command]
async fn spotify_current_track(state: State<'_, AppState>) -> Result<Option<SpotifyTrack>, String> {
    spotify::current_track(&state.db).await
}

#[tauri::command]
async fn spotify_list_playlists(state: State<'_, AppState>) -> Result<Vec<SpotifyPlaylist>, String> {
    spotify::list_playlists(&state.db).await
}

#[tauri::command]
async fn spotify_search_playlists(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SpotifyPlaylist>, String> {
    spotify::search_playlists(&state.db, query).await
}

#[tauri::command]
async fn spotify_play_uri(state: State<'_, AppState>, uri: String) -> Result<bool, String> {
    spotify::play_uri(&state.db, uri).await
}

#[tauri::command]
async fn spotify_pause(state: State<'_, AppState>) -> Result<bool, String> {
    spotify::pause(&state.db).await
}

#[tauri::command]
async fn spotify_next(state: State<'_, AppState>) -> Result<bool, String> {
    spotify::next(&state.db).await
}

#[tauri::command]
async fn spotify_previous(state: State<'_, AppState>) -> Result<bool, String> {
    spotify::previous(&state.db).await
}

#[tauri::command]
async fn spotify_set_volume(state: State<'_, AppState>, volume: u8) -> Result<bool, String> {
    spotify::set_volume(&state.db, volume).await
}

#[tauri::command]
async fn spotify_play_liked_tracks(state: State<'_, AppState>) -> Result<bool, String> {
    spotify::play_liked_tracks(&state.db).await
}

#[tauri::command]
async fn play_music(state: State<'_, AppState>, query: String) -> Result<MusicCommandResult, String> {
    music::play_music(&state.db, &query).await
}

#[tauri::command]
async fn play_liked_music(state: State<'_, AppState>) -> Result<MusicCommandResult, String> {
    music::play_liked_music(&state.db).await
}

#[tauri::command]
async fn pause_music(state: State<'_, AppState>) -> Result<MusicCommandResult, String> {
    music::pause_music(&state.db).await
}

#[tauri::command]
async fn next_track(state: State<'_, AppState>) -> Result<MusicCommandResult, String> {
    music::next_track(&state.db).await
}

#[tauri::command]
async fn previous_track(state: State<'_, AppState>) -> Result<MusicCommandResult, String> {
    music::previous_track(&state.db).await
}

#[tauri::command]
fn list_logs(state: State<AppState>) -> Result<Vec<LogEntry>, String> {
    state.db.list_logs()
}

fn make_log(module: &str, message: &str, level: &str) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4().to_string(),
        module: module.to_string(),
        level: level.to_string(),
        message: message.to_string(),
        created_at: Utc::now().to_rfc3339(),
    }
}

fn make_music_log(module: &str, result: &MusicCommandResult) -> LogEntry {
    make_log(
        module,
        &result.message,
        if result.playback_started { "info" } else { "warn" },
    )
}

fn compose_result_summary(executed: &[String], blocked: &[String], failed: &[String]) -> String {
    let mut parts = Vec::new();
    if !executed.is_empty() {
        parts.push(format!("Executado: {}", executed.join("; ")));
    }
    if !blocked.is_empty() {
        parts.push(format!("Bloqueado: {}", blocked.join("; ")));
    }
    if !failed.is_empty() {
        parts.push(format!("Falhou: {}", failed.join("; ")));
    }
    if parts.is_empty() {
        "Nenhuma acao executada.".to_string()
    } else {
        parts.join(" | ")
    }
}

fn ensure_permission(enabled: bool, label: &str) -> Result<(), String> {
    if enabled {
        Ok(())
    } else {
        Err(format!("Permissao de {label} desativada nas configuracoes."))
    }
}

fn compose_autonomous_reply(base: &str, executed: &[String], blocked: &[String], failed: &[String]) -> String {
    let mut parts = vec![base.trim().to_string()];
    if !executed.is_empty() {
        parts.push(format!("Executado automaticamente: {}.", executed.join("; ")));
    }
    if !blocked.is_empty() {
        parts.push(format!("Precisa de confirmacao: {}.", blocked.join("; ")));
    }
    if !failed.is_empty() {
        parts.push(format!("Nao consegui concluir: {}.", failed.join("; ")));
    }
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn update_task_state(
    state: &State<'_, AppState>,
    objective: &str,
    executed: &[String],
    blocked: &[String],
    failed: &[String],
) -> Result<(), String> {
    let next_step = if !failed.is_empty() {
        Some(format!("Resolver falha: {}", failed.join("; ")))
    } else if !blocked.is_empty() {
        Some(format!("Aguardando confirmacao: {}", blocked.join("; ")))
    } else {
        None
    };
    let task_state = TaskState {
        objective: Some(objective.to_string()),
        completed_steps: executed.to_vec(),
        next_step,
        updated_at: Some(Utc::now().to_rfc3339()),
    };
    state.db.save_task_state(&task_state)
}

fn is_liked_music_query(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("curtidas")
        || lower.contains("musicas favoritas")
        || lower.contains("minhas favoritas")
        || lower.contains("liked songs")
}

fn memory_title(value: &str) -> String {
    let mut title = value
        .split_whitespace()
        .take(7)
        .collect::<Vec<_>>()
        .join(" ");
    if title.chars().count() > 64 {
        title = title.chars().take(64).collect();
    }
    if title.is_empty() {
        "Memoria".to_string()
    } else {
        title
    }
}

fn persist_music_memory(db: &Database, favorite: &MusicFavorite) -> Result<(), String> {
    let content = favorite
        .query
        .clone()
        .or_else(|| favorite.uri.clone())
        .unwrap_or_else(|| favorite.name.clone());
    let now = Utc::now().to_rfc3339();
    db.save_memory(&MemoryEntry {
        id: format!("music-favorite-{}", favorite.id),
        title: format!("Favorito musical: {}", favorite.name),
        content,
        category: "musica".to_string(),
        pinned: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Abrir", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("PC Control AI")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let db = Arc::new(Database::new(&app.handle())?);
            let settings = db.get_settings().unwrap_or_default();
            automation::start_worker(app.handle().clone(), db.clone());
            app.manage(AppState {
                db,
                spotify_auth: SpotifyAuthState::new(),
                local_ai_runtime: local_ai::LlamaRuntimeState::new(),
                voice_runtime: voice::VoiceRuntimeState::new(&settings),
            });
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let should_hide = window
                    .app_handle()
                    .try_state::<AppState>()
                    .and_then(|state| state.db.get_settings().ok())
                    .map(|settings| settings.minimize_to_tray)
                    .unwrap_or(false);
                if should_hide {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_runtime_status,
            complete_onboarding,
            local_ai_status,
            set_local_ai_path,
            list_messages,
            send_chat_message,
            execute_action,
            list_routines,
            save_routine,
            run_routine,
            list_memories,
            save_memory,
            delete_memory,
            list_music_favorites,
            save_music_favorite,
            delete_music_favorite,
            get_settings,
            save_settings,
            get_pc_control_state,
            list_processes,
            close_process,
            get_volume,
            set_volume,
            get_brightness,
            set_brightness,
            take_screenshot,
            list_installed_apps,
            open_app,
            close_app,
            list_windows,
            focus_window,
            minimize_window,
            maximize_window,
            get_voice_state,
            start_listening,
            stop_listening,
            test_microphone,
            get_task_state,
            clear_task_state,
            list_history,
            clear_history,
            google_login,
            google_logout,
            get_account_status,
            spotify_begin_auth,
            spotify_finish_auth,
            spotify_current_track,
            spotify_list_playlists,
            spotify_search_playlists,
            spotify_play_uri,
            spotify_pause,
            spotify_next,
            spotify_previous,
            spotify_set_volume,
            spotify_play_liked_tracks,
            play_music,
            play_liked_music,
            pause_music,
            next_track,
            previous_track,
            list_logs
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar PC Control AI");
}
