# Setup

## Requisitos

- Windows 10/11.
- Python 3.10 ou superior.
- Microfone funcional.
- Internet na primeira execucao de voz para baixar o modelo Vosk portugues.

## Instalar

```bat
python -m venv .venv
.venv\Scripts\activate
python -m pip install -r requirements.txt
```

## Rodar

```bat
python assistant_panel.py
```

ou:

```bat
start-jarvis.bat
```

## Build

```bat
build.bat
```

Saida esperada:

```text
dist\JarvisAssistant.exe
dist\WorkspaceLauncher.exe
dist\WorkspaceConfig.exe
dist\assistant_config.json
dist\workspace-config.json
```

## Tauri experimental

```bat
cd assistant-desktop
npm install
npm run dev
```

Para `npm run tauri:build`, instale Rust/Cargo antes.
