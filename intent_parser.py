"""Parser de comandos em portugues-BR para o assistente desktop."""

from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field
from typing import Any

from openai_controller import interpret_intent


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


COMMAND_STARTERS = (
    "abre ",
    "abrir ",
    "abra ",
    "inicia ",
    "iniciar ",
    "fecha ",
    "fechar ",
    "encerra ",
    "encerrar ",
    "toca ",
    "tocar ",
    "toque ",
    "coloca ",
    "colocar ",
    "coloque ",
    "pesquisa ",
    "pesquisar ",
    "procura ",
    "procurar ",
    "busca ",
    "buscar ",
    "pausa",
    "pause",
    "proxima",
    "anterior",
    "aumenta",
    "abaixa",
    "diminui",
    "volume",
    "ativar ",
    "ativa ",
    "modo ",
    "copiar ",
    "copia ",
)


def normalize_text(text: str) -> str:
    text = unicodedata.normalize("NFD", text.lower())
    text = "".join(ch for ch in text if unicodedata.category(ch) != "Mn")
    text = text.replace("youtube", "you tube")
    text = re.sub(r"\s+", " ", text).strip()
    return text.replace("you tube", "youtube")


def strip_wake_words(text: str, wake_words: list[str]) -> str:
    norm = normalize_text(text)
    for wake in wake_words:
        wake_norm = normalize_text(wake)
        if norm == wake_norm:
            return ""
        if norm.startswith(wake_norm + " "):
            return norm[len(wake_norm) + 1 :].strip()
    return norm


def parse_command(text: str, config: dict[str, Any] | None = None) -> CommandIntent:
    config = config or {}
    ai_intent = _parse_with_openai(text, config)
    if ai_intent and ai_intent.type != "unknown" and ai_intent.confidence >= 0.65:
        return ai_intent
    return parse_local(text, config)


def parse_local(text: str, config: dict[str, Any] | None = None) -> CommandIntent:
    config = config or {}
    raw = text.strip()
    norm = strip_wake_words(raw, config.get("wake_words", []))

    if not norm:
        return CommandIntent("wake", confidence=0.95, raw_text=raw)

    if norm in {"sim", "confirmar", "confirma", "pode", "pode executar", "executar"}:
        return CommandIntent("confirm", confidence=0.99, raw_text=raw)
    if norm in {"nao", "não", "cancelar", "cancela", "para", "parar"}:
        return CommandIntent("cancel", confidence=0.99, raw_text=raw)

    sequence = _extract_sequence(norm)
    if sequence:
        return CommandIntent("sequence", parameters={"steps": sequence}, confidence=0.84, raw_text=raw)

    youtube_search = _match_youtube_search(norm)
    if youtube_search:
        return CommandIntent("search_youtube", youtube_search, confidence=0.91, raw_text=raw)

    close_target = _match_after(norm, ["fecha", "fechar", "encerra", "encerrar", "finaliza", "finalizar"])
    if close_target:
        return CommandIntent("close_app", close_target, confidence=0.86, raw_text=raw)

    routine = _match_after(norm, ["ativar rotina", "ativa rotina", "iniciar rotina", "inicia rotina", "ativar modo", "ativa modo", "iniciar modo", "inicia modo", "modo"])
    if routine:
        return CommandIntent("run_routine", routine, confidence=0.86, raw_text=raw)

    open_target = _match_after(norm, ["abre", "abrir", "abra", "inicia", "iniciar", "inicie"])
    if open_target:
        sites = config.get("sites", {})
        folders = config.get("pastas", {})
        clean_target = _strip_articles(open_target)
        if clean_target in sites or _looks_like_url(clean_target):
            return CommandIntent("open_site", clean_target, confidence=0.92, raw_text=raw)
        if clean_target in folders or clean_target.startswith(("pasta ", "minha pasta ")):
            clean_target = clean_target.replace("minha pasta ", "").replace("pasta ", "").strip()
            return CommandIntent("open_folder", clean_target, confidence=0.86, raw_text=raw)
        return CommandIntent("open_app", open_target, confidence=0.88, raw_text=raw)

    web_search = _match_after(norm, ["pesquisa por", "pesquisar por", "pesquisa", "pesquisar", "procura por", "procurar por", "procura", "procurar", "busca", "buscar"])
    if web_search:
        return CommandIntent("search_web", web_search, confidence=0.86, raw_text=raw)

    spotify_query = _match_after(
        norm,
        [
            "spotify toca",
            "spotify tocar",
            "spotify toque",
            "spotify coloca",
            "spotify colocar",
            "spotify coloque",
            "toca",
            "tocar",
            "toque",
            "coloca",
            "colocar",
            "coloque",
        ],
    )
    if spotify_query:
        spotify_query = _strip_music_words(spotify_query)
        return CommandIntent("spotify_play", spotify_query, confidence=0.88, raw_text=raw)
    if "pausa spotify" in norm or norm in {"pausa musica", "pausar musica", "pause musica", "pausa", "pause"}:
        return CommandIntent("spotify_pause", confidence=0.9, raw_text=raw)
    if "proxima" in norm or "passa musica" in norm or "pular musica" in norm:
        return CommandIntent("spotify_next", confidence=0.9, raw_text=raw)
    if "anterior" in norm or "volta musica" in norm or "musica anterior" in norm:
        return CommandIntent("spotify_previous", confidence=0.9, raw_text=raw)

    if "aumenta volume" in norm or "volume mais alto" in norm or norm == "aumenta o som":
        return CommandIntent("volume_up", confidence=0.86, raw_text=raw)
    if "abaixa volume" in norm or "diminui volume" in norm or "volume baixo" in norm or norm == "diminui o som":
        return CommandIntent("volume_down", confidence=0.86, raw_text=raw)
    volume_match = re.search(r"volume (?:em|para|no) (\d{1,3})", norm)
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

    if norm.startswith(("resume ", "resumir ", "resuma ", "me explica ", "explique ")):
        return CommandIntent(
            "assistant_answer",
            parameters={"prompt": raw},
            confidence=0.75,
            raw_text=raw,
        )
    if norm.endswith("?") or norm.startswith(("o que e ", "quem e ", "como faco ", "como fazer ")):
        return CommandIntent("assistant_answer", parameters={"prompt": raw}, confidence=0.7, raw_text=raw)

    if "listar comandos" in norm or "o que voce faz" in norm or norm in {"ajuda", "comandos"}:
        return CommandIntent("list_commands", confidence=0.9, raw_text=raw)

    return CommandIntent("unknown", norm, confidence=0.2, raw_text=raw)


