# Setup

## Requisitos

- Windows 10/11.
- Python 3.10 ou superior para modo fonte.
- Microfone funcional para comandos de voz.
- Internet na primeira execucao de voz para baixar o modelo Vosk portugues.
- Inno Setup 6 para gerar `PCControlSetup.exe`.

## Rodar em modo fonte

```bat
python -m venv .venv
.venv\Scripts\activate
python -m pip install -r requirements.txt
python assistant_panel.py
```

## Dados do usuario

O app instalado separa programa e dados:

```text
%APPDATA%\PC Control\assistant_config.json
%APPDATA%\PC Control\assistant_logs.jsonl
%LOCALAPPDATA%\PC Control\Cache
%LOCALAPPDATA%\PC Control\Session
```

Na primeira execucao, a config padrao e copiada para AppData.

## Configurar Google opcional

1. Crie um OAuth Client ID do tipo Desktop app no Google Cloud.
2. Abra **Integracoes** no PC Control.
3. Cole o Client ID.
4. Clique em **Entrar com Google**.

O app usa `openid email profile`, abre o navegador padrao e recebe o retorno em `http://127.0.0.1:43880/google/callback`. A sessao e temporaria por padrao e o app continua funcionando em modo local sem login.

## Configurar OpenAI

OpenAI e opcional. Abra **Integracoes**, cole a chave e salve. Tambem funciona por variavel de ambiente:

```bat
set OPENAI_API_KEY=sk-sua-chave
python assistant_panel.py
```

## Configurar Spotify Web API

O app funciona com Spotify por URI/teclas de midia sem login. Para controles Web API:

1. Crie um app no Spotify Developer Dashboard.
2. Adicione o redirect URI `http://127.0.0.1:43879/callback`.
3. Copie o Client ID.
4. No PC Control, abra **Integracoes**, cole o Client ID e clique em **Conectar Spotify**.

## Build

Executavel:

```bat
cmd /c build.bat
```

Instalador:

```bat
cmd /c build-installer.bat
```

Saida esperada:

```text
dist\PCControl.exe
dist\PCControlSetup.exe
```

## Tauri experimental

```bat
npm install
npm run dev
```

Os scripts da raiz redirecionam para `assistant-desktop/`.
