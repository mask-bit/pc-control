use crate::actions;
use crate::models::{ActionSpec, AssistantSettings, LogEntry, MemoryEntry, MusicFavorite, Routine};
use crate::{music, system_control};
use crate::storage::Database;
use chrono::{Local, Utc};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};
use tauri::AppHandle;
use uuid::Uuid;

pub fn start_worker(app: AppHandle, db: Arc<Database>) {
    std::thread::spawn(move || {
        let mut seen_processes: Vec<String> = Vec::new();
        let mut last_schedule_minute = String::new();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();

        loop {
            if let Ok(routines) = db.list_routines() {
                let now = Local::now().format("%H:%M").to_string();
                if now != last_schedule_minute {
                    for routine in routines.iter().filter(|item| item.enabled && item.trigger.kind == "schedule") {
                        if routine.trigger.value.as_deref() == Some(now.as_str()) {
                            execute_triggered_routine(&app, &db, runtime.as_ref(), routine.clone(), "horario");
                        }
                    }
                    last_schedule_minute = now;
                }

                let mut system = System::new_all();
                system.refresh_processes(ProcessesToUpdate::All, true);
                let current: Vec<String> = system
                    .processes()
                    .values()
                    .map(|process| process.name().to_string_lossy().to_lowercase())
                    .collect();

                for routine in routines.iter().filter(|item| item.enabled && item.trigger.kind == "app_open") {
                    if let Some(value) = routine.trigger.value.as_deref() {
                        let target = value.to_lowercase();
                        let is_open = current.iter().any(|name| name.contains(&target));
                        let was_open = seen_processes.iter().any(|name| name.contains(&target));
                        if is_open && !was_open {
                            execute_triggered_routine(&app, &db, runtime.as_ref(), routine.clone(), "app aberto");
                        }
                    }
                }
                seen_processes = current;
            }
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}

fn execute_triggered_routine(
    app: &AppHandle,
    db: &Database,
    runtime: Option<&tokio::runtime::Runtime>,
    routine: Routine,
    trigger_reason: &str,
) {
    let _ = add_log(
        db,
        "automation",
        &format!("Gatilho de {trigger_reason} executando rotina: {}", routine.name),
        "info",
    );

    let settings = match db.get_settings() {
        Ok(settings) => settings,
        Err(error) => {
            let _ = add_log(db, "automation", &format!("Rotina bloqueada: {error}"), "error");
            return;
        }
    };

    for action in routine.actions {
        if let Some(reason) = automation_block_reason(&action, &settings) {
            let _ = add_log(
                db,
                "automation",
                &format!("Acao bloqueada em {}: {} ({reason})", routine.name, action.label),
                "warn",
            );
            continue;
        }

        let result = match runtime {
            Some(runtime) => runtime.block_on(execute_automation_action(app, db, action)),
            None => Err("Runtime async indisponivel para automacoes.".to_string()),
        };

        match result {
            Ok(log) => {
                let _ = db.add_log(&log);
            }
            Err(error) => {
                let _ = add_log(
                    db,
                    "automation",
                    &format!("Falha em rotina {}: {error}", routine.name),
                    "error",
                );
            }
        }
    }

    let _ = add_log(
        db,
        "automation",
        &format!("Rotina automatica finalizada: {}", routine.name),
        "info",
    );
}

fn automation_block_reason(action: &ActionSpec, settings: &AssistantSettings) -> Option<String> {
    actions::permission_denial(action, settings).or_else(|| actions::protected_reason(action))
}

async fn execute_automation_action(
    app: &AppHandle,
    db: &Database,
    action: ActionSpec,
) -> Result<LogEntry, String> {
    match action.action_type.as_str() {
        "open_app" => {
            let target = string_arg(&action, "app").unwrap_or(&action.target);
            system_control::open_app(target)?;
            Ok(make_log("apps", &format!("Aplicativo aberto: {}", action.label), "info"))
        }
        "close_app" => {
            let target = string_arg(&action, "app").unwrap_or(&action.target);
            system_control::close_app(target)?;
            Ok(make_log("apps", &format!("Aplicativo fechado: {}", action.label), "info"))
        }
        "system_volume" => {
            let level = number_arg(&action, "level", 50).min(100) as u8;
            system_control::set_system_volume(level)?;
            let mut settings = db.get_settings()?;
            settings.system_volume = level;
            db.save_settings(&settings)?;
            Ok(make_log("pc_control", &format!("Volume do Windows ajustado para {}%.", level), "info"))
        }
        "brightness" => {
            let level = number_arg(&action, "level", 50).min(100) as u8;
            system_control::set_brightness(level)?;
            Ok(make_log("pc_control", &format!("Brilho ajustado para {}%.", level), "info"))
        }
        "take_screenshot" => {
            let path = system_control::take_screenshot(app)?;
            Ok(make_log("pc_control", &format!("Screenshot salvo em {path}"), "info"))
        }
        "list_processes" => Ok(make_log("pc_control", "Processos atualizados pela automacao.", "info")),
        "close_process" => {
            let pid = action
                .args
                .get("pid")
                .and_then(Value::as_u64)
                .ok_or("PID ausente.")? as u32;
            system_control::close_process(pid)?;
            Ok(make_log("pc_control", &format!("Processo {pid} encerrado."), "warn"))
        }
        "window_focus" => {
            let id = string_arg(&action, "id").unwrap_or(&action.target);
            system_control::focus_window(id)?;
            Ok(make_log("windows", &format!("Janela focada: {}", action.label), "info"))
        }
        "window_minimize" => {
            let id = string_arg(&action, "id").unwrap_or(&action.target);
            system_control::minimize_window(id)?;
            Ok(make_log("windows", &format!("Janela minimizada: {}", action.label), "info"))
        }
        "window_maximize" => {
            let id = string_arg(&action, "id").unwrap_or(&action.target);
            system_control::maximize_window(id)?;
            Ok(make_log("windows", &format!("Janela maximizada: {}", action.label), "info"))
        }
        "play_music" | "spotify_play" => {
            let query = string_arg(&action, "uri")
                .or_else(|| string_arg(&action, "query"))
                .unwrap_or(&action.target);
            let result = music::play_music(db, query).await?;
            Ok(make_log("spotify", &result.message, if result.playback_started { "info" } else { "warn" }))
        }
        "play_liked_music" | "spotify_liked" => {
            let result = music::play_liked_music(db).await?;
            Ok(make_log("spotify", &result.message, if result.playback_started { "info" } else { "warn" }))
        }
        "pause_music" | "spotify_pause" => {
            let result = music::pause_music(db).await?;
            Ok(make_log("spotify", &result.message, "info"))
        }
        "next_track" | "spotify_next" => {
            let result = music::next_track(db).await?;
            Ok(make_log("spotify", &result.message, "info"))
        }
        "previous_track" | "spotify_previous" => {
            let result = music::previous_track(db).await?;
            Ok(make_log("spotify", &result.message, "info"))
        }
        "save_memory" => save_memory_from_action(db, &action),
        "save_music_favorite" => save_music_favorite_from_action(db, &action),
        _ => actions::execute_basic_action(&action)?
            .ok_or_else(|| format!("Acao nao implementada para automacao: {}", action.action_type)),
    }
}

fn save_memory_from_action(db: &Database, action: &ActionSpec) -> Result<LogEntry, String> {
    let content = string_arg(action, "content")
        .or_else(|| string_arg(action, "text"))
        .unwrap_or(&action.target)
        .trim()
        .to_string();
    if content.is_empty() {
        return Err("Memoria vazia.".to_string());
    }
    let now = Utc::now().to_rfc3339();
    let memory = MemoryEntry {
        id: Uuid::new_v4().to_string(),
        title: string_arg(action, "title")
            .map(str::to_string)
            .unwrap_or_else(|| short_title(&content)),
        content,
        category: string_arg(action, "category").unwrap_or("geral").to_string(),
        pinned: true,
        created_at: now.clone(),
        updated_at: now,
    };
    db.save_memory(&memory)?;
    Ok(make_log("memory", &format!("Memoria salva: {}", memory.title), "info"))
}

fn save_music_favorite_from_action(db: &Database, action: &ActionSpec) -> Result<LogEntry, String> {
    let name = string_arg(action, "name").unwrap_or(&action.target).trim().to_string();
    let uri = string_arg(action, "uri").map(str::to_string);
    let query = string_arg(action, "query").map(str::to_string);
    if name.is_empty() || (uri.is_none() && query.is_none()) {
        return Err("Favorito musical incompleto.".to_string());
    }
    let favorite = MusicFavorite {
        id: Uuid::new_v4().to_string(),
        name,
        uri,
        query,
        kind: string_arg(action, "kind").unwrap_or("playlist").to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    db.save_music_favorite(&favorite)?;
    Ok(make_log("spotify", &format!("Favorito musical salvo: {}", favorite.name), "info"))
}

fn string_arg<'a>(action: &'a ActionSpec, key: &str) -> Option<&'a str> {
    action.args.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn number_arg(action: &ActionSpec, key: &str, fallback: u64) -> u64 {
    action.args.get(key).and_then(Value::as_u64).unwrap_or(fallback)
}

fn short_title(value: &str) -> String {
    let title = value.split_whitespace().take(7).collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        "Memoria".to_string()
    } else {
        title.chars().take(64).collect()
    }
}

fn add_log(db: &Database, module: &str, message: &str, level: &str) -> Result<(), String> {
    db.add_log(&make_log(module, message, level))
}

fn make_log(module: &str, message: &str, level: &str) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4().to_string(),
        level: level.to_string(),
        module: module.to_string(),
        message: message.to_string(),
        created_at: Utc::now().to_rfc3339(),
    }
}
