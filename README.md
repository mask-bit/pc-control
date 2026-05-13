# PC Control

Aplicativo desktop real para Windows, em portugues-BR, focado em controle local do PC por texto e voz. Ele abre apps, sites e pastas, pesquisa no YouTube, controla Spotify, ajusta volume, fecha programas e executa rotinas simples.

O app principal e Python + CustomTkinter, com Vosk offline para voz, executor local seguro, OpenAI opcional para interpretar frases vagas, Spotify por URI/teclas ou Web API OAuth PKCE, e login Google opcional para sessao local leve.

## Status

Alpha funcional com empacotamento de produto.

- App principal: `PCControl.exe`.
- Instalador Windows: Inno Setup per-user.
- Dados do usuario: `%APPDATA%\PC Control`.
- Cache/sessao temporaria: `%LOCALAPPDATA%\PC Control`.
- Login Google: opcional, escopos `openid email profile`, sem persistencia padrao.
- Tauri/React: experimental em `assistant-desktop/`.

## Quickstart

Modo desenvolvimento:

```bat
python -m venv .venv
.venv\Scripts\activate
python -m pip install -r requirements.txt
python assistant_panel.py
```

Na primeira execucao de voz, o app baixa o modelo Vosk portugues `vosk-model-small-pt-0.3`.

## Build Windows

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

Para gerar o instalador, instale o Inno Setup 6 antes. Se `ISCC.exe` nao estiver disponivel, `build-installer.bat` mostra uma mensagem clara.

## Comandos de exemplo

```text
abre o Chrome
abre YouTube
pesquisar lo-fi no YouTube
abrir Chrome e Spotify
toca playlist foco
pausa Spotify
proxima musica
fechar Spotify
volume para 30
ativar modo estudo
listar comandos
```

## Integracoes

OpenAI e opcional. Voce pode salvar a chave na tela Integracoes ou usar:

```bat
set OPENAI_API_KEY=sk-sua-chave
```

Spotify Web API e opcional. Configure no Spotify Developer Dashboard o redirect URI `http://127.0.0.1:43879/callback`, cole o Client ID na tela Integracoes e clique em Conectar Spotify.

Google e opcional. Crie um OAuth Client ID de app desktop no Google Cloud, cole na tela Integracoes e clique em Entrar com Google. O app usa `openid email profile`, nao salva senha e nao persiste a sessao por padrao.

## Estrutura

```text
assistant_panel.py        Painel principal do PC Control
app_paths.py              Identidade, AppData, cache, sessao e assets
auth_google.py            Login Google opcional via OAuth PKCE local
voice_engine.py           Microfone, Vosk, wake words e hotkey
intent_parser.py          Parser local de comandos em portugues-BR
command_router.py         Orquestrador seguro de intencoes e acoes
local_executor.py         Executor Windows: apps, sites, YouTube, pastas, volume, clipboard, fechar apps
spotify_controller.py     Spotify por URI/teclas e Web API OAuth PKCE
openai_controller.py      Interpretacao e resposta opcional via OpenAI Responses API
secrets_store.py          Segredos locais com keyring ou DPAPI no Windows
assets/                   Icone e logo do app
installer/                Script Inno Setup
tests/                    Testes automatizados
```

## Validacao

```bat
python -m py_compile assistant_panel.py app_paths.py auth_google.py voice_engine.py intent_parser.py command_router.py local_executor.py spotify_controller.py openai_controller.py secrets_store.py
python -m pytest
python -m ruff check assistant_panel.py app_paths.py auth_google.py voice_engine.py intent_parser.py command_router.py local_executor.py spotify_controller.py openai_controller.py secrets_store.py tests
```

## Seguranca

O app nao executa shell livre por IA. Comandos desconhecidos falham de forma segura. Chaves e tokens ficam fora do repositorio; Google usa sessao temporaria por padrao; logs nao devem conter segredos.

Veja [SECURITY.md](SECURITY.md).
