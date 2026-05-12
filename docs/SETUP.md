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

Na primeira execucao de voz, o app baixa o modelo Vosk portugues `vosk-model-small-pt-0.3`.

## Configurar OpenAI

OpenAI e opcional. Abra a tela **Integracoes**, cole sua chave e salve. A chave fica fora do `assistant_config.json`, protegida localmente por keyring ou DPAPI no Windows.

Tambem funciona por variavel de ambiente:

```bat
set OPENAI_API_KEY=sk-sua-chave
python assistant_panel.py
```

## Configurar Spotify Web API

O app funciona com Spotify por URI/teclas de midia sem login. Para controles Web API:

1. Crie um app no Spotify Developer Dashboard.
2. Adicione o redirect URI `http://127.0.0.1:43879/callback`.
3. Copie o Client ID.
4. No Jarvis, abra **Integracoes**, cole o Client ID e clique em **Conectar Spotify**.

O fluxo usa OAuth PKCE e nao usa client secret no desktop.

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
npm install
npm run dev
```

Os scripts da raiz redirecionam para `assistant-desktop/`.

Para `npm run tauri:build`, instale Rust/Cargo antes.