def _parse_with_openai(text: str, config: dict[str, Any]) -> CommandIntent | None:
    data = interpret_intent(text, config)
    if not data:
        return None
    return CommandIntent(
        type=str(data.get("type", "unknown")),
        target=str(data.get("target", "")),
        parameters=data.get("parameters") or {},
        confidence=float(data.get("confidence", 0.5)),
        requires_confirmation=bool(data.get("requires_confirmation", False)),
        raw_text=text,
    )


def _extract_sequence(text: str) -> list[str]:
    open_many = _match_open_many(text)
    if open_many:
        return open_many

    normalized = text.replace(" e depois ", " depois ")
    parts = [part.strip(" ,;") for part in re.split(r"\s+depois\s+|;\s*", normalized) if part.strip(" ,;")]
    if len(parts) > 1:
        return _complete_followup_steps(parts)

    if " e " not in text:
        return []
    left, right = [part.strip() for part in text.split(" e ", 1)]
    if right.startswith(COMMAND_STARTERS):
        return _complete_followup_steps([left, right])
    return []


def _match_open_many(text: str) -> list[str]:
    open_prefix = re.match(r"^(abre|abrir|abra|inicia|iniciar|inicie)\s+(.+)$", text)
    if not open_prefix:
        return []
    targets_text = open_prefix.group(2)
    if any(marker in targets_text for marker in (" depois ", " toca ", " pesquisa ", " pausa ", " fecha ")):
        return []
    separators = re.split(r"\s*,\s*|\s+e\s+", targets_text)
    targets = [_strip_articles(part.strip()) for part in separators if part.strip()]
    if len(targets) <= 1:
        return []
    return [f"abrir {target}" for target in targets]


def _complete_followup_steps(parts: list[str]) -> list[str]:
    completed: list[str] = []
    last_open_prefix = ""
    for part in parts:
        if part.startswith(COMMAND_STARTERS):
            completed.append(part)
            if part.startswith(("abre ", "abrir ", "abra ", "inicia ", "iniciar ")):
                last_open_prefix = "abrir"
            continue
        if last_open_prefix:
            completed.append(f"{last_open_prefix} {part}")
        else:
            completed.append(part)
    return completed


def _match_youtube_search(text: str) -> str:
    patterns = [
        r"^(?:pesquisa|pesquisar|procura|procurar|busca|buscar)\s+(.+?)\s+(?:no|na|pelo|para o)\s+youtube$",
        r"^(?:abre|abrir|abra)\s+(?:videos?|pesquisa)\s+(.+?)\s+(?:no|na|pelo|para o)\s+youtube$",
        r"^(?:videos?|video)\s+de\s+(.+?)\s+(?:no|na|pelo|para o)\s+youtube$",
    ]
    for pattern in patterns:
        match = re.match(pattern, text)
        if match:
            return match.group(1).strip()
    return ""


def _match_after(text: str, prefixes: list[str]) -> str:
    for prefix in prefixes:
        if text.startswith(prefix + " "):
            return text[len(prefix) + 1 :].strip()
    return ""


def _strip_articles(value: str) -> str:
    value = (value or "").strip()
    for prefix in ("o ", "a ", "os ", "as ", "um ", "uma ", "meu ", "minha "):
        if value.startswith(prefix):
            return value[len(prefix) :].strip()
    return value


def _strip_music_words(value: str) -> str:
    value = _strip_articles(value)
    for prefix in ("musica de ", "musicas de ", "playlist de "):
        if value.startswith(prefix):
            return value[len(prefix) :].strip()
    return value


def _looks_like_url(value: str) -> bool:
    return value.startswith(("http://", "https://", "www.")) or "." in value
