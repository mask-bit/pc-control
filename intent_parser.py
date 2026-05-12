"""Parser local de comandos em portugues-BR para o assistente de voz."""

from __future__ import annotations

import json
import os
import re
import unicodedata
import urllib.request
from dataclasses import dataclass, field
from typing import Any


@dataclass
class VoiceEvent:
    text: str
    confidence: float = 1.0
    source: str = "text"
    timestamp: str = ""


@dataclass
class CommandIntent:
    type: str
    target: str = ""
    parameters: dict[str, Any] = field(default_factory=dict)
    confidence: float = 0.0
    requires_confirmation: bool = False
    raw_text: str = ""


def normalize_text(text: str) -> str:
    text = unicodedata.normalize("NFD", text.lower())
    text = "".join(ch for ch in text if unicodedata.category(ch) != "Mn")
    return re.sub(r"\s+", " ", text).strip()


def strip_wake_words(text: str, wake_words: list[str]) -> str:
    norm = normalize_text(text)
    for wake in wake_words:
        wake_norm = normalize_text(wake)
        if norm == wake_norm:
            return ""
        if norm.startswith(wake_norm + " "):
            return norm[len(wake_norm) + 1 :].strip()
    return norm


def parse_local(text: str, config: dict[str, Any] | None = None) -> CommandIntent:
    config = config or {}
    raw = text.strip()
    norm = strip_wake_words(raw, config.get("wake_words", []))

    if not norm:
        return CommandIntent("wake", confidence=0.95, raw_text=raw)

    if norm in {"sim", "confirmar", "confirma", "pode", "pode executar", "executar"}:
        return CommandIntent("confirm", confidence=0.99, raw_text=raw)
    if norm in {"nao", "cancelar", "cancela", "para", "parar"}:
        return CommandIntent("cancel", confidence=0.99, raw_text=raw)

    routine = _match_after(norm, ["ativar rotina", "ativa rotina", "ativar modo", "ativa modo", "modo"])
    if routine:
        return CommandIntent("run_routine", routine, confidence=0.85, raw_text=raw)

    if norm.startswith(("abre ", "abrir ", "abra ", "inicia ", "iniciar ")):
        target = re.sub(r"^(abre|abrir|abra|inicia|iniciar)\s+", "", norm).strip()
        sites = config.get("sites", {})
        if target in sites or target.startswith(("http", "www.")):
            return CommandIntent("open_site", target, confidence=0.92, raw_text=raw)
        folders = config.get("pastas", {})
        if target in folders or target.startswith(("pasta ", "minha pasta ")):
            target = target.replace("minha pasta ", "").replace("pasta ", "").strip()
            return CommandIntent("open_folder", target, confidence=0.86, raw_text=raw)
        return CommandIntent("open_app", target, confidence=0.88, raw_text=raw)

    search = _match_after(norm, ["pesquisa por", "pesquisar por", "pesquisa", "pesquisar", "procura por", "procura"])
    if search:
        return CommandIntent("search_web", search, confidence=0.88, raw_text=raw)

    spotify_query = _match_after(norm, ["toca", "toque", "coloca", "coloque", "spotify toca", "spotify coloque"])
    if spotify_query:
        return CommandIntent("spotify_play", spotify_query, confidence=0.88, raw_text=raw)
    if "pausa" in norm or "pause" in norm:
        return CommandIntent("spotify_pause", confidence=0.9, raw_text=raw)
    if "proxima" in norm or "passa musica" in norm or "pular musica" in norm:
        return CommandIntent("spotify_next", confidence=0.9, raw_text=raw)
    if "anterior" in norm or "volta musica" in norm:
        return CommandIntent("spotify_previous", confidence=0.9, raw_text=raw)
    if "aumenta volume" in norm or "volume mais alto" in norm:
        return CommandIntent("volume_up", confidence=0.86, raw_text=raw)
    if "abaixa volume" in norm or "diminui volume" in norm or "volume baixo" in norm:
        return CommandIntent("volume_down", confidence=0.86, raw_text=raw)
    volume_match = re.search(r"volume (?:em|para) (\d{1,3})", norm)
    if volume_match:
        return CommandIntent(
            "system_volume_set",
            parameters={"level": int(volume_match.group(1))},
            confidence=0.86,
            raw_text=raw,
        )

    copy = _match_after(norm, ["copia", "copiar"])
    if copy:
        return CommandIntent("copy_text", copy, confidence=0.78, raw_text=raw)

    if "listar comandos" in norm or "o que voce faz" in norm or "ajuda" == norm:
        return CommandIntent("list_commands", confidence=0.9, raw_text=raw)

    return CommandIntent("unknown", norm, confidence=0.2, raw_text=raw)


def parse_with_openai(text: str, config: dict[str, Any]) -> CommandIntent | None:
    if not config.get("usar_openai"):
        return None
    key = os.environ.get(config.get("openai_api_key_env", "OPENAI_API_KEY"), "").strip()
    if not key:
        return None

    body = {
        "model": config.get("openai_model", "gpt-4.1-mini"),
        "input": [
            {
                "role": "system",
                "content": (
                    "Converta o comando em portugues-BR para JSON com as chaves "
                    "type,target,parameters,confidence,requires_confirmation. "
                    "Tipos permitidos: open_app, open_site, open_folder, search_web, "
                    "spotify_play, spotify_pause, spotify_next, spotify_previous, "
                    "volume_up, volume_down, system_volume_set, copy_text, "
                    "run_routine, list_commands, unknown."
                ),
            },
            {"role": "user", "content": text},
        ],
        "text": {"format": {"type": "json_object"}},
    }
    req = urllib.request.Request(
        "https://api.openai.com/v1/responses",
        data=json.dumps(body).encode("utf-8"),
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=12) as res:
            payload = json.loads(res.read().decode("utf-8"))
    except Exception:
        return None

    output_text = payload.get("output_text", "")
    if not output_text and payload.get("output"):
        parts = payload["output"][0].get("content", [])
        output_text = "".join(part.get("text", "") for part in parts)
    try:
        data = json.loads(output_text)
    except Exception:
        return None
    return CommandIntent(
        type=data.get("type", "unknown"),
        target=data.get("target", ""),
        parameters=data.get("parameters") or {},
        confidence=float(data.get("confidence", 0.5)),
        requires_confirmation=bool(data.get("requires_confirmation", False)),
        raw_text=text,
    )


def parse_command(text: str, config: dict[str, Any] | None = None) -> CommandIntent:
    config = config or {}
    ai_intent = parse_with_openai(text, config)
    if ai_intent and ai_intent.type != "unknown" and ai_intent.confidence >= 0.65:
        return ai_intent
    return parse_local(text, config)


def _match_after(text: str, prefixes: list[str]) -> str:
    for prefix in prefixes:
        if text.startswith(prefix + " "):
            return text[len(prefix) + 1 :].strip()
    return ""
