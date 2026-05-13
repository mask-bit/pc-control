"""Painel principal do PC Control para Windows."""

from __future__ import annotations

import atexit
import json
import os
import threading
from dataclasses import asdict
from tkinter import messagebox

import customtkinter as ctk

from app_paths import (
    APP_NAME,
    CONFIG_PATH,
    LOG_PATH,
    app_root,
    clear_ephemeral_session,
    ensure_app_dirs,
    ensure_user_config,
    executable_path,
    is_frozen,
    resolve_asset,
    source_pythonw,
)
from auth_google import GoogleAuthManager
from command_router import CommandRouter, ExecutionResult, load_assistant_config
from intent_parser import parse_local
from openai_controller import get_openai_key, test_openai_connection
from secrets_store import set_secret
from spotify_controller import authorize_pkce, get_status, is_connected
from voice_engine import VoiceEngine, VoiceEngineState, list_input_devices, timestamp

BG = "#090d16"
SURFACE = "#111827"
SURFACE_2 = "#172033"
LINE = "#243044"
TEXT = "#f8fafc"
MUTED = "#94a3b8"
ACCENT = "#38bdf8"
ACCENT_DARK = "#0f7490"
SUCCESS = "#22c55e"
WARNING = "#f59e0b"
ERROR = "#ef4444"


