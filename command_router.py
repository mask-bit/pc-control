"""Roteador seguro entre intencao, acao local e resultado visual."""

from __future__ import annotations

import json
import os
from dataclasses import asdict, dataclass, field
from datetime import datetime
from typing import Any

from intent_parser import CommandIntent, parse_command, parse_local
from local_executor import (
    close_app,
    copy_text,
    open_app,
    open_path,
    open_url,
    search_web,
    search_youtube,
    set_system_volume,
)
from openai_controller import answer_with_openai
from spotify_controller import next_track, pause, play, previous_track, volume_down, volume_up
from spotify_controller import set_volume as set_spotify_volume


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
    step_results: list[str] = field(default_factory=list)


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
            message = self.help_text() if intent.type == "list_commands" else "Nao entendi um comando seguro para executar."
            return self._result(False, message, intent=intent)

        if self._needs_confirmation(action) and not confirmed:
            self.pending_action = action
            return self._result(
                False,
                f"Confirmar acao: {action.label}? Diga 'confirmar' ou clique em Confirmar.",
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

        if intent.type == "sequence":
            actions: list[dict[str, Any]] = []
            labels: list[str] = []
            for step in intent.parameters.get("steps", []):
                step_intent = parse_local(str(step), self.config)
                step_action = self.intent_to_action(step_intent)
                if step_action:
                    actions.append(asdict(step_action))
                    labels.append(step_action.label)
            if actions:
                return ActionSpec("sequence", "Executar sequencia: " + " -> ".join(labels), {"actions": actions})
            return None

        if intent.type == "open_app":
            clean_target = clean_name(intent.target)
            if clean_target in sites:
                return ActionSpec("open_url", f"Abrir {clean_target}", {"url": sites[clean_target]})
            target = apps.get(clean_target, apps.get(intent.target, clean_target))
            return ActionSpec("open_app", f"Abrir {clean_target}", {"target": target})
        if intent.type in {"open_site", "open_url"}:
            clean_target = clean_name(intent.target)
            target = sites.get(clean_target, sites.get(intent.target, clean_target))
            return ActionSpec("open_url", f"Abrir {clean_target}", {"url": target})
        if intent.type == "open_folder":
            clean_target = clean_name(intent.target)
            target = folders.get(clean_target, folders.get(intent.target, clean_target))
            return ActionSpec("open_path", f"Abrir pasta {clean_target}", {"path": target})
        if intent.type == "search_web":
            return ActionSpec("search_web", f"Pesquisar {intent.target}", {"query": intent.target})
        if intent.type == "search_youtube":
            return ActionSpec("search_youtube", f"Pesquisar {intent.target} no YouTube", {"query": intent.target})
        if intent.type == "close_app":
            clean_target = clean_name(intent.target)
            process = self.config.get("processos", {}).get(clean_target, "")
            return ActionSpec("close_app", f"Fechar {clean_target}", {"target": clean_target, "process": process}, risk="medium")
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
            level = int(intent.parameters.get("level", 30))
            return ActionSpec("system_volume_set", f"Definir volume em {level}%", {"level": level})
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
        if intent.type == "assistant_answer":
            prompt = intent.parameters.get("prompt") or intent.target or intent.raw_text
            return ActionSpec("assistant_answer", "Responder com IA", {"prompt": prompt})
        return None

    def execute(self, action: ActionSpec, intent: CommandIntent | None = None, confirmed: bool = False) -> ExecutionResult:
        step_results: list[str] = []
        try:
            if action.type == "sequence":
                for item in action.args.get("actions", []):
                    nested = ActionSpec(**item)
                    nested.requires_confirmation = False
                    result = self.execute(nested, intent=intent, confirmed=True)
                    step_results.append(result.message)
                action.status = "done"
                return self._result(True, "Sequencia executada.", intent=intent, action=action, step_results=step_results)
            if action.type == "open_app":
                open_app(str(action.args["target"]))
            elif action.type == "open_url":
                open_url(str(action.args["url"]))
            elif action.type == "open_path":
                open_path(str(action.args["path"]))
            elif action.type == "search_web":
                search_web(str(action.args["query"]))
            elif action.type == "search_youtube":
                search_youtube(str(action.args["query"]))
            elif action.type == "close_app":
                close_app(str(action.args["target"]), str(action.args.get("process") or ""))
            elif action.type == "spotify_play":
                play(self._resolve_spotify_value(str(action.args["query"])), self.config)
            elif action.type == "spotify_pause":
                pause(self.config)
            elif action.type == "spotify_next":
                next_track(self.config)
            elif action.type == "spotify_previous":
                previous_track(self.config)
            elif action.type == "volume_up":
                volume_up(int(self.config.get("volume_step", 5)))
            elif action.type == "volume_down":
                volume_down(int(self.config.get("volume_step", 5)))
            elif action.type == "system_volume_set":
                level = int(action.args.get("level", 30))
                set_system_volume(level)
                set_spotify_volume(level, self.config)
            elif action.type == "copy_text":
                copy_text(str(action.args["text"]))
            elif action.type == "run_routine":
                for item in action.args.get("actions", []):
                    nested = ActionSpec(**item)
                    nested.requires_confirmation = False
                    result = self.execute(nested, intent=intent, confirmed=True)
                    step_results.append(result.message)
                action.status = "done"
                return self._result(True, f"Rotina executada: {action.args.get('name')}", intent=intent, action=action, step_results=step_results)
            elif action.type == "assistant_answer":
                message = answer_with_openai(str(action.args.get("prompt", "")), self.config)
                action.status = "done"
                return self._result(True, message, intent=intent, action=action)
            else:
                return self._result(False, "Acao ainda nao implementada.", intent=intent, action=action)
        except Exception as exc:
            action.status = "failed"
            return self._result(False, f"Falha ao executar: {exc}", intent=intent, action=action)

        action.status = "done"
        return self._result(True, f"Executado: {action.label}", intent=intent, action=action)

    def help_text(self) -> str:
        return (
            "Voce pode dizer: abrir Chrome, abrir YouTube, pesquisar lo-fi no YouTube, "
            "abrir Chrome e Spotify, tocar playlist foco, pausar musica, proxima musica, "
            "fechar Spotify, volume para 30, ativar modo estudo."
        )

    def _resolve_spotify_value(self, value: str) -> str:
        spotify_cfg = self.config.get("spotify", {})
        return spotify_cfg.get(value) or spotify_cfg.get(value.replace(" ", "_")) or value

    def _needs_confirmation(self, action: ActionSpec) -> bool:
        if action.requires_confirmation:
            return True
        if not self.config.get("confirmar_acoes_sensiveis", True):
            return False
        if action.risk in {"high", "critical"}:
            return True
        if action.type == "sequence":
            return any(ActionSpec(**item).requires_confirmation for item in action.args.get("actions", []))
        return False

    def _result(
        self,
        success: bool,
        message: str,
        intent: CommandIntent | None = None,
        action: ActionSpec | None = None,
        needs_confirmation: bool = False,
        step_results: list[str] | None = None,
    ) -> ExecutionResult:
        result = ExecutionResult(success, message, intent, action, [message], needs_confirmation, step_results or [])
        self.log_result(result)
        return result

    def log_result(self, result: ExecutionResult) -> None:
        entry = {
            "time": datetime.now().isoformat(timespec="seconds"),
            "success": result.success,
            "message": result.message,
            "intent": asdict(result.intent) if result.intent else None,
            "action": asdict(result.action) if result.action else None,
            "steps": result.step_results,
        }
        with open(self.log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")


def load_assistant_config(path: str = "assistant_config.json") -> dict[str, Any]:
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def clean_name(value: str) -> str:
    value = (value or "").strip().lower()
    for prefix in ("o ", "a ", "os ", "as ", "um ", "uma ", "meu ", "minha "):
        if value.startswith(prefix):
            return value[len(prefix) :].strip()
    return value
