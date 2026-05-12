"""Painel real do assistente de controle por voz."""

from __future__ import annotations

import json
import threading
from pathlib import Path
from tkinter import messagebox

import customtkinter as ctk

from command_router import CommandRouter, ExecutionResult, load_assistant_config
from voice_engine import VoiceEngine, VoiceEngineState, timestamp

BASE_DIR = Path(__file__).resolve().parent
CONFIG_PATH = BASE_DIR / "assistant_config.json"
LOG_PATH = BASE_DIR / "assistant_logs.jsonl"


class AssistantPanel:
    def __init__(self) -> None:
        self.config = load_assistant_config(str(CONFIG_PATH))
        self.router = CommandRouter(self.config, str(LOG_PATH))
        self.engine: VoiceEngine | None = None
        self.hotkey_registered = False

        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")

        self.root = ctk.CTk()
        self.root.title("Jarvis - Controle por Voz")
        self.root.geometry("880x680")
        self.root.minsize(760, 560)

        self.status_var = ctk.StringVar(value="Inicializando microfone")
        self.last_text_var = ctk.StringVar(value="Nenhum comando ouvido ainda")
        self.intent_var = ctk.StringVar(value="-")
        self.action_var = ctk.StringVar(value="-")
        self.result_var = ctk.StringVar(value="-")
        self.test_var = ctk.StringVar()
        self.wake_var = ctk.StringVar(value=", ".join(self.config.get("wake_words", [])))
        self.hotkey_var = ctk.StringVar(value=self.config.get("hotkey", "Ctrl+Shift+J"))
        self.logs_box: ctk.CTkTextbox | None = None

        self._build_ui()
        self._start_voice()
        self._register_hotkey()
        self._start_tray()
        self._refresh_logs()

    def run(self) -> None:
        self.root.mainloop()

    def _build_ui(self) -> None:
        header = ctk.CTkFrame(self.root, fg_color="transparent")
        header.pack(fill="x", padx=22, pady=(20, 10))
        ctk.CTkLabel(header, text="Jarvis - controle real do PC por voz", font=("Segoe UI", 23, "bold")).pack(anchor="w")
        ctk.CTkLabel(
            header,
            text="Vosk offline em portugues-BR, hotkey, wake word, executor local e Spotify.",
            text_color="#9ca3af",
        ).pack(anchor="w", pady=(2, 0))

        tabs = ctk.CTkTabview(self.root, corner_radius=8)
        tabs.pack(fill="both", expand=True, padx=20, pady=(0, 14))
        tab_control = tabs.add("Controle")
        tab_routines = tabs.add("Rotinas")
        tab_spotify = tabs.add("Spotify")
        tab_logs = tabs.add("Logs")
        tab_settings = tabs.add("Configuracoes")

        self._build_control(tab_control)
        self._build_routines(tab_routines)
        self._build_spotify(tab_spotify)
        self._build_logs(tab_logs)
        self._build_settings(tab_settings)

    def _build_control(self, parent: ctk.CTkFrame) -> None:
        status = ctk.CTkFrame(parent, corner_radius=8)
        status.pack(fill="x", padx=8, pady=(10, 8))
        ctk.CTkLabel(status, textvariable=self.status_var, font=("Segoe UI", 19, "bold")).pack(anchor="w", padx=16, pady=(14, 4))
        ctk.CTkLabel(status, text=f"Hotkey: {self.config.get('hotkey')}  |  Wake words: {', '.join(self.config.get('wake_words', []))}", text_color="#9ca3af").pack(anchor="w", padx=16, pady=(0, 14))

        grid = ctk.CTkFrame(parent, fg_color="transparent")
        grid.pack(fill="x", padx=8, pady=6)
        for col in range(2):
            grid.grid_columnconfigure(col, weight=1)

        self._info_card(grid, "Ultimo comando ouvido", self.last_text_var, 0, 0)
        self._info_card(grid, "Intencao detectada", self.intent_var, 0, 1)
        self._info_card(grid, "Acao prevista", self.action_var, 1, 0)
        self._info_card(grid, "Resultado", self.result_var, 1, 1)

        buttons = ctk.CTkFrame(parent, fg_color="transparent")
        buttons.pack(fill="x", padx=8, pady=(12, 6))
        ctk.CTkButton(buttons, text="Falar agora", command=self._arm_voice, height=38).pack(side="left", padx=(0, 8))
        ctk.CTkButton(buttons, text="Pausar microfone", command=self._toggle_pause, height=38, fg_color="#444", hover_color="#555").pack(side="left", padx=(0, 8))
        ctk.CTkButton(buttons, text="Confirmar acao", command=self._confirm_pending, height=38, fg_color="#2f855a").pack(side="left")

        test = ctk.CTkFrame(parent, corner_radius=8)
        test.pack(fill="x", padx=8, pady=(12, 0))
        ctk.CTkLabel(test, text="Testar comando por texto", font=("Segoe UI", 15, "bold")).pack(anchor="w", padx=16, pady=(12, 4))
        row = ctk.CTkFrame(test, fg_color="transparent")
        row.pack(fill="x", padx=16, pady=(0, 14))
        ctk.CTkEntry(row, textvariable=self.test_var, placeholder_text="abre o Chrome e toca playlist foco").pack(side="left", fill="x", expand=True, padx=(0, 8))
        ctk.CTkButton(row, text="Executar", command=lambda: self._process_text(self.test_var.get(), "text")).pack(side="left")

    def _info_card(self, parent: ctk.CTkFrame, title: str, variable: ctk.StringVar, row: int, col: int) -> None:
        card = ctk.CTkFrame(parent, corner_radius=8)
        card.grid(row=row, column=col, sticky="nsew", padx=6, pady=6)
        ctk.CTkLabel(card, text=title, text_color="#9ca3af").pack(anchor="w", padx=14, pady=(12, 2))
        ctk.CTkLabel(card, textvariable=variable, wraplength=360, font=("Segoe UI", 14, "bold")).pack(anchor="w", padx=14, pady=(0, 12))

    def _build_routines(self, parent: ctk.CTkFrame) -> None:
        ctk.CTkLabel(parent, text="Rotinas configuradas", font=("Segoe UI", 17, "bold")).pack(anchor="w", padx=16, pady=(16, 8))
        routines = self.config.get("rotinas", {})
        if not routines:
            ctk.CTkLabel(parent, text="Nenhuma rotina configurada.").pack(anchor="w", padx=16)
            return
        for name, actions in routines.items():
            row = ctk.CTkFrame(parent, corner_radius=8)
            row.pack(fill="x", padx=16, pady=5)
            ctk.CTkLabel(row, text=name, font=("Segoe UI", 14, "bold")).pack(side="left", padx=12, pady=12)
            ctk.CTkLabel(row, text=f"{len(actions)} acoes", text_color="#9ca3af").pack(side="left", padx=8)
            ctk.CTkButton(row, text="Executar", width=100, command=lambda n=name: self._process_text(f"ativar {n}", "routine")).pack(side="right", padx=12)

    def _build_spotify(self, parent: ctk.CTkFrame) -> None:
        ctk.CTkLabel(parent, text="Comandos Spotify", font=("Segoe UI", 17, "bold")).pack(anchor="w", padx=16, pady=(16, 8))
        commands = ["toca lo-fi", "toca playlist foco", "pausa Spotify", "proxima musica", "musica anterior", "aumenta volume"]
        for cmd in commands:
            ctk.CTkButton(parent, text=cmd, command=lambda c=cmd: self._process_text(c, "quick"), height=36, fg_color="#444", hover_color="#555").pack(anchor="w", padx=16, pady=4)

    def _build_logs(self, parent: ctk.CTkFrame) -> None:
        ctk.CTkButton(parent, text="Atualizar logs", command=self._refresh_logs).pack(anchor="w", padx=16, pady=(16, 8))
        self.logs_box = ctk.CTkTextbox(parent, height=440)
        self.logs_box.pack(fill="both", expand=True, padx=16, pady=(0, 16))

    def _build_settings(self, parent: ctk.CTkFrame) -> None:
        form = ctk.CTkFrame(parent, corner_radius=8)
        form.pack(fill="x", padx=16, pady=16)
        ctk.CTkLabel(form, text="Wake words", text_color="#9ca3af").pack(anchor="w", padx=16, pady=(14, 2))
        ctk.CTkEntry(form, textvariable=self.wake_var).pack(fill="x", padx=16, pady=(0, 10))
        ctk.CTkLabel(form, text="Hotkey", text_color="#9ca3af").pack(anchor="w", padx=16, pady=(4, 2))
        ctk.CTkEntry(form, textvariable=self.hotkey_var).pack(fill="x", padx=16, pady=(0, 12))
        ctk.CTkButton(form, text="Salvar configuracoes", command=self._save_settings).pack(anchor="w", padx=16, pady=(0, 14))

    def _start_voice(self) -> None:
        self.engine = VoiceEngine(
            wake_words=self.config.get("wake_words", ["jarvis", "assistente"]),
            language=self.config.get("idioma", "pt-BR"),
            listen_seconds=int(self.config.get("listen_seconds", 7)),
            on_text=lambda text: self.root.after(0, lambda: self._process_text(text, "voice")),
            on_state=lambda state: self.root.after(0, lambda: self._update_voice_state(state)),
        )
        self.engine.start()

    def _register_hotkey(self) -> None:
        try:
            import keyboard

            keyboard.add_hotkey(self.config.get("hotkey", "ctrl+shift+j").lower(), self._arm_voice)
            self.hotkey_registered = True
        except Exception as exc:
            self.status_var.set(f"Hotkey indisponivel: {exc}")

    def _start_tray(self) -> None:
        try:
            import pystray
            from PIL import Image, ImageDraw

            image = Image.new("RGB", (64, 64), "#111827")
            draw = ImageDraw.Draw(image)
            draw.ellipse((14, 14, 50, 50), fill="#38bdf8")

            def show_window(icon, item):  # noqa: ANN001
                self.root.after(0, self.root.deiconify)

            def quit_app(icon, item):  # noqa: ANN001
                icon.stop()
                self.root.after(0, self.root.destroy)

            menu = pystray.Menu(
                pystray.MenuItem("Abrir painel", show_window),
                pystray.MenuItem("Falar agora", lambda icon, item: self._arm_voice()),
                pystray.MenuItem("Sair", quit_app),
            )
            icon = pystray.Icon("Jarvis", image, "Jarvis - Controle por Voz", menu)
            threading.Thread(target=icon.run, daemon=True).start()
        except Exception:
            pass

    def _arm_voice(self) -> None:
        if self.engine:
            self.engine.resume()
            self.engine.arm()
            self.status_var.set("Ouvindo comando...")

    def _toggle_pause(self) -> None:
        if not self.engine:
            return
        if self.engine.state.paused:
            self.engine.resume()
            self.status_var.set("Microfone ativo")
        else:
            self.engine.pause()
            self.status_var.set("Microfone pausado")

    def _confirm_pending(self) -> None:
        result = self.router.handle_text("confirmar", confirmed=True)
        self._show_result(result)

    def _process_text(self, text: str, source: str) -> None:
        text = (text or "").strip()
        if not text:
            return
        self.last_text_var.set(f"{text} ({source} {timestamp()})")
        result = self.router.handle_text(text)
        self._show_result(result)
        self._refresh_logs()

    def _show_result(self, result: ExecutionResult) -> None:
        if result.intent:
            self.intent_var.set(f"{result.intent.type} -> {result.intent.target or result.intent.parameters}")
        if result.action:
            self.action_var.set(result.action.label)
        self.result_var.set(result.message)
        if result.needs_confirmation:
            self.status_var.set("Aguardando confirmacao")
        elif result.success:
            self.status_var.set("Executado")
        else:
            self.status_var.set("Pronto")

    def _update_voice_state(self, state: VoiceEngineState) -> None:
        if state.last_error:
            self.status_var.set(state.last_error)
        elif state.paused:
            self.status_var.set("Microfone pausado")
        elif state.listening:
            self.status_var.set("Ouvindo comando...")
        elif state.model_ready:
            self.status_var.set("Aguardando wake word ou hotkey")
        if state.last_text:
            self.last_text_var.set(state.last_text)

    def _refresh_logs(self) -> None:
        if not self.logs_box:
            return
        self.logs_box.configure(state="normal")
        self.logs_box.delete("1.0", "end")
        if LOG_PATH.exists():
            lines = LOG_PATH.read_text(encoding="utf-8").splitlines()[-120:]
            self.logs_box.insert("end", "\n".join(lines))
        self.logs_box.configure(state="disabled")

    def _save_settings(self) -> None:
        self.config["wake_words"] = [item.strip() for item in self.wake_var.get().split(",") if item.strip()]
        self.config["hotkey"] = self.hotkey_var.get().strip() or "Ctrl+Shift+J"
        CONFIG_PATH.write_text(json.dumps(self.config, indent=2, ensure_ascii=False), encoding="utf-8")
        messagebox.showinfo("Configuracoes", "Configuracoes salvas. Reinicie o painel para reaplicar hotkey e wake words.")


def main() -> None:
    AssistantPanel().run()


if __name__ == "__main__":
    main()
