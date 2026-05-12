"""Roteador seguro entre intencao, acao e execucao."""

from __future__ import annotations

import json
import os
from dataclasses import asdict, dataclass, field
from datetime import datetime
from typing import Any

from intent_parser import CommandIntent, parse_command
from local_executor import copy_text, open_app, open_path, open_url, search_web, set_system_volume
from spotify_controller import next_track, pause, play, previous_track, volume_down, volume_up


@dataclass
class ActionSpec:
    type: str
    label: str
    args: dict[str, Any] = field(default_factory=dict)
    risk: str = "low"
    requires_confirmation: bool = False
    status: str = "pending"


@dataclass
class ExecutionResult:
    success: bool
    message: str
    intent: CommandIntent | None = None
    action: ActionSpec | None = None
    logs: list[str] = field(default_factory=list)
    needs_confirmation: bool = False


class CommandRouter:
    def __init__(self, config: dict[str, Any], log_path: str = "assistant_logs.jsonl"):
        self.config = config
        self.log_path = log_path
        self.pending_action: ActionSpec | None = None

    def handle_text(self, text: str, confirmed: bool = False) -> ExecutionResult:
        intent = parse_command(text, self.config)
        if intent.type == "confirm" and self.pending_action:
            action = self.pending_action
            self.pending_action = None
            return self.execute(action, intent=intent, confirmed=True)
        if intent.type == "cancel":
            self.pending_action = None
            return self._result(False, "Comando cancelado.", intent=intent)

        action = self.intent_to_action(intent)
        if not action:
            return self._result(False, self.help_text() if intent.type == "list_commands" else "Nao entendi um comando seguro para executar.", intent=intent)

        if action.requires_confirmation and not confirmed:
            self.pending_action = action
            return self._result(
                False,
                f"Confirmar acao: {action.label}? Diga 'confirmar' ou clique em executar.",
                intent=intent,
                action=action,
                needs_confirmation=True,
            )
        return self.execute(action, intent=intent, confirmed=confirmed)

    def intent_to_action(self, intent: CommandIntent) -> ActionSpec | None:
        apps = self.config.get("apps", {})
        sites = self.config.get("sites", {})
        folders = self.config.get("pastas", {})
        spotify_cfg = self.config.get("spotify", {})

        if intent.type == "open_app":
            clean_target = clean_name(intent.target)
            target = apps.get(clean_target, apps.get(intent.target, clean_target))
            return ActionSpec("open_app", f"Abrir {clean_target}", {"target": target})
        if intent.type == "open_site":
            clean_target = clean_name(intent.target)
            target = sites.get(clean_target, sites.get(intent.target, clean_target))
            return ActionSpec("open_url", f"Abrir {clean_target}", {"url": target})
        if intent.type == "open_folder":
            clean_target = clean_name(intent.target)
            target = folders.get(clean_target, folders.get(intent.target, clean_target))
            return ActionSpec("open_path", f"Abrir pasta {clean_target}", {"path": target})
        if intent.type == "search_web":
            return ActionSpec("search_web", f"Pesquisar {intent.target}", {"query": intent.target})
        if intent.type == "spotify_play":
            clean_target = clean_name(intent.target)
            query = (
                spotify_cfg.get(clean_target)
                or spotify_cfg.get(clean_target.replace(" ", "_"))
                or clean_target
                or spotify_cfg.get("default_query", "lo-fi focus")
            )
            return ActionSpec("spotify_play", f"Tocar {intent.target or query}", {"query": query})
        if intent.type in {"spotify_pause", "spotify_next", "spotify_previous", "volume_up", "volume_down"}:
            labels = {
                "spotify_pause": "Pausar ou continuar musica",
                "spotify_next": "Proxima musica",
                "spotify_previous": "Musica anterior",
                "volume_up": "Aumentar volume",
                "volume_down": "Abaixar volume",
            }
            return ActionSpec(intent.type, labels[intent.type], {})
        if intent.type == "system_volume_set":
            return ActionSpec("system_volume_set", f"Definir volume em {intent.parameters.get('level', 30)}%", intent.parameters)
        if intent.type == "copy_text":
            return ActionSpec("copy_text", "Copiar texto", {"text": intent.target})
        if intent.type == "run_routine":
            routine_name = clean_name(intent.target)
            routines = self.config.get("rotinas", {})
            routine = routines.get(routine_name)
            if not routine and not routine_name.startswith("modo "):
                routine_name = f"modo {routine_name}"
                routine = routines.get(routine_name)
            if routine:
                return ActionSpec("run_routine", f"Executar {routine_name}", {"name": routine_name, "actions": routine})
        return None

    def execute(self, action: ActionSpec, intent: CommandIntent | None = None, confirmed: bool = False) -> ExecutionResult:
        try:
            if action.type == "open_app":
                open_app(str(action.args["target"]))
            elif action.type == "open_url":
                open_url(str(action.args["url"]))
            elif action.type == "open_path":
                open_path(str(action.args["path"]))
            elif action.type == "search_web":
                search_web(str(action.args["query"]))
            elif action.type == "spotify_play":
                play(self._resolve_spotify_value(str(action.args["query"])))
            elif action.type == "spotify_pause":
                pause()
            elif action.type == "spotify_next":
                next_track()
            elif action.type == "spotify_previous":
                previous_track()
            elif action.type == "volume_up":
                volume_up(int(self.config.get("volume_step", 5)))
            elif action.type == "volume_down":
                volume_down(int(self.config.get("volume_step", 5)))
            elif action.type == "system_volume_set":
                set_system_volume(int(action.args.get("level", 30)))
            elif action.type == "copy_text":
                copy_text(str(action.args["text"]))
            elif action.type == "run_routine":
                for item in action.args.get("actions", []):
                    nested = ActionSpec(**item)
                    nested.requires_confirmation = False
                    self.execute(nested, intent=intent, confirmed=True)
            else:
                return self._result(False, "Acao ainda nao implementada.", intent=intent, action=action)
        except Exception as exc:
            return self._result(False, f"Falha ao executar: {exc}", intent=intent, action=action)

        action.status = "done"
        return self._result(True, f"Executado: {action.label}", intent=intent, action=action)

    def help_text(self) -> str:
        return (
            "Comandos: abre Chrome, abre YouTube, pesquisa clima, toca lo-fi, "
            "pausa Spotify, proxima musica, aumenta volume, ativar modo estudo."
        )

    def _resolve_spotify_value(self, value: str) -> str:
        spotify_cfg = self.config.get("spotify", {})
        return spotify_cfg.get(value) or spotify_cfg.get(value.replace(" ", "_")) or value

    def _result(
        self,
        success: bool,
        message: str,
        intent: CommandIntent | None = None,
        action: ActionSpec | None = None,
        needs_confirmation: bool = False,
    ) -> ExecutionResult:
        result = ExecutionResult(success, message, intent, action, [message], needs_confirmation)
        self.log_result(result)
        return result

    def log_result(self, result: ExecutionResult) -> None:
        entry = {
            "time": datetime.now().isoformat(timespec="seconds"),
            "success": result.success,
            "message": result.message,
            "intent": asdict(result.intent) if result.intent else None,
            "action": asdict(result.action) if result.action else None,
        }
        with open(self.log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")


def load_assistant_config(path: str = "assistant_config.json") -> dict[str, Any]:
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def clean_name(value: str) -> str:
    value = (value or "").strip()
    for prefix in ("o ", "a ", "os ", "as ", "um ", "uma "):
        if value.startswith(prefix):
            return value[len(prefix) :].strip()
    return value
