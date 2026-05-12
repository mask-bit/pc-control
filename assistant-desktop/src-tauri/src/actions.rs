use crate::models::{ActionRisk, ActionSpec, LogEntry};
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
                .ok_or("URL ausente")?;
            open::that(url).map_err(|err| err.to_string())?;
        }
        "open_path" => {
            let path = get_arg(&action.args, "path")
                .or_else(|| get_arg(&action.args, "target"))
                .ok_or("Caminho ausente")?;
            open::that(path).map_err(|err| err.to_string())?;
        }
        "open_app" => {
            let target = get_arg(&action.args, "app")
                .or_else(|| get_arg(&action.args, "target"))
                .ok_or("Aplicativo ausente")?;
            if target.starts_with("shell:") {
                Command::new("cmd")
                    .args(["/C", "start", "", target])
                    .spawn()
                    .map_err(|err| err.to_string())?;
            } else {
                Command::new(target).spawn().map_err(|err| err.to_string())?;
            }
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
    if matches!(action.risk, ActionRisk::High) || require_confirmation {
        action.requires_confirmation = true;
    }
    action
}

fn get_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}
