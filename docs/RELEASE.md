# Release

## Checklist

1. Atualizar `CHANGELOG.md`.
2. Rodar validacoes:

```bat
python -m py_compile assistant_panel.py app_paths.py auth_google.py voice_engine.py intent_parser.py command_router.py local_executor.py spotify_controller.py openai_controller.py secrets_store.py
python -m pytest
cmd /c build.bat
cmd /c build-installer.bat
```

3. Testar manualmente:

```text
abre o Chrome
abre YouTube
toca playlist foco
pausa Spotify
ativar modo estudo
```

4. Publicar `dist\PCControl.exe` e `dist\PCControlSetup.exe`.
