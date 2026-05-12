use crate::actions::sanitize_action;
use crate::models::{ActionRisk, ActionSpec, AssistantSettings, ChatMessage, ChatResult};
use crate::secrets;
use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn send_chat(
    api_key: Option<String>,
    settings: AssistantSettings,
    history: Vec<ChatMessage>,
    content: String,
    mode: String,
) -> Result<ChatResult, String> {
    if api_key.as_ref().is_none_or(|key| key.trim().is_empty()) {
        return Ok(local_fallback(content, mode, settings.require_confirmation));
    }

    let api_key = api_key.unwrap();
    let client = Client::new();
    let mut input = vec![json!({
        "role": "system",
        "content": system_prompt(&mode),
    })];

    for message in history.into_iter().rev().take(12).collect::<Vec<_>>().into_iter().rev() {
        if message.role == "user" || message.role == "assistant" {
            input.push(json!({
                "role": message.role,
                "content": message.content,
            }));
        }
    }
    input.push(json!({ "role": "user", "content": content }));

    let body = json!({
        "model": settings.model,
        "input": input,
        "tools": [tool_schema()],
        "tool_choice": "auto",
    });

    let response = client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Falha ao chamar OpenAI: {err}"))?;

    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|err| format!("Resposta OpenAI invalida: {err}"))?;

    if !status.is_success() {
        let message = value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("OpenAI retornou erro.");
        return Err(message.to_string());
    }

    let (reply, actions) = parse_openai_response(value, settings.require_confirmation)?;
    let message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: reply,
        created_at: Utc::now().to_rfc3339(),
        tool_calls: if actions.is_empty() { None } else { Some(actions.clone()) },
        status: Some(if actions.is_empty() { "done" } else { "pending" }.to_string()),
    };

    Ok(ChatResult {
        message,
        proposed_actions: actions,
    })
}

pub fn test_key() -> bool {
    secrets::has_secret("openai_api_key")
}

fn system_prompt(mode: &str) -> String {
    format!(
        r#"Voce e um assistente inteligente de desktop em portugues-BR.
Modo atual: {mode}.
Responda de forma objetiva. Quando o usuario pedir uma acao local ou Spotify,
proponha acoes por tool calling usando propose_actions. Nunca invente execucao
de shell livre. Acoes criticas devem pedir confirmacao."#
    )
}

fn tool_schema() -> Value {
    json!({
        "type": "function",
        "name": "propose_actions",
        "description": "Propoe acoes locais seguras para o orquestrador executar.",
        "parameters": {
            "type": "object",
            "properties": {
                "assistant_reply": {
                    "type": "string",
                    "description": "Resposta curta em portugues-BR para mostrar no chat."
                },
                "actions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": [
                                    "open_app",
                                    "open_url",
                                    "open_path",
                                    "copy_text",
                                    "spotify_play",
                                    "spotify_pause",
                                    "spotify_next",
                                    "spotify_previous",
                                    "spotify_volume",
                                    "run_routine"
                                ]
                            },
                            "label": { "type": "string" },
                            "args": {
                                "type": "object",
                                "additionalProperties": {
                                    "type": ["string", "number", "boolean", "null"]
                                }
                            },
                            "risk": {
                                "type": "string",
                                "enum": ["low", "medium", "high"]
                            },
                            "requires_confirmation": { "type": "boolean" }
                        },
                        "required": ["type", "label", "args", "risk", "requires_confirmation"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["assistant_reply", "actions"],
            "additionalProperties": false
        }
    })
}

fn parse_openai_response(value: Value, require_confirmation: bool) -> Result<(String, Vec<ActionSpec>), String> {
    let mut reply = value
        .get("output_text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut actions = Vec::new();

    if let Some(output) = value.get("output").and_then(Value::as_array) {
        for item in output {
            match item.get("type").and_then(Value::as_str) {
                Some("message") => {
                    if reply.is_empty() {
                        if let Some(content) = item.get("content").and_then(Value::as_array) {
                            for part in content {
                                if let Some(text) = part.get("text").and_then(Value::as_str) {
                                    reply.push_str(text);
                                }
                            }
                        }
                    }
                }
                Some("function_call") => {
                    if item.get("name").and_then(Value::as_str) == Some("propose_actions") {
                        if let Some(args_raw) = item.get("arguments").and_then(Value::as_str) {
                            let parsed: Value = serde_json::from_str(args_raw).map_err(|err| err.to_string())?;
                            if let Some(text) = parsed.get("assistant_reply").and_then(Value::as_str) {
                                reply = text.to_string();
                            }
                            if let Some(raw_actions) = parsed.get("actions").and_then(Value::as_array) {
                                for raw in raw_actions {
                                    let action: ActionSpec =
                                        serde_json::from_value(raw.clone()).map_err(|err| err.to_string())?;
                                    actions.push(sanitize_action(action, require_confirmation));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if reply.trim().is_empty() {
        reply = "Entendi. Posso ajudar com isso.".to_string();
    }
    Ok((reply, actions))
}

fn local_fallback(content: String, _mode: String, require_confirmation: bool) -> ChatResult {
    let lower = content.to_lowercase();
    let mut actions = Vec::new();

    if lower.contains("chrome") || lower.contains("navegador") {
        actions.push(ActionSpec {
            action_type: "open_url".to_string(),
            label: "Abrir navegador com pesquisa".to_string(),
            args: json!({ "url": "https://www.google.com" }),
            risk: ActionRisk::Low,
            requires_confirmation: require_confirmation,
        });
    }

    if lower.contains("spotify") || lower.contains("playlist") || lower.contains("toca") {
        actions.push(ActionSpec {
            action_type: "spotify_play".to_string(),
            label: "Tocar conteudo no Spotify".to_string(),
            args: json!({ "query": content }),
            risk: ActionRisk::Low,
            requires_confirmation: require_confirmation,
        });
    }

    let text = if actions.is_empty() {
        "OpenAI ainda nao esta conectada. Salve sua API key em Integracoes para respostas inteligentes completas.".to_string()
    } else {
        "OpenAI ainda nao esta conectada, mas preparei acoes locais basicas a partir do seu pedido.".to_string()
    };

    let message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: text,
        created_at: Utc::now().to_rfc3339(),
        tool_calls: if actions.is_empty() { None } else { Some(actions.clone()) },
        status: Some(if actions.is_empty() { "done" } else { "pending" }.to_string()),
    };

    ChatResult {
        message,
        proposed_actions: actions,
    }
}