class AssistantPanel:
    def __init__(self) -> None:
        ensure_app_dirs()
        ensure_user_config()
        self.config = load_assistant_config(str(CONFIG_PATH))
        self.router = CommandRouter(self.config, str(LOG_PATH))
        self.google_auth = GoogleAuthManager()
        self.engine: VoiceEngine | None = None
        self.tray_icon = None
        self.hotkey_registered = False
        self.current_page = "home"
        self.history: list[dict[str, str]] = []

        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")

        self.root = ctk.CTk(fg_color=BG)
        self.root.title(f"{APP_NAME} - Controle inteligente do PC")
        self.root.geometry("1180x760")
        self.root.minsize(1020, 660)
        self.root.protocol("WM_DELETE_WINDOW", self._hide_window)
        self._apply_window_icon()

        self.status_var = ctk.StringVar(value="Inicializando")
        self.subtitle_var = ctk.StringVar(value="Preparando microfone, hotkey e executor local.")
        self.last_text_var = ctk.StringVar(value="Nenhum comando recebido")
        self.intent_var = ctk.StringVar(value="-")
        self.action_var = ctk.StringVar(value="-")
        self.result_var = ctk.StringVar(value="-")
        self.command_var = ctk.StringVar()
        self.wake_var = ctk.StringVar(value=", ".join(self.config.get("wake_words", [])))
        self.hotkey_var = ctk.StringVar(value=self.config.get("hotkey", "Ctrl+Shift+J"))
        self.openai_key_var = ctk.StringVar()
        self.spotify_client_var = ctk.StringVar(value=self.config.get("spotify_web", {}).get("client_id", ""))
        self.google_client_var = ctk.StringVar(value=self.config.get("google_auth", {}).get("client_id", ""))
        self.google_status_var = ctk.StringVar(value=self.google_auth.status_label())
        self.spotify_query_var = ctk.StringVar(value="lo-fi focus")
        self.routine_name_var = ctk.StringVar(value="modo foco")
        self.mic_var = ctk.StringVar()
        self.startup_var = ctk.BooleanVar(value=self.config.get("iniciar_com_windows", False))

        self.content: ctk.CTkFrame
        self.audio_bar: ctk.CTkProgressBar | None = None
        self.transcript_box: ctk.CTkTextbox | None = None
        self.logs_box: ctk.CTkTextbox | None = None
        self.routine_steps_box: ctk.CTkTextbox | None = None
        self.chat_box: ctk.CTkTextbox | None = None

        self._build_shell()
        atexit.register(clear_ephemeral_session)
        self._show_page("home")
        self.root.after(450, self._show_onboarding_if_needed)
        self._start_voice()
        self._register_hotkey()
        self._start_tray()

    def run(self) -> None:
        self.root.mainloop()

    def _apply_window_icon(self) -> None:
        icon_path = resolve_asset("pc-control.ico")
        if icon_path.exists():
            try:
                self.root.iconbitmap(str(icon_path))
            except Exception:
                pass

    def _show_onboarding_if_needed(self) -> None:
        if self.config.get("onboarding_complete"):
            return
        dialog = ctk.CTkToplevel(self.root)
        dialog.title(f"Bem-vindo ao {APP_NAME}")
        dialog.geometry("560x420")
        dialog.resizable(False, False)
        dialog.configure(fg_color=BG)
        dialog.transient(self.root)
        dialog.grab_set()
        ctk.CTkLabel(dialog, text=APP_NAME, font=("Segoe UI", 30, "bold"), text_color=TEXT).pack(
            anchor="w", padx=28, pady=(28, 4)
        )
        ctk.CTkLabel(
            dialog,
            text=(
                "Controle o PC por texto ou voz, abra apps, pesquise no YouTube, use Spotify e crie rotinas. "
                "O login Google e opcional e a sessao padrao nao fica salva permanentemente."
            ),
            text_color=MUTED,
            wraplength=500,
            justify="left",
        ).pack(anchor="w", padx=28, pady=(0, 22))
        ctk.CTkButton(
            dialog,
            text="Entrar com Google",
            height=44,
            fg_color="#2563eb",
            command=lambda: self._onboarding_google(dialog),
        ).pack(fill="x", padx=28, pady=(0, 10))
        ctk.CTkButton(
            dialog,
            text="Continuar em modo local",
            height=44,
            fg_color=SURFACE_2,
            hover_color="#223049",
            command=lambda: self._finish_onboarding(dialog),
        ).pack(fill="x", padx=28, pady=(0, 14))
        ctk.CTkLabel(
            dialog,
            text="Voce pode conectar Google, OpenAI e Spotify depois na tela Integracoes.",
            text_color=MUTED,
            wraplength=500,
            justify="left",
        ).pack(anchor="w", padx=28)

    def _onboarding_google(self, dialog: ctk.CTkToplevel) -> None:
        self._save_google_client()
        self._login_google()
        self._finish_onboarding(dialog)

    def _finish_onboarding(self, dialog: ctk.CTkToplevel) -> None:
        self.config["onboarding_complete"] = True
        self._save_config()
        dialog.destroy()

    def _build_shell(self) -> None:
        self.root.grid_columnconfigure(1, weight=1)
        self.root.grid_rowconfigure(0, weight=1)

        sidebar = ctk.CTkFrame(self.root, width=246, corner_radius=0, fg_color="#070b12")
        sidebar.grid(row=0, column=0, sticky="nsew")
        sidebar.grid_propagate(False)

        brand = ctk.CTkFrame(sidebar, fg_color="transparent")
        brand.pack(fill="x", padx=22, pady=(24, 22))
        ctk.CTkLabel(brand, text=APP_NAME, font=("Segoe UI", 28, "bold"), text_color=TEXT).pack(anchor="w")
        ctk.CTkLabel(
            brand,
            text="Assistente de controle para Windows",
            font=("Segoe UI", 12),
            text_color=MUTED,
            wraplength=190,
            justify="left",
        ).pack(anchor="w", pady=(2, 0))

        nav_items = [
            ("Inicio", "home"),
            ("Assistente", "assistant"),
            ("Spotify", "spotify"),
            ("Rotinas", "routines"),
            ("Voz e audio", "voice"),
            ("Integracoes", "integrations"),
            ("Configuracoes", "settings"),
            ("Logs", "logs"),
        ]
        for label, page in nav_items:
            ctk.CTkButton(
                sidebar,
                text=label,
                anchor="w",
                height=42,
                corner_radius=8,
                fg_color="transparent",
                hover_color=SURFACE_2,
                text_color=TEXT,
                command=lambda p=page: self._show_page(p),
            ).pack(fill="x", padx=14, pady=3)

        bottom = ctk.CTkFrame(sidebar, fg_color=SURFACE, corner_radius=10)
        bottom.pack(side="bottom", fill="x", padx=14, pady=16)
        ctk.CTkLabel(bottom, textvariable=self.status_var, font=("Segoe UI", 14, "bold"), text_color=TEXT).pack(
            anchor="w", padx=14, pady=(12, 2)
        )
        ctk.CTkLabel(bottom, textvariable=self.subtitle_var, text_color=MUTED, wraplength=188, justify="left").pack(
            anchor="w", padx=14, pady=(0, 12)
        )

        main = ctk.CTkFrame(self.root, fg_color=BG, corner_radius=0)
        main.grid(row=0, column=1, sticky="nsew")
        main.grid_rowconfigure(1, weight=1)
        main.grid_columnconfigure(0, weight=1)

        topbar = ctk.CTkFrame(main, fg_color=BG, height=74)
        topbar.grid(row=0, column=0, sticky="ew", padx=26, pady=(18, 0))
        topbar.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(
            topbar,
            text="Controle inteligente do PC",
            font=("Segoe UI", 24, "bold"),
            text_color=TEXT,
        ).grid(row=0, column=0, sticky="w")
        ctk.CTkLabel(
            topbar,
            text=f"Fale ou escreva um comando. O {APP_NAME} interpreta, valida e executa.",
            text_color=MUTED,
        ).grid(row=1, column=0, sticky="w", pady=(2, 0))
        ctk.CTkButton(topbar, text="Falar agora", width=128, height=38, command=self._arm_voice).grid(
            row=0, column=1, rowspan=2, padx=(14, 0)
        )

        self.content = ctk.CTkFrame(main, fg_color=BG, corner_radius=0)
        self.content.grid(row=1, column=0, sticky="nsew", padx=26, pady=(6, 22))
        self.content.grid_columnconfigure(0, weight=1)
        self.content.grid_rowconfigure(0, weight=1)

    def _show_page(self, page: str) -> None:
        self.current_page = page
        for child in self.content.winfo_children():
            child.destroy()
        self.audio_bar = None
        self.transcript_box = None
        self.logs_box = None
        self.routine_steps_box = None
        self.chat_box = None
        builders = {
            "home": self._build_home,
            "assistant": self._build_assistant,
            "spotify": self._build_spotify,
            "routines": self._build_routines,
            "voice": self._build_voice,
            "integrations": self._build_integrations,
            "settings": self._build_settings,
            "logs": self._build_logs,
        }
        builders.get(page, self._build_home)()

    def _build_home(self) -> None:
        page = self._page_frame()
        hero = ctk.CTkFrame(page, fg_color=SURFACE, corner_radius=14, border_color=LINE, border_width=1)
        hero.pack(fill="x", pady=(0, 14))
        hero.grid_columnconfigure(0, weight=1)
        ctk.CTkLabel(hero, text="O que voce quer controlar agora?", font=("Segoe UI", 26, "bold"), text_color=TEXT).grid(
            row=0, column=0, sticky="w", padx=22, pady=(20, 4)
        )
        ctk.CTkLabel(
            hero,
            text="Exemplos: abrir YouTube, pesquisar lo-fi no YouTube, tocar playlist foco, abrir Chrome e Spotify.",
            text_color=MUTED,
            wraplength=780,
            justify="left",
        ).grid(row=1, column=0, sticky="w", padx=22)
        command_row = ctk.CTkFrame(hero, fg_color="transparent")
        command_row.grid(row=2, column=0, sticky="ew", padx=22, pady=(18, 22))
        command_row.grid_columnconfigure(0, weight=1)
        entry = ctk.CTkEntry(
            command_row,
            textvariable=self.command_var,
            height=46,
            placeholder_text="Digite um comando para executar no PC",
            border_color=LINE,
            fg_color="#0b1220",
        )
        entry.grid(row=0, column=0, sticky="ew", padx=(0, 10))
        entry.bind("<Return>", lambda _event: self._submit_command())
        ctk.CTkButton(command_row, text="Executar", width=120, height=46, command=self._submit_command).grid(row=0, column=1)
        ctk.CTkButton(
            command_row,
            text="Microfone",
            width=120,
            height=46,
            fg_color=ACCENT_DARK,
            hover_color="#155e75",
            command=self._arm_voice,
        ).grid(row=0, column=2, padx=(10, 0))

        grid = ctk.CTkFrame(page, fg_color="transparent")
        grid.pack(fill="x", pady=(0, 14))
        for col in range(4):
            grid.grid_columnconfigure(col, weight=1)
        self._metric_card(grid, "Ultima fala", self.last_text_var, 0)
        self._metric_card(grid, "Intencao", self.intent_var, 1)
        self._metric_card(grid, "Acao prevista", self.action_var, 2)
        self._metric_card(grid, "Resultado", self.result_var, 3)

        lower = ctk.CTkFrame(page, fg_color="transparent")
        lower.pack(fill="both", expand=True)
        lower.grid_columnconfigure(0, weight=1)
        lower.grid_columnconfigure(1, weight=1)
        lower.grid_rowconfigure(0, weight=1)

        quick = self._card(lower, "Acoes rapidas", "Comandos prontos para testar o fluxo real.")
        quick.grid(row=0, column=0, sticky="nsew", padx=(0, 7))
        for cmd in [
            "abrir YouTube",
            "pesquisar lo-fi no YouTube",
            "abrir calculadora",
            "tocar playlist foco",
            "pausar musica",
            "ativar modo estudo",
        ]:
            ctk.CTkButton(
                quick,
                text=cmd,
                anchor="w",
                height=34,
                fg_color=SURFACE_2,
                hover_color="#223049",
                command=lambda c=cmd: self._process_text(c, "atalho"),
            ).pack(fill="x", padx=16, pady=4)

        recent = self._card(lower, "Historico recente", "Ultimos comandos executados nesta sessao.")
        recent.grid(row=0, column=1, sticky="nsew", padx=(7, 0))
        self.chat_box = ctk.CTkTextbox(recent, fg_color="#0b1220", border_color=LINE, border_width=1)
        self.chat_box.pack(fill="both", expand=True, padx=16, pady=(8, 16))
        self._render_history()

    def _build_assistant(self) -> None:
        page = self._page_frame()
        page.grid_columnconfigure(0, weight=1)
        page.grid_rowconfigure(0, weight=1)
        card = self._card(page, "Assistente operacional", "Historico de fala, interpretacao, acoes executadas e retorno.")
        card.grid(row=0, column=0, sticky="nsew")
        self.chat_box = ctk.CTkTextbox(card, fg_color="#0b1220", border_color=LINE, border_width=1)
        self.chat_box.pack(fill="both", expand=True, padx=16, pady=(10, 12))
        self._render_history()
        row = ctk.CTkFrame(card, fg_color="transparent")
        row.pack(fill="x", padx=16, pady=(0, 16))
        row.grid_columnconfigure(0, weight=1)
        entry = ctk.CTkEntry(row, textvariable=self.command_var, height=42, placeholder_text="Pergunte ou mande um comando")
        entry.grid(row=0, column=0, sticky="ew", padx=(0, 10))
        entry.bind("<Return>", lambda _event: self._submit_command())
        ctk.CTkButton(row, text="Enviar", width=100, height=42, command=self._submit_command).grid(row=0, column=1)
        ctk.CTkButton(row, text="Confirmar", width=112, height=42, fg_color=SUCCESS, command=self._confirm_pending).grid(
            row=0, column=2, padx=(10, 0)
        )

    def _build_spotify(self) -> None:
        page = self._page_frame()
        page.grid_columnconfigure(0, weight=1)
        page.grid_columnconfigure(1, weight=1)

        status = get_status(self.config)
        now = self._card(page, "Spotify", "Controle por Web API quando conectado; fallback por URI e teclas de midia.")
        now.grid(row=0, column=0, sticky="nsew", padx=(0, 7), pady=(0, 14))
        badge_color = SUCCESS if status.connected else WARNING
        ctk.CTkLabel(now, text=status.message, text_color=badge_color, font=("Segoe UI", 14, "bold")).pack(
            anchor="w", padx=16, pady=(8, 4)
        )
        track = status.current_track or "Nenhuma faixa consultada"
        artist = status.artist or "Conecte o Spotify ou use comandos por URI"
        ctk.CTkLabel(now, text=track, text_color=TEXT, font=("Segoe UI", 22, "bold"), wraplength=430).pack(
            anchor="w", padx=16, pady=(6, 2)
        )
        ctk.CTkLabel(now, text=artist, text_color=MUTED, wraplength=430).pack(anchor="w", padx=16, pady=(0, 14))
        controls = ctk.CTkFrame(now, fg_color="transparent")
        controls.pack(fill="x", padx=16, pady=(2, 16))
        for label, cmd in [
            ("Anterior", "musica anterior"),
            ("Pausar", "pausar musica"),
            ("Proxima", "proxima musica"),
            ("Volume 30", "volume para 30"),
        ]:
            ctk.CTkButton(controls, text=label, width=98, height=36, command=lambda c=cmd: self._process_text(c, "spotify")).pack(
                side="left", padx=(0, 8)
            )

        search = self._card(page, "Tocar agora", "Busque uma musica, artista, album ou playlist pelo comando.")
        search.grid(row=0, column=1, sticky="nsew", padx=(7, 0), pady=(0, 14))
        ctk.CTkEntry(search, textvariable=self.spotify_query_var, height=42, placeholder_text="jazz, lo-fi, playlist foco").pack(
            fill="x", padx=16, pady=(10, 10)
        )
        ctk.CTkButton(search, text="Tocar no Spotify", height=40, command=self._play_spotify_query).pack(
            fill="x", padx=16, pady=(0, 12)
        )
        ctk.CTkButton(
            search,
            text="Atualizar musica atual",
            height=38,
            fg_color=SURFACE_2,
            hover_color="#223049",
            command=lambda: self._show_page("spotify"),
        ).pack(fill="x", padx=16, pady=(0, 16))

        shortcuts = self._card(page, "Atalhos musicais", "Itens configurados no PC Control.")
        shortcuts.grid(row=1, column=0, columnspan=2, sticky="nsew")
        for name in sorted((self.config.get("spotify") or {}).keys()):
            if name == "default_query":
                continue
            ctk.CTkButton(
                shortcuts,
                text=name,
                anchor="w",
                height=34,
                fg_color=SURFACE_2,
                hover_color="#223049",
                command=lambda n=name: self._process_text(f"tocar {n}", "spotify"),
            ).pack(fill="x", padx=16, pady=4)

    def _build_routines(self) -> None:
        page = self._page_frame()
        page.grid_columnconfigure(0, weight=1)
        page.grid_columnconfigure(1, weight=1)
        page.grid_rowconfigure(0, weight=1)

        routines_card = self._card(page, "Rotinas", "Sequencias reais que podem ser executadas por voz ou texto.")
        routines_card.grid(row=0, column=0, sticky="nsew", padx=(0, 7))
        routines = self.config.get("rotinas", {})
        if not routines:
            ctk.CTkLabel(routines_card, text="Nenhuma rotina criada ainda.", text_color=MUTED).pack(anchor="w", padx=16, pady=14)
        for name, actions in routines.items():
            row = ctk.CTkFrame(routines_card, fg_color="#0b1220", corner_radius=10, border_color=LINE, border_width=1)
            row.pack(fill="x", padx=16, pady=6)
            ctk.CTkLabel(row, text=name, text_color=TEXT, font=("Segoe UI", 15, "bold")).pack(
                side="left", padx=12, pady=12
            )
            ctk.CTkLabel(row, text=f"{len(actions)} acoes", text_color=MUTED).pack(side="left", padx=6)
            ctk.CTkButton(row, text="Executar", width=96, command=lambda n=name: self._process_text(f"ativar {n}", "rotina")).pack(
                side="right", padx=10
            )
            ctk.CTkButton(
                row,
                text="Excluir",
                width=76,
                fg_color="#7f1d1d",
                hover_color="#991b1b",
                command=lambda n=name: self._delete_routine(n),
            ).pack(side="right")

        editor = self._card(page, "Criar rotina", "Digite um comando por linha. O app salva as acoes estruturadas.")
        editor.grid(row=0, column=1, sticky="nsew", padx=(7, 0))
        ctk.CTkLabel(editor, text="Nome da rotina", text_color=MUTED).pack(anchor="w", padx=16, pady=(10, 2))
        ctk.CTkEntry(editor, textvariable=self.routine_name_var, height=38).pack(fill="x", padx=16, pady=(0, 10))
        ctk.CTkLabel(editor, text="Passos", text_color=MUTED).pack(anchor="w", padx=16, pady=(2, 2))
        self.routine_steps_box = ctk.CTkTextbox(editor, height=210, fg_color="#0b1220", border_color=LINE, border_width=1)
        self.routine_steps_box.pack(fill="both", expand=True, padx=16, pady=(0, 12))
        self.routine_steps_box.insert("1.0", "abrir YouTube\npesquisar lo-fi no YouTube\ntocar playlist foco\nvolume para 30")
        ctk.CTkButton(editor, text="Salvar rotina", height=40, command=self._save_routine).pack(fill="x", padx=16, pady=(0, 16))

    def _build_voice(self) -> None:
        page = self._page_frame()
        page.grid_columnconfigure(0, weight=1)
        page.grid_columnconfigure(1, weight=1)

        control = self._card(page, "Voz e audio", "Fluxo real: microfone -> Vosk -> intencao -> execucao.")
        control.grid(row=0, column=0, sticky="nsew", padx=(0, 7))
        devices = list_input_devices()
        device_options = [f"{item['index']} - {item['name']}" for item in devices] or ["-1 - Nenhum microfone encontrado"]
        current_device = str(self.config.get("microphone_device", ""))
        selected = next((item for item in device_options if item.startswith(current_device + " -")), device_options[0])
        self.mic_var.set(selected)
        ctk.CTkLabel(control, text="Microfone", text_color=MUTED).pack(anchor="w", padx=16, pady=(10, 2))
        ctk.CTkOptionMenu(control, values=device_options, variable=self.mic_var).pack(fill="x", padx=16, pady=(0, 12))
        ctk.CTkLabel(control, text="Nivel de entrada", text_color=MUTED).pack(anchor="w", padx=16, pady=(4, 4))
        self.audio_bar = ctk.CTkProgressBar(control, height=14, progress_color=ACCENT)
        self.audio_bar.pack(fill="x", padx=16, pady=(0, 16))
        self.audio_bar.set(0)
        buttons = ctk.CTkFrame(control, fg_color="transparent")
        buttons.pack(fill="x", padx=16, pady=(0, 16))
        ctk.CTkButton(buttons, text="Falar agora", command=self._arm_voice).pack(side="left", padx=(0, 8))
        ctk.CTkButton(buttons, text="Pausar microfone", fg_color=SURFACE_2, command=self._toggle_pause).pack(side="left")
        ctk.CTkButton(control, text="Salvar microfone", height=38, command=self._save_microphone).pack(fill="x", padx=16, pady=(0, 16))

        transcript = self._card(page, "Transcricao", "Ultimas frases reconhecidas pelo Vosk.")
        transcript.grid(row=0, column=1, sticky="nsew", padx=(7, 0))
        self.transcript_box = ctk.CTkTextbox(transcript, fg_color="#0b1220", border_color=LINE, border_width=1)
        self.transcript_box.pack(fill="both", expand=True, padx=16, pady=(10, 16))
        self.transcript_box.insert("end", self.last_text_var.get())

    def _build_integrations(self) -> None:
        page = self._page_frame()
        page.grid_columnconfigure(0, weight=1)
        page.grid_columnconfigure(1, weight=1)
        page.grid_rowconfigure(1, weight=1)

        openai = self._card(page, "OpenAI", "Opcional: melhora interpretacao de frases vagas e responde pedidos de texto.")
        openai.grid(row=0, column=0, sticky="nsew", padx=(0, 7))
        openai_status = "Conectada" if get_openai_key(self.config) else "Nao configurada"
        ctk.CTkLabel(openai, text=openai_status, text_color=SUCCESS if get_openai_key(self.config) else WARNING, font=("Segoe UI", 15, "bold")).pack(
            anchor="w", padx=16, pady=(10, 8)
        )
        ctk.CTkEntry(openai, textvariable=self.openai_key_var, placeholder_text="Cole sua OPENAI_API_KEY", show="*", height=40).pack(
            fill="x", padx=16, pady=(0, 10)
        )
        ctk.CTkButton(openai, text="Salvar chave com seguranca", height=38, command=self._save_openai_key).pack(
            fill="x", padx=16, pady=(0, 8)
        )
        ctk.CTkButton(
            openai,
            text="Testar OpenAI",
            height=38,
            fg_color=SURFACE_2,
            hover_color="#223049",
            command=self._test_openai,
        ).pack(fill="x", padx=16, pady=(0, 16))

        spotify = self._card(page, "Spotify", "Login via OAuth PKCE. Sem client secret no app desktop.")
        spotify.grid(row=0, column=1, sticky="nsew", padx=(7, 0))
        connected = is_connected(self.config)
        ctk.CTkLabel(spotify, text="Conectado" if connected else "Nao conectado", text_color=SUCCESS if connected else WARNING, font=("Segoe UI", 15, "bold")).pack(
            anchor="w", padx=16, pady=(10, 8)
        )
        ctk.CTkEntry(spotify, textvariable=self.spotify_client_var, placeholder_text="Spotify Client ID", height=40).pack(
            fill="x", padx=16, pady=(0, 10)
        )
        ctk.CTkButton(spotify, text="Salvar Client ID", height=38, command=self._save_spotify_client).pack(
            fill="x", padx=16, pady=(0, 8)
        )
        ctk.CTkButton(
            spotify,
            text="Conectar Spotify",
            height=38,
            fg_color="#1db954",
            hover_color="#169c46",
            command=self._connect_spotify,
        ).pack(fill="x", padx=16, pady=(0, 16))

        google = self._card(page, "Google", "Login opcional via navegador. A sessao padrao e temporaria e o app funciona sem conta.")
        google.grid(row=1, column=0, columnspan=2, sticky="nsew", pady=(14, 0))
        ctk.CTkLabel(
            google,
            textvariable=self.google_status_var,
            text_color=SUCCESS if self.google_auth.session.active else MUTED,
            font=("Segoe UI", 15, "bold"),
        ).pack(anchor="w", padx=16, pady=(10, 8))
        ctk.CTkEntry(
            google,
            textvariable=self.google_client_var,
            placeholder_text="Google OAuth Client ID",
            height=40,
        ).pack(fill="x", padx=16, pady=(0, 10))
        row = ctk.CTkFrame(google, fg_color="transparent")
        row.pack(fill="x", padx=16, pady=(0, 16))
        ctk.CTkButton(row, text="Salvar Client ID", height=38, command=self._save_google_client).pack(side="left", padx=(0, 8))
        ctk.CTkButton(row, text="Entrar com Google", height=38, fg_color="#2563eb", command=self._login_google).pack(
            side="left", padx=(0, 8)
        )
        ctk.CTkButton(row, text="Sair", height=38, fg_color=SURFACE_2, hover_color="#223049", command=self._logout_google).pack(
            side="left"
        )

    def _build_settings(self) -> None:
        page = self._page_frame()
        form = self._card(page, "Configuracoes", "Ajustes principais do assistente local.")
        form.pack(fill="x")
        ctk.CTkLabel(form, text="Wake words", text_color=MUTED).pack(anchor="w", padx=16, pady=(10, 2))
        ctk.CTkEntry(form, textvariable=self.wake_var, height=38).pack(fill="x", padx=16, pady=(0, 10))
        ctk.CTkLabel(form, text="Hotkey global", text_color=MUTED).pack(anchor="w", padx=16, pady=(4, 2))
        ctk.CTkEntry(form, textvariable=self.hotkey_var, height=38).pack(fill="x", padx=16, pady=(0, 10))
        ctk.CTkCheckBox(form, text="Iniciar com o Windows", variable=self.startup_var).pack(anchor="w", padx=16, pady=(4, 12))
        ctk.CTkButton(form, text="Salvar configuracoes", height=40, command=self._save_settings).pack(
            fill="x", padx=16, pady=(0, 16)
        )

    def _build_logs(self) -> None:
        page = self._page_frame()
        card = self._card(page, "Logs", "Eventos locais. Tokens e chaves nao sao salvos aqui.")
        card.pack(fill="both", expand=True)
        ctk.CTkButton(card, text="Atualizar logs", width=140, command=self._refresh_logs).pack(anchor="w", padx=16, pady=(10, 8))
        self.logs_box = ctk.CTkTextbox(card, fg_color="#0b1220", border_color=LINE, border_width=1)
        self.logs_box.pack(fill="both", expand=True, padx=16, pady=(0, 16))
        self._refresh_logs()

    def _page_frame(self) -> ctk.CTkFrame:
        page = ctk.CTkFrame(self.content, fg_color=BG, corner_radius=0)
        page.pack(fill="both", expand=True)
        return page

    def _card(self, parent: ctk.CTkFrame, title: str, subtitle: str = "") -> ctk.CTkFrame:
        card = ctk.CTkFrame(parent, fg_color=SURFACE, corner_radius=14, border_color=LINE, border_width=1)
        ctk.CTkLabel(card, text=title, font=("Segoe UI", 18, "bold"), text_color=TEXT).pack(anchor="w", padx=16, pady=(16, 2))
        if subtitle:
            ctk.CTkLabel(card, text=subtitle, text_color=MUTED, wraplength=520, justify="left").pack(anchor="w", padx=16)
        return card

    def _metric_card(self, parent: ctk.CTkFrame, title: str, variable: ctk.StringVar, column: int) -> None:
        card = ctk.CTkFrame(parent, fg_color=SURFACE, corner_radius=12, border_color=LINE, border_width=1)
        card.grid(row=0, column=column, sticky="nsew", padx=(0 if column == 0 else 7, 0 if column == 3 else 7))
        ctk.CTkLabel(card, text=title, text_color=MUTED).pack(anchor="w", padx=14, pady=(12, 2))
        ctk.CTkLabel(card, textvariable=variable, text_color=TEXT, font=("Segoe UI", 13, "bold"), wraplength=230).pack(
            anchor="w", padx=14, pady=(0, 12)
        )

    def _start_voice(self) -> None:
        device_value = self.config.get("microphone_device", "")
        device: int | str | None = None
        if str(device_value).strip():
            try:
                device = int(device_value)
            except ValueError:
                device = str(device_value)
        self.engine = VoiceEngine(
            wake_words=self.config.get("wake_words", ["jarvis", "assistente"]),
            language=self.config.get("idioma", "pt-BR"),
            listen_seconds=int(self.config.get("listen_seconds", 7)),
            device=device,
            on_text=lambda text: self.root.after(0, lambda: self._process_text(text, "voz")),
            on_state=lambda state: self.root.after(0, lambda: self._update_voice_state(state)),
        )
        self.engine.start()

    def _register_hotkey(self) -> None:
        try:
            import keyboard

            keyboard.add_hotkey(self.config.get("hotkey", "ctrl+shift+j").lower(), self._arm_voice)
            self.hotkey_registered = True
        except Exception as exc:
            self.status_var.set("Hotkey indisponivel")
            self.subtitle_var.set(str(exc))

    def _start_tray(self) -> None:
        try:
            import pystray
            from PIL import Image, ImageDraw

            icon_path = resolve_asset("pc-control.ico")
            if icon_path.exists():
                image = Image.open(icon_path)
            else:
                image = Image.new("RGB", (64, 64), "#0b1220")
                draw = ImageDraw.Draw(image)
                draw.rounded_rectangle((10, 10, 54, 54), radius=12, fill="#38bdf8")
                draw.ellipse((24, 20, 40, 36), fill="#0b1220")

            def show_window(icon, item):  # noqa: ANN001
                self.root.after(0, self._show_window)

            def quit_app(icon, item):  # noqa: ANN001
                icon.stop()
                self.root.after(0, self._quit_app)

            menu = pystray.Menu(
                pystray.MenuItem(f"Abrir {APP_NAME}", show_window),
                pystray.MenuItem("Falar agora", lambda icon, item: self.root.after(0, self._arm_voice)),
                pystray.MenuItem("Sair", quit_app),
            )
            self.tray_icon = pystray.Icon(APP_NAME, image, APP_NAME, menu)
            threading.Thread(target=self.tray_icon.run, daemon=True).start()
        except Exception:
            pass

    def _arm_voice(self) -> None:
        if self.engine:
            self.engine.resume()
            self.engine.arm()
            self.status_var.set("Ouvindo")
            self.subtitle_var.set("Fale seu comando agora.")

    def _toggle_pause(self) -> None:
        if not self.engine:
            return
        if self.engine.state.paused:
            self.engine.resume()
            self.status_var.set("Microfone ativo")
            self.subtitle_var.set("Aguardando wake word ou hotkey.")
        else:
            self.engine.pause()
            self.status_var.set("Microfone pausado")
            self.subtitle_var.set("Use o botao ou a hotkey para voltar.")

    def _confirm_pending(self) -> None:
        result = self.router.handle_text("confirmar", confirmed=True)
        self._show_result(result)

    def _submit_command(self) -> None:
        text = self.command_var.get().strip()
        if not text:
            return
        self.command_var.set("")
        self._process_text(text, "texto")

    def _process_text(self, text: str, source: str) -> None:
        text = (text or "").strip()
        if not text:
            return
        self.last_text_var.set(f"{text} ({source} {timestamp()})")
        self.status_var.set("Executando")
        self.subtitle_var.set(text)
        result = self.router.handle_text(text)
        self._show_result(result)
        self._refresh_logs()

    def _show_result(self, result: ExecutionResult) -> None:
        if result.intent:
            target = result.intent.target or result.intent.parameters
            self.intent_var.set(f"{result.intent.type} -> {target}")
        if result.action:
            self.action_var.set(result.action.label)
        self.result_var.set(result.message)
        if result.needs_confirmation:
            self.status_var.set("Aguardando confirmacao")
            self.subtitle_var.set("Diga confirmar ou clique no botao Confirmar.")
        elif result.success:
            self.status_var.set("Pronto")
            self.subtitle_var.set("Acao executada com sucesso.")
        else:
            self.status_var.set("Atencao")
            self.subtitle_var.set(result.message)
        self._add_history(result)

    def _add_history(self, result: ExecutionResult) -> None:
        self.history.append(
            {
                "time": timestamp(),
                "input": self.last_text_var.get(),
                "intent": self.intent_var.get(),
                "action": self.action_var.get(),
                "result": result.message,
            }
        )
        self.history = self.history[-80:]
        self._render_history()

    def _render_history(self) -> None:
        if not self.chat_box:
            return
        self.chat_box.configure(state="normal")
        self.chat_box.delete("1.0", "end")
        for item in self.history[-40:]:
            self.chat_box.insert("end", f"[{item['time']}] Usuario: {item['input']}\n")
            self.chat_box.insert("end", f"Intencao: {item['intent']}\n")
            self.chat_box.insert("end", f"Acao: {item['action']}\n")
            self.chat_box.insert("end", f"{APP_NAME}: {item['result']}\n\n")
        self.chat_box.configure(state="disabled")

    def _update_voice_state(self, state: VoiceEngineState) -> None:
        if self.audio_bar:
            self.audio_bar.set(state.audio_level)
        if state.last_error:
            self.status_var.set("Erro de voz")
            self.subtitle_var.set(state.last_error)
        elif state.paused:
            self.status_var.set("Microfone pausado")
            self.subtitle_var.set("A escuta esta desligada temporariamente.")
        elif state.listening:
            self.status_var.set("Ouvindo")
            self.subtitle_var.set("Fale o comando.")
        elif state.model_ready:
            self.status_var.set("Pronto para ouvir")
            device = f" em {state.device_name}" if state.device_name else ""
            self.subtitle_var.set(f"Aguardando wake word ou hotkey{device}.")
        if state.last_text:
            self.last_text_var.set(state.last_text)
            if self.transcript_box:
                self.transcript_box.configure(state="normal")
                self.transcript_box.insert("end", f"\n[{timestamp()}] {state.last_text}")
                self.transcript_box.see("end")
                self.transcript_box.configure(state="disabled")

    def _play_spotify_query(self) -> None:
        query = self.spotify_query_var.get().strip()
        if query:
            self._process_text(f"tocar {query}", "spotify")

    def _save_routine(self) -> None:
        name = self.routine_name_var.get().strip().lower()
        if not name:
            messagebox.showerror("Rotina", "Informe um nome para a rotina.")
            return
        if self.routine_steps_box is None:
            return
        lines = [line.strip() for line in self.routine_steps_box.get("1.0", "end").splitlines() if line.strip()]
        actions = []
        for line in lines:
            action = self.router.intent_to_action(parse_local(line, self.config))
            if action:
                actions.append(asdict(action))
        if not actions:
            messagebox.showerror("Rotina", "Nenhum passo virou uma acao segura.")
            return
        self.config.setdefault("rotinas", {})[name] = actions
        self._save_config()
        self.router.config = self.config
        self._show_page("routines")
        self.status_var.set("Rotina salva")
        self.subtitle_var.set(name)

    def _delete_routine(self, name: str) -> None:
        if name in self.config.get("rotinas", {}):
            del self.config["rotinas"][name]
            self._save_config()
            self.router.config = self.config
            self._show_page("routines")

    def _save_openai_key(self) -> None:
        key = self.openai_key_var.get().strip()
        if not key:
            messagebox.showerror("OpenAI", "Cole uma chave valida.")
            return
        set_secret("openai_api_key", key)
        self.config["usar_openai"] = True
        self._save_config()
        self.router.config = self.config
        self.openai_key_var.set("")
        messagebox.showinfo("OpenAI", "Chave salva com seguranca local.")
        self._show_page("integrations")

    def _test_openai(self) -> None:
        self.status_var.set("Testando OpenAI")

        def worker() -> None:
            ok, message = test_openai_connection(self.config)
            self.root.after(0, lambda: self._integration_result("OpenAI", ok, message))

        threading.Thread(target=worker, daemon=True).start()

    def _save_spotify_client(self) -> None:
        client_id = self.spotify_client_var.get().strip()
        self.config.setdefault("spotify_web", {})["client_id"] = client_id
        self._save_config()
        self.router.config = self.config
        messagebox.showinfo("Spotify", "Client ID salvo.")

    def _connect_spotify(self) -> None:
        self._save_spotify_client()
        self.status_var.set("Conectando Spotify")
        self.subtitle_var.set("O navegador sera aberto para autorizacao.")

        def worker() -> None:
            ok, message = authorize_pkce(self.config)
            self.root.after(0, lambda: self._integration_result("Spotify", ok, message, refresh_page="integrations"))

        threading.Thread(target=worker, daemon=True).start()

    def _save_google_client(self) -> None:
        client_id = self.google_client_var.get().strip()
        self.config.setdefault("google_auth", {})["client_id"] = client_id
        self.config["google_auth"]["redirect_uri"] = "http://127.0.0.1:43880/google/callback"
        self.config["google_auth"]["scopes"] = "openid email profile"
        self.config["google_auth"]["remember_session"] = False
        self._save_config()
        self.router.config = self.config

    def _login_google(self) -> None:
        self._save_google_client()
        client_id = self.config.get("google_auth", {}).get("client_id", "")
        self.status_var.set("Conectando Google")
        self.subtitle_var.set("O navegador sera aberto para login opcional.")

        def worker() -> None:
            try:
                ok, message = self.google_auth.login(client_id)
            except Exception as exc:
                ok, message = False, f"Falha no login Google: {exc}"
            self.root.after(0, lambda: self._google_result(ok, message))

        threading.Thread(target=worker, daemon=True).start()

    def _logout_google(self) -> None:
        self.google_auth.logout()
        self.google_status_var.set(self.google_auth.status_label())
        self.status_var.set("Modo local")
        self.subtitle_var.set("Sessao Google temporaria encerrada.")
        if self.current_page == "integrations":
            self._show_page("integrations")

    def _google_result(self, ok: bool, message: str) -> None:
        self.google_status_var.set(self.google_auth.status_label())
        self.status_var.set("Google conectado" if ok else "Google indisponivel")
        self.subtitle_var.set(message)
        if self.current_page == "integrations":
            self._show_page("integrations")
        messagebox.showinfo("Google", message) if ok else messagebox.showerror("Google", message)

    def _integration_result(self, name: str, ok: bool, message: str, refresh_page: str | None = None) -> None:
        self.status_var.set(f"{name}: {'OK' if ok else 'falha'}")
        self.subtitle_var.set(message)
        if refresh_page:
            self._show_page(refresh_page)
        messagebox.showinfo(name, message) if ok else messagebox.showerror(name, message)

    def _save_microphone(self) -> None:
        selected = self.mic_var.get().split(" - ", 1)[0].strip()
        self.config["microphone_device"] = "" if selected == "-1" else selected
        self._save_config()
        messagebox.showinfo("Microfone", "Microfone salvo. Reinicie o painel para trocar o stream de audio.")

    def _save_settings(self) -> None:
        self.config["wake_words"] = [item.strip() for item in self.wake_var.get().split(",") if item.strip()]
        self.config["hotkey"] = self.hotkey_var.get().strip() or "Ctrl+Shift+J"
        self.config["iniciar_com_windows"] = bool(self.startup_var.get())
        self._save_config()
        self._configure_startup(bool(self.startup_var.get()))
        messagebox.showinfo("Configuracoes", "Configuracoes salvas. Reinicie para reaplicar hotkey e wake words.")

    def _configure_startup(self, enabled: bool) -> None:
        if os.name != "nt":
            return
        import winreg

        run_key = r"Software\Microsoft\Windows\CurrentVersion\Run"
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, run_key, 0, winreg.KEY_SET_VALUE) as key:
            if not enabled:
                try:
                    winreg.DeleteValue(key, "PC Control")
                except FileNotFoundError:
                    pass
                return
            if is_frozen():
                command = f'"{executable_path()}"'
            else:
                command = f'"{source_pythonw()}" "{app_root() / "assistant_panel.py"}"'
            winreg.SetValueEx(key, "PC Control", 0, winreg.REG_SZ, command)

    def _refresh_logs(self) -> None:
        if not self.logs_box:
            return
        self.logs_box.configure(state="normal")
        self.logs_box.delete("1.0", "end")
        if LOG_PATH.exists():
            lines = LOG_PATH.read_text(encoding="utf-8").splitlines()[-180:]
            pretty_lines = []
            for line in lines:
                try:
                    entry = json.loads(line)
                    pretty_lines.append(f"[{entry.get('time')}] {entry.get('message')}")
                except Exception:
                    pretty_lines.append(line)
            self.logs_box.insert("end", "\n".join(pretty_lines))
        self.logs_box.configure(state="disabled")

    def _save_config(self) -> None:
        CONFIG_PATH.write_text(json.dumps(self.config, indent=2, ensure_ascii=False), encoding="utf-8")

    def _hide_window(self) -> None:
        self.root.withdraw()

    def _quit_app(self) -> None:
        if self.engine:
            self.engine.stop()
        clear_ephemeral_session()
        self.root.destroy()

    def _show_window(self) -> None:
        self.root.deiconify()
        self.root.lift()


def main() -> None:
    AssistantPanel().run()


if __name__ == "__main__":
    main()
