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
from array import array
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
    audio_level: float = 0.0
    device_name: str = ""


class VoiceEngine:
    def __init__(
        self,
        wake_words: list[str],
        language: str = "pt-BR",
        listen_seconds: int = 7,
        device: int | str | None = None,
        on_text: Callable[[str], None] | None = None,
        on_state: Callable[[VoiceEngineState], None] | None = None,
    ):
        self.wake_words = [w.lower() for w in wake_words]
        self.language = language
        self.listen_seconds = listen_seconds
        self.device = device
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
            if self.device is not None and str(self.device).strip() != "":
                sd.default.device = (self.device, None)
            try:
                device_info = sd.query_devices(sd.default.device[0] if isinstance(sd.default.device, (list, tuple)) else sd.default.device, "input")
                self.state.device_name = str(device_info.get("name", "Microfone"))
            except Exception:
                self.state.device_name = "Microfone padrao"
            self.state.model_ready = True
            self._emit_state()

            def callback(indata, frames, time_info, status):  # noqa: ANN001
                if status:
                    self.state.last_error = str(status)
                    self._emit_state()
                try:
                    raw = bytes(indata)
                    self.state.audio_level = audio_level(raw)
                    self._audio_q.put_nowait(raw)
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


def audio_level(raw_audio: bytes) -> float:
    if not raw_audio:
        return 0.0
    samples = array("h")
    samples.frombytes(raw_audio)
    if not samples:
        return 0.0
    avg = sum(abs(sample) for sample in samples) / len(samples)
    return max(0.0, min(1.0, avg / 12000))


def list_input_devices() -> list[dict[str, str | int]]:
    try:
        import sounddevice as sd
    except Exception as exc:
        return [{"index": -1, "name": f"sounddevice indisponivel: {exc}"}]
    devices: list[dict[str, str | int]] = []
    for index, item in enumerate(sd.query_devices()):
        if int(item.get("max_input_channels", 0)) > 0:
            devices.append({"index": index, "name": str(item.get("name", f"Microfone {index}"))})
    return devices


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
