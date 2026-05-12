"""Integracao opcional com OpenAI para interpretar comandos do Jarvis."""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from typing import Any

from secrets_store import get_secret

INTENT_TYPES = [
    "open_app",
    "open_site",
    "open_folder",
    "open_url",
    "search_web",
    "search_youtube",
    "close_app",
    "spotify_play",
    "spotify_pause",
    "spotify_next",
    "spotify_previous",
    "volume_up",
    "volume_down",
    "system_volume_set",
    "copy_text",
    "run_routine",
    "sequence",
    "assistant_answer",
    "list_commands",
    "unknown",
]


def get_openai_key(config: dict[str, Any] | None = None) -> str:
    config = config or {}
    env_name = config.get("openai_api_key_env", "OPENAI_API_KEY")
    return os.environ.get(env_name, "").strip() or get_secret("openai_api_key")


def openai_enabled(config: dict[str, Any] | None = None) -> bool:
    config = config or {}
    return bool(config.get("usar_openai") and get_openai_key(config))


def interpret_intent(text: str, config: dict[str, Any]) -> dict[str, Any] | None:
    """Return an intent dict from OpenAI, or None on any recoverable failure."""

    if not openai_enabled(config):
        return None

    apps = sorted((config.get("apps") or {}).keys())
    sites = sorted((config.get("sites") or {}).keys())
    routines = sorted((config.get("rotinas") or {}).keys())
    schema = {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "type": {"type": "string", "enum": INTENT_TYPES},
            "target": {"type": "string"},
            "parameters": {"type": "object", "additionalProperties": True},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1},
            "requires_confirmation": {"type": "boolean"},
        },
        "required": ["type", "target", "parameters", "confidence", "requires_confirmation"],
    }
    body = {
        "model": config.get("openai_model", "gpt-4.1-mini"),
        "instructions": (
            "Voce e o interpretador de comandos de um assistente desktop Windows em portugues-BR. "
            "Converta a frase do usuario em uma unica intencao estruturada. "
            "Nunca crie comandos shell livres. Para pedidos compostos, use type=sequence e "
            "parameters.steps como lista de frases curtas executaveis. "
            f"Apps conhecidos: {', '.join(apps[:80])}. "
            f"Sites conhecidos: {', '.join(sites[:80])}. "
            f"Rotinas conhecidas: {', '.join(routines[:80])}."
        ),
        "input": text,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "command_intent",
                "schema": schema,
                "strict": False,
            }
        },
        "max_output_tokens": 350,
    }
    try:
        payload = _post_response(body, get_openai_key(config), timeout=12)
        data = _extract_json(payload)
    except Exception:
        return None
    if not isinstance(data, dict):
        return None
    if data.get("type") not in INTENT_TYPES:
        data["type"] = "unknown"
    data.setdefault("target", "")
    data.setdefault("parameters", {})
    data.setdefault("confidence", 0.5)
    data.setdefault("requires_confirmation", False)
    return data


def answer_with_openai(text: str, config: dict[str, Any]) -> str:
    if not openai_enabled(config):
        return "A OpenAI nao esta conectada. Configure sua chave para respostas inteligentes."
    body = {
        "model": config.get("openai_model", "gpt-4.1-mini"),
        "instructions": (
            "Responda em portugues-BR como um assistente operacional de PC. "
            "Seja curto, util e objetivo. Nao finja executar acoes locais."
        ),
        "input": text,
        "max_output_tokens": 700,
    }
    try:
        payload = _post_response(body, get_openai_key(config), timeout=20)
    except urllib.error.HTTPError as exc:
        return f"Falha na OpenAI: HTTP {exc.code}."
    except Exception as exc:
        return f"Falha na OpenAI: {exc}."
    return _extract_text(payload) or "A IA respondeu sem texto."


def test_openai_connection(config: dict[str, Any]) -> tuple[bool, str]:
    if not get_openai_key(config):
        return False, "Chave da OpenAI nao configurada."
    body = {
        "model": config.get("openai_model", "gpt-4.1-mini"),
        "input": "Responda apenas OK.",
        "max_output_tokens": 8,
    }
    try:
        payload = _post_response(body, get_openai_key(config), timeout=12)
    except Exception as exc:
        return False, f"Falha na conexao: {exc}"
    text = _extract_text(payload)
    return True, text.strip() or "Conexao OK."


def _post_response(body: dict[str, Any], api_key: str, timeout: int) -> dict[str, Any]:
    req = urllib.request.Request(
        "https://api.openai.com/v1/responses",
        data=json.dumps(body).encode("utf-8"),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as res:
        return json.loads(res.read().decode("utf-8"))


def _extract_text(payload: dict[str, Any]) -> str:
    if payload.get("output_text"):
        return str(payload["output_text"])
    parts: list[str] = []
    for item in payload.get("output", []) or []:
        for content in item.get("content", []) or []:
            if "text" in content:
                parts.append(str(content["text"]))
    return "".join(parts).strip()


def _extract_json(payload: dict[str, Any]) -> dict[str, Any] | None:
    text = _extract_text(payload)
    if not text:
        return None
    return json.loads(text)
