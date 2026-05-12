# Jarvis Assistant

Assistente desktop real para Windows, em portugues-BR, inspirado em uma Alexa/Jarvis para PC. O foco e transformar texto ou fala em acoes reais: abrir aplicativos, abrir YouTube, pesquisar, controlar Spotify, fechar apps, ajustar volume e executar rotinas.

O app principal e o core Python com painel profissional em CustomTkinter, Vosk offline para voz, executor local seguro, OpenAI opcional para interpretar frases vagas e Spotify com fallback por URI/teclas de midia ou Web API via OAuth PKCE. O prototipo Tauri em `assistant-desktop/` existe apenas como experimento visual.

## Status

Alpha funcional com reconstrução do produto principal.

- Core Python real: ativo.
- Voz offline com Vosk: ativo.
- Hotkey + wake words: ativo.
- Executor local Windows: apps, sites, YouTube, pastas, fechar apps, clipboard e volume.
- Spotify: URI/teclas de midia e Web API opcional via OAuth PKCE.
- OpenAI: opcional, com chave local segura, para interpretacao avançada e respostas curtas.
- Painel: Inicio, Assistente, Spotify, Rotinas, Voz e audio, Integracoes, Configuracoes e Logs.
- Tauri/React: experimental.

## Quickstart

App principal real:

```bat
python -m venv .venv
.venv\Scripts\activate
python -m pip install -r requirements.txt
python assistant_panel.py
```

Ou rode:

```bat
start-jarvis.bat
```

Na primeira execucao de voz, o app baixa o modelo Vosk portugues `vosk-model-small-pt-0.3`.

Prototipo visual experimental:

```bat
npm install
npm run dev
```

Esses comandos na raiz redirecionam para `assistant-desktop/`.

## Comandos de exemplo

```text
abre o Chrome
abre YouTube
pesquisar lo-fi no YouTube
pesquisa Python no Google
abrir Chrome e Spotify
toca playlist foco
pausa Spotify
proxima musica
fechar Spotify
aumenta volume
ativar modo estudo
listar comandos
```

## Build Windows

```bat
build.bat
```

Saida:

```text
dist\JarvisAssistant.exe
dist\WorkspaceLauncher.exe
dist\WorkspaceConfig.exe
dist\assistant_config.json
dist\workspace-config.json
```

## Estrutura

```text
assistant_panel.py        Painel principal premium do assistente
voice_engine.py           Microfone, Vosk, wake words e hotkey
intent_parser.py          Parser local de comandos em portugues-BR
command_router.py         Orquestrador seguro de intencoes e acoes
local_executor.py         Executor Windows: apps, sites, YouTube, pastas, volume, clipboard, fechar apps
spotify_controller.py     Spotify por URI/teclas e Web API OAuth PKCE
openai_controller.py      Interpretacao e resposta opcional via OpenAI Responses API
secrets_store.py          Segredos locais com keyring ou DPAPI no Windows
assistant_config.json     Configuracao do assistente
workspace.py              Launcher legado e compatibilidade
config_gui.py             Configurador legado
assistant-desktop/        Prototipo visual Tauri/React experimental
docs/                     Documentacao tecnica
tests/                    Testes automatizados
```

## Arquitetura

```mermaid
flowchart LR
  Mic["Microfone / hotkey / wake word"] --> Voice["voice_engine.py"]
  Voice --> Parser["intent_parser.py"]
  Parser --> Router["command_router.py"]
  Router --> Exec["local_executor.py"]
  Router --> Spotify["spotify_controller.py"]
  Parser --> OpenAI["openai_controller.py opcional"]
  OpenAI --> Secrets["secrets_store.py"]
  Router --> Logs["assistant_logs.jsonl"]
  Panel["assistant_panel.py"] --> Voice
  Config["assistant_config.json"] --> Parser
  Config --> Router
```

## Configuracao

Edite `assistant_config.json` ou use o painel para adicionar integrações e rotinas. Apps, sites, pastas, playlists e rotinas ficam no JSON; chaves e tokens ficam fora dele.

OpenAI e opcional. Voce pode salvar a chave na tela Integracoes ou usar variavel de ambiente:

```bat
set OPENAI_API_KEY=sk-sua-chave
```

Sem OpenAI, os comandos essenciais continuam funcionando pelo parser local.

Spotify Web API e opcional. Para conectar, crie um app no Spotify Developer Dashboard, configure o redirect URI `http://127.0.0.1:43879/callback`, cole o Client ID na tela Integracoes e clique em Conectar Spotify.

## Validacao

```bat
python -m py_compile assistant_panel.py voice_engine.py intent_parser.py command_router.py local_executor.py spotify_controller.py workspace.py config_gui.py config_utils.py clap-trigger.py voice-trigger.py
python -m pytest
```

Para tooling de desenvolvimento:

```bat
python -m pip install -r requirements-dev.txt
```

## Documentacao

- [Arquitetura](docs/ARCHITECTURE.md)
- [Setup](docs/SETUP.md)
- [Comandos](docs/COMMANDS.md)
- [Modelo de seguranca](docs/SECURITY_MODEL.md)
- [Release](docs/RELEASE.md)
- [Roadmap](ROADMAP.md)

## Seguranca

O app executa acoes locais no computador, entao o projeto evita shell livre no MVP. Comandos desconhecidos falham de forma segura, acoes sensiveis devem pedir confirmacao e logs locais sao ignorados pelo Git.

Veja [SECURITY.md](SECURITY.md).

## Licenca

MIT. Veja [LICENSE](LICENSE).
