# Release

## Checklist

1. Atualizar `CHANGELOG.md`.
2. Rodar validacoes:

```bat
python -m py_compile assistant_panel.py voice_engine.py intent_parser.py command_router.py local_executor.py spotify_controller.py workspace.py config_gui.py config_utils.py clap-trigger.py voice-trigger.py
python -m pytest
cmd /c build.bat
```

3. Testar manualmente:

```text
abre o Chrome
abre YouTube
toca playlist foco
pausa Spotify
ativar modo estudo
```

4. Publicar artefatos de `dist/`.
