"""Execucao local segura para o assistente de voz."""

from __future__ import annotations

import os
import subprocess
import urllib.parse
from dataclasses import asdict
from typing import Any


def expand_path(value: str) -> str:
    return os.path.expandvars(os.path.expanduser(value))


def open_url(url: str) -> None:
    if not url.startswith(("http://", "https://", "spotify:", "msteams:", "shell:")):
        url = "https://" + url
    if url.startswith("shell:"):
        subprocess.Popen(["cmd.exe", "/c", "start", "", url], shell=False)
    else:
        os.startfile(url)  # type: ignore[attr-defined]


def open_app(target: str, args: str = "") -> None:
    target = expand_path(target)
    if target.startswith("shell:"):
        subprocess.Popen(["cmd.exe", "/c", "start", "", target], shell=False)
        return
    if target.endswith(":"):
        os.startfile(target)  # type: ignore[attr-defined]
        return
    if args:
        subprocess.Popen([target, args], shell=False)
    else:
        subprocess.Popen([target], shell=False)


def open_path(path: str) -> None:
    os.startfile(expand_path(path))  # type: ignore[attr-defined]


def search_web(query: str) -> None:
    encoded = urllib.parse.quote_plus(query)
    open_url(f"https://www.google.com/search?q={encoded}")


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

    # nircmd/extra deps seriam mais pesados; PowerShell SendKeys resolve MVP.
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
    # Sem dependencia externa: reduz/aumenta por teclas de midia ate aproximar.
    press_media_key("volume_mute")
    press_media_key("volume_mute")
    press_media_key("volume_down", 50)
    press_media_key("volume_up", max(0, round(level / 2)))


def safe_dict(obj: Any) -> dict[str, Any]:
    if hasattr(obj, "__dataclass_fields__"):
        return asdict(obj)
    if isinstance(obj, dict):
        return obj
    return {"value": str(obj)}
