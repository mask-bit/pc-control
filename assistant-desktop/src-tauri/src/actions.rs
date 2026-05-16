use crate::models::{ActionRisk, ActionSpec, AssistantSettings, LogEntry};
use arboard::Clipboard;
use chrono::Utc;
use serde_json::Value;
use std::process::Command;
use uuid::Uuid;

pub fn execute_basic_action(action: &ActionSpec) -> Result<Option<LogEntry>, String> {
    match action.action_type.as_str() {
        "open_url" => {
            let url = get_arg(&action.args, "url")
                .or_else(|| get_arg(&action.args, "target"))
                .or_else(|| non_empty(&action.target))
                .ok_or("URL ausente")?;
            open::that(url).map_err(|err| err.to_string())?;
        }
        "open_path" => {
            let path = get_arg(&action.args, "path")
                .or_else(|| get_arg(&action.args, "target"))
                .or_else(|| non_empty(&action.target))
                .ok_or("Caminho ausente")?;
            open::that(path).map_err(|err| err.to_string())?;
        }
        "open_app" => {
            let target = get_arg(&action.args, "app")
                .or_else(|| get_arg(&action.args, "target"))
                .or_else(|| non_empty(&action.target))
                .ok_or("Aplicativo ausente")?;
            Command::new("cmd")
                .args(["/C", "start", "", target])
                .spawn()
                .map_err(|err| err.to_string())?;
        }
        "close_app" => {
            let target = get_arg(&action.args, "app")
                .or_else(|| get_arg(&action.args, "process"))
                .or_else(|| non_empty(&action.target))
                .ok_or("Aplicativo ausente")?;
            let exe = normalize_process_name(target);
            Command::new("taskkill")
                .args(["/IM", &exe, "/F"])
                .output()
                .map_err(|err| err.to_string())?;
        }
        "copy_text" => {
            let text = get_arg(&action.args, "text").ok_or("Texto ausente")?;
            let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
            clipboard.set_text(text.to_string()).map_err(|err| err.to_string())?;
        }
        _ => return Ok(None),
    }

    Ok(Some(LogEntry {
        id: Uuid::new_v4().to_string(),
        level: "info".to_string(),
        module: "executor".to_string(),
        message: format!("Acao executada: {}", action.label),
        created_at: Utc::now().to_rfc3339(),
    }))
}

pub fn sanitize_action(mut action: ActionSpec, require_confirmation: bool) -> ActionSpec {
    if matches!(&action.risk, ActionRisk::High) || (require_confirmation && protected_reason(&action).is_some()) {
        action.requires_confirmation = true;
    }
    action
}

pub fn protected_reason(action: &ActionSpec) -> Option<String> {
    if matches!(&action.risk, ActionRisk::High) || action.requires_confirmation {
        return Some("acao marcada como sensivel pela IA".to_string());
    }

    let action_text = format!("{} {}", action.label, action.args);
    if contains_sensitive_text(&action_text) {
        return Some("pedido parece irreversivel ou sensivel".to_string());
    }

    match action.action_type.as_str() {
        "open_url" => {
            let url = get_arg(&action.args, "url")
                .or_else(|| get_arg(&action.args, "target"))
                .or_else(|| non_empty(&action.target))?;
            if url.starts_with("http://") || url.starts_with("https://") {
                None
            } else {
                Some("URL fora de http/https bloqueada".to_string())
            }
        }
        "open_app" => {
            let target = get_arg(&action.args, "app")
                .or_else(|| get_arg(&action.args, "target"))
                .or_else(|| non_empty(&action.target))?;
            let lower = target.to_lowercase();
            let blocked = [
                "cmd", "powershell", "pwsh", "wt.exe", "terminal", "wscript", "cscript", "mshta",
                "regedit", "reg.exe", "rundll32", "wmic",
            ];
            if blocked.iter().any(|item| lower.contains(item)) || action.args.get("command").is_some() {
                Some("abertura de shell ou ferramenta administrativa bloqueada".to_string())
            } else {
                None
            }
        }
        "open_path"
        | "close_app"
        | "copy_text"
        | "system_volume"
        | "brightness"
        | "take_screenshot"
        | "list_processes"
        | "window_minimize"
        | "window_maximize"
        | "window_focus"
        | "play_music"
        | "play_liked_music"
        | "pause_music"
        | "next_track"
        | "previous_track"
        | "spotify_play"
        | "spotify_pause"
        | "spotify_next"
        | "spotify_previous"
        | "spotify_volume"
        | "spotify_liked"
        | "run_routine"
        | "save_memory"
        | "save_music_favorite" => None,
        _ => Some("tipo de acao nao permitido no modo autonomo".to_string()),
    }
}

pub fn permission_denial(action: &ActionSpec, settings: &AssistantSettings) -> Option<String> {
    let permissions = &settings.permissions;
    match action.action_type.as_str() {
        "open_app" | "close_app" => (!permissions.apps).then_some("permissao de aplicativos desativada".to_string()),
        "copy_text" => (!permissions.clipboard).then_some("permissao da area de transferencia desativada".to_string()),
        "take_screenshot" => (!permissions.screenshots).then_some("permissao de capturas de tela desativada".to_string()),
        "list_processes" | "close_process" => (!permissions.processes).then_some("permissao de processos desativada".to_string()),
        "power" => (!permissions.power).then_some("permissao de energia desativada".to_string()),
        "system_volume" | "brightness" | "window_minimize" | "window_maximize" | "window_focus" => {
            (!permissions.windows).then_some("permissao de controle do Windows desativada".to_string())
        }
        _ => None,
    }
}

fn contains_sensitive_text(value: &str) -> bool {
    let lower = value.to_lowercase();
    [
        "apagar",
        "deletar",
        "excluir",
        "remover arquivo",
        "formatar",
        "comprar",
        "pagamento",
        "cartao",
        "cartão",
        "senha",
        "token",
        "api key",
        "enviar senha",
        "powershell",
        "prompt de comando",
        "cmd.exe",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn get_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}

fn normalize_process_name(value: &str) -> String {
    let lower = value.to_lowercase();
    let mapped = match lower.as_str() {
        "chrome" => "chrome.exe",
        "edge" | "msedge" => "msedge.exe",
        "spotify" => "Spotify.exe",
        "notepad" | "bloco de notas" => "notepad.exe",
        "calc" | "calculadora" => "CalculatorApp.exe",
        "code" | "vscode" | "vs code" => "Code.exe",
        other => other,
    };
    if mapped.ends_with(".exe") {
        mapped.to_string()
    } else {
        format!("{mapped}.exe")
    }
}
