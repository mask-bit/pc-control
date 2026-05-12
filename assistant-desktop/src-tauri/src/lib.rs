mod actions;
mod automation;
mod models;
mod openai;
mod secrets;
mod spotify;
mod storage;

use crate::models::{
    ActionRisk, ActionSpec, AssistantSettings, ChatMessage, ChatResult, IntegrationStatus, LogEntry,
    Routine, SpotifyPlaylist, SpotifyTrack, Trigger,
};
use crate::spotify::SpotifyAuthState;
use crate::storage::Database;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

pub struct AppState {
    db: Arc<Database>,
    spotify_auth: SpotifyAuthState,
}

#[tauri::command]
fn get_integration_status(state: State<AppState>) -> Result<IntegrationStatus, String> {
    let openai_connected = secrets::has_secret("openai_api_key");
    let spotify_connected = spotify::has_refresh_token();
    Ok(IntegrationStatus {
        openai_connected,
        spotify_connected,
        spotify_device_active: spotify_connected,
        last_error: None,
    })
}

#[tauri::command]
fn list_messages(state: State<AppState>) -> Result<Vec<ChatMessage>, String> {
    state.db.list_messages()
}

#[tauri::command]
async fn send_chat_message(
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
    let api_key = secrets::get_secret("openai_api_key");
    let result = openai::send_chat(api_key, settings, history, content, mode).await?;
    state.db.save_message(&result.message)?;
    Ok(result)
}

#[tauri::command]
async fn execute_action(state: State<'_, AppState>, action: ActionSpec) -> Result<LogEntry, String> {
    let maybe_log = match action.action_type.as_str() {
        "spotify_play" => {
            let uri = action
                .args
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| action.args.get("query").and_then(Value::as_str).map(str::to_string))
                .ok_or("URI ou busca Spotify ausente.")?;
            if uri.starts_with("spotify:") {
                spotify::play_uri(&state.db, uri).await?;
            } else {
                let results = spotify::search_playlists(&state.db, uri).await?;
                let first = results.first().ok_or("Nenhuma playlist encontrada.")?;
                spotify::play_uri(&state.db, first.uri.clone()).await?;
            }
            Some(make_log("spotify", &format!("Acao executada: {}", action.label), "info"))
        }
        "spotify_pause" => {
            spotify::pause(&state.db).await?;
            Some(make_log("spotify", &format!("Acao executada: {}", action.label), "info"))
        }
        "spotify_next" => {
            spotify::next(&state.db).await?;
            Some(make_log("spotify", &format!("Acao executada: {}", action.label), "info"))
        }
        "spotify_previous" => {
            spotify::previous(&state.db).await?;
            Some(make_log("spotify", &format!("Acao executada: {}", action.label), "info"))
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
            Some(run_routine_inner(&state, id).await?)
        }
        _ => actions::execute_basic_action(&action)?,
    };

    let log = maybe_log.unwrap_or_else(|| make_log("executor", "Acao nao implementada no MVP.", "warn"));
    state.db.add_log(&log)?;
    Ok(log)
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
async fn run_routine(state: State<'_, AppState>, id: String) -> Result<LogEntry, String> {
    let log = run_routine_inner(&state, &id).await?;
    state.db.add_log(&log)?;
    Ok(log)
}

async fn run_routine_inner(state: &State<'_, AppState>, id: &str) -> Result<LogEntry, String> {
    let routines = state.db.list_routines()?;
    let routine = routines
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("Rotina nao encontrada.")?;

    for action in routine.actions {
        execute_action_inner(state, action).await?;
    }

    Ok(make_log(
        "routines",
        &format!("Rotina executada: {}", routine.name),
        "info",
    ))
}

async fn execute_action_inner(state: &State<'_, AppState>, action: ActionSpec) -> Result<(), String> {
    match action.action_type.as_str() {
        "spotify_play" | "spotify_pause" | "spotify_next" | "spotify_previous" | "spotify_volume" => {
            let _ = execute_action(state.clone(), action).await?;
        }
        _ => {
            let _ = actions::execute_basic_action(&action)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn import_workspace_config(app: AppHandle, state: State<AppState>) -> Result<Vec<Routine>, String> {
    let path = find_workspace_config(&app).ok_or("workspace-config.json nao encontrado.")?;
    let raw = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let value: Value = serde_json::from_str(&raw).map_err(|err| err.to_string())?;
    let profiles = value
        .get("profile")
        .or_else(|| value.get("profiles"))
        .and_then(Value::as_object)
        .ok_or("Nenhum perfil encontrado no config antigo.")?;

    let mut imported = Vec::new();
    for (name, profile) in profiles {
        let mut actions = Vec::new();
        if let Some(apps) = profile.get("aplikacje").and_then(Value::as_array) {
            for app in apps {
                let label = app.get("nazwa").and_then(Value::as_str).unwrap_or("App");
                let exe = app.get("exe").and_then(Value::as_str).unwrap_or("");
                let args = app.get("argumenty").and_then(Value::as_str).unwrap_or("");
                let (action_type, action_args) = if args.starts_with("http") {
                    ("open_url", json!({ "url": args }))
                } else if args.starts_with("spotify:") {
                    ("spotify_play", json!({ "uri": args }))
                } else if !exe.is_empty() {
                    ("open_app", json!({ "target": exe }))
                } else {
                    ("open_url", json!({ "url": args }))
                };
                actions.push(ActionSpec {
                    action_type: action_type.to_string(),
                    label: format!("Abrir {label}"),
                    args: action_args,
                    risk: ActionRisk::Low,
                    requires_confirmation: false,
                });
            }
        }

        let routine = Routine {
            id: Uuid::new_v4().to_string(),
            name: format!("Perfil antigo: {name}"),
            enabled: true,
            trigger: Trigger {
                kind: "manual".to_string(),
                label: "Manual".to_string(),
                value: None,
            },
            actions,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };
        state.db.save_routine(&routine)?;
        imported.push(routine);
    }

    Ok(state.db.list_routines()?)
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<AssistantSettings, String> {
    state.db.get_settings()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: AssistantSettings) -> Result<AssistantSettings, String> {
    state.db.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
fn save_openai_key(api_key: String) -> Result<bool, String> {
    if api_key.trim().is_empty() {
        return Err("Chave OpenAI vazia.".to_string());
    }
    secrets::set_secret("openai_api_key", api_key.trim())?;
    Ok(true)
}

#[tauri::command]
fn test_openai() -> Result<bool, String> {
    Ok(openai::test_key())
}

#[tauri::command]
fn spotify_begin_auth(
    state: State<AppState>,
    client_id: String,
    redirect_uri: String,
) -> Result<String, String> {
    spotify::begin_auth(&state.spotify_auth, client_id, redirect_uri)
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

fn find_workspace_config(app: &AppHandle) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join("workspace-config.json"));
        if let Some(parent) = current.parent() {
            candidates.push(parent.join("workspace-config.json"));
        }
    }
    if let Ok(resource) = app.path().resource_dir() {
        candidates.push(resource.join("workspace-config.json"));
    }
    candidates.into_iter().find(|path| path.exists())
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Abrir", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Assistente Inteligente")
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
            automation::start_worker(app.handle().clone(), db.clone());
            app.manage(AppState {
                db,
                spotify_auth: SpotifyAuthState::new(),
            });
            setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_integration_status,
            list_messages,
            send_chat_message,
            execute_action,
            list_routines,
            save_routine,
            run_routine,
            import_workspace_config,
            get_settings,
            save_settings,
            save_openai_key,
            test_openai,
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
            list_logs
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar Assistente Inteligente");
}
