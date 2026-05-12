"""Motor de voz offline com Vosk para comandos do assistente."""

from __future__ import annotations

import json
import os
import queue
import shutil
import threading
import time
import urllib.request
import zipfile
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime


@dataclass
class VoiceEngineState:
    listening: bool = False
    paused: bool = False
    last_text: str = ""
    last_error: str = ""
    model_ready: bool = False


class VoiceEngine:
    def __init__(
        self,
        wake_words: list[str],
        language: str = "pl",
        listen_seconds: int = 7,
        on_text: Callable[[str], None] | None = None,
        on_state: Callable[[VoiceEngineState], None] | None = None,
    ):
        self.wake_words = [w.lower() for w in wake_words]
        self.language = language
        self.listen_seconds = listen_seconds
        self.on_text = on_text
        self.on_state = on_state
        self.state = VoiceEngineState()
        self._running = False
        self._armed_until = 0.0
        self._audio_q: queue.Queue[bytes] = queue.Queue(maxsize=80)
        self._stream = None

    def start(self) -> None:
        if self._running:
            return
        self._running = True
        threading.Thread(target=self._run, daemon=True).start()

    def stop(self) -> None:
        self._running = False
        if self._stream:
            try:
                self._stream.stop()
                self._stream.close()
            except Exception:
                pass

    def arm(self) -> None:
        self._armed_until = time.time() + self.listen_seconds
        self.state.listening = True
        self._emit_state()

    def pause(self) -> None:
        self.state.paused = True
        self.state.listening = False
        self._emit_state()

    def resume(self) -> None:
        self.state.paused = False
        self._emit_state()

    def _run(self) -> None:
        try:
            import sounddevice as sd
            from vosk import KaldiRecognizer, Model, SetLogLevel

            SetLogLevel(-1)
            model_path = get_vosk_model_path(self.language)
            recognizer = KaldiRecognizer(Model(model_path), 16000)
            self.state.model_ready = True
            self._emit_state()

            def callback(indata, frames, time_info, status):  # noqa: ANN001
                if status:
                    self.state.last_error = str(status)
                    self._emit_state()
                try:
                    self._audio_q.put_nowait(bytes(indata))
                except queue.Full:
                    pass

            self._stream = sd.RawInputStream(
                samplerate=16000,
                blocksize=8000,
                dtype="int16",
                channels=1,
                callback=callback,
            )
            self._stream.start()
            while self._running:
                if self.state.paused:
                    time.sleep(0.2)
                    continue
                data = self._audio_q.get()
                if recognizer.AcceptWaveform(data):
                    result = json.loads(recognizer.Result())
                    text = (result.get("text") or "").strip()
                    if text:
                        self._handle_text(text)
                if self.state.listening and time.time() > self._armed_until:
                    self.state.listening = False
                    self._emit_state()
        except Exception as exc:
            self.state.last_error = f"{exc}. Instale dependencias com: pip install -r requirements.txt"
            self._emit_state()

    def _handle_text(self, text: str) -> None:
        lower = text.lower()
        self.state.last_text = text
        woke = any(wake in lower.split() or lower.startswith(wake + " ") for wake in self.wake_words)
        if woke:
            self.arm()
            command = lower
            for wake in self.wake_words:
                if command.startswith(wake + " "):
                    command = command[len(wake) + 1 :].strip()
                    break
            if command and command not in self.wake_words and self.on_text:
                self.on_text(command)
            self._emit_state()
            return
        if self.state.listening and self.on_text:
            self.on_text(text)
        self._emit_state()

    def _emit_state(self) -> None:
        if self.on_state:
            self.on_state(self.state)


def timestamp() -> str:
    return datetime.now().strftime("%H:%M:%S")


VOSK_MODELS = {
    "pt": (
        "vosk-model-small-pt-0.3",
        "https://alphacephei.com/vosk/models/vosk-model-small-pt-0.3.zip",
    ),
    "pt-BR": (
        "vosk-model-small-pt-0.3",
        "https://alphacephei.com/vosk/models/vosk-model-small-pt-0.3.zip",
    ),
    "en": (
        "vosk-model-small-en-us-0.15",
        "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip",
    ),
}


def get_vosk_model_path(language: str) -> str:
    key = language if language in VOSK_MODELS else "pt-BR"
    model_name, url = VOSK_MODELS[key]
    base_dir = os.path.join(os.path.expanduser("~"), "vosk_models")
    model_dir = os.path.join(base_dir, model_name)
    if os.path.isdir(model_dir):
        return model_dir

    os.makedirs(base_dir, exist_ok=True)
    zip_path = model_dir + ".zip"
    urllib.request.urlretrieve(url, zip_path)
    tmp_dir = model_dir + "_tmp"
    if os.path.isdir(tmp_dir):
        shutil.rmtree(tmp_dir)
    os.makedirs(tmp_dir, exist_ok=True)
    with zipfile.ZipFile(zip_path, "r") as zf:
        zf.extractall(tmp_dir)
    extracted = os.path.join(tmp_dir, model_name)
    if os.path.isdir(extracted):
        shutil.move(extracted, model_dir)
    else:
        shutil.move(tmp_dir, model_dir)
    if os.path.isdir(tmp_dir):
        shutil.rmtree(tmp_dir, ignore_errors=True)
    try:
        os.remove(zip_path)
    except OSError:
        pass
    return model_dir
