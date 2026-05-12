"""Execucao local segura para o assistente no Windows."""

from __future__ import annotations

import os
import shlex
import subprocess
import urllib.parse
from dataclasses import asdict
from pathlib import Path
from typing import Any

PROCESS_ALIASES = {
    "chrome": "chrome.exe",
    "google chrome": "chrome.exe",
    "edge": "msedge.exe",
    "discord": "Discord.exe",
    "spotify": "Spotify.exe",
    "word": "WINWORD.EXE",
    "excel": "EXCEL.EXE",
    "powerpoint": "POWERPNT.EXE",
    "notepad": "notepad.exe",
    "bloco de notas": "notepad.exe",
    "calculadora": "CalculatorApp.exe",
    "explorador": "explorer.exe",
    "explorer": "explorer.exe",
}


def expand_path(value: str) -> str:
    return os.path.expandvars(os.path.expanduser(value))


def open_url(url: str) -> None:
    url = (url or "").strip()
    if not url:
        raise ValueError("URL vazia.")
    if not url.startswith(("http://", "https://", "spotify:", "msteams:", "shell:", "mailto:")):
        url = "https://" + url
    os.startfile(url)  # type: ignore[attr-defined]


def open_app(target: str, args: str = "") -> None:
    target = expand_path((target or "").strip())
    if not target:
        raise ValueError("Aplicativo vazio.")
    if target.startswith(("http://", "https://", "spotify:", "msteams:", "shell:")) or target.endswith(":"):
        open_url(target)
        return

    if Path(target).exists():
        command = [target]
        if args:
            command.extend(shlex.split(args, posix=False))
        subprocess.Popen(command, shell=False)
        return

    try:
        command = [target]
        if args:
            command.extend(shlex.split(args, posix=False))
        subprocess.Popen(command, shell=False)
    except OSError:
        start_args = ["cmd.exe", "/c", "start", "", target]
        if args:
            start_args.extend(shlex.split(args, posix=False))
        subprocess.Popen(start_args, shell=False)


def open_path(path: str) -> None:
    expanded = expand_path(path)
    if not expanded:
        raise ValueError("Caminho vazio.")
    os.startfile(expanded)  # type: ignore[attr-defined]


def search_web(query: str) -> None:
    encoded = urllib.parse.quote_plus(query)
    open_url(f"https://www.google.com/search?q={encoded}")


def search_youtube(query: str) -> None:
    encoded = urllib.parse.quote_plus(query)
    open_url(f"https://www.youtube.com/results?search_query={encoded}")


def close_app(target: str, process_name: str = "") -> None:
    clean_target = (target or "").strip().lower()
    process = process_name or PROCESS_ALIASES.get(clean_target, clean_target)
    if not process:
        raise ValueError("Aplicativo para fechar nao informado.")
    if not process.lower().endswith(".exe"):
        process += ".exe"
    subprocess.run(["taskkill", "/IM", process, "/T", "/F"], check=False, capture_output=True, text=True)


def copy_text(text: str) -> None:
    subprocess.run("clip", input=text, text=True, check=True)


def press_media_key(key: str, count: int = 1) -> None:
    keyboard_names = {
        "play_pause": "play/pause media",
        "next": "next track",
        "previous": "previous track",
        "volume_up": "volume up",
        "volume_down": "volume down",
        "volume_mute": "volume mute",
    }
    try:
        import keyboard

        for _ in range(max(1, count)):
            keyboard.press_and_release(keyboard_names[key])
        return
    except Exception:
        pass

    key_map = {
        "play_pause": "{MEDIA_PLAY_PAUSE}",
        "next": "{MEDIA_NEXT_TRACK}",
        "previous": "{MEDIA_PREV_TRACK}",
        "volume_up": "{VOLUME_UP}",
        "volume_down": "{VOLUME_DOWN}",
        "volume_mute": "{VOLUME_MUTE}",
    }
    token = key_map[key]
    script = (
        "Add-Type -AssemblyName System.Windows.Forms; "
        + "; ".join([f"[System.Windows.Forms.SendKeys]::SendWait('{token}')" for _ in range(max(1, count))])
    )
    subprocess.run(["powershell", "-NoProfile", "-Command", script], check=False)


def set_system_volume(level: int) -> None:
    level = max(0, min(100, int(level)))
    press_media_key("volume_down", 50)
    press_media_key("volume_up", max(0, round(level / 2)))


def safe_dict(obj: Any) -> dict[str, Any]:
    if hasattr(obj, "__dataclass_fields__"):
        return asdict(obj)
    if isinstance(obj, dict):
        return obj
    return {"value": str(obj)}
