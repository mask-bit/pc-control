# Arquitetura

## Visao geral

O projeto tem um nucleo Python real para controle do PC por voz e texto. A interface Tauri existe como experimento visual, mas nao e o produto principal. O produto oficial e o app Windows **PC Control**, iniciado por `assistant_panel.py` no modo fonte e por `PCControl.exe` no modo instalado.

```mermaid
flowchart LR
  Mic["Microfone / hotkey / wake word"] --> Voice["voice_engine.py"]
  Voice --> Parser["intent_parser.py"]
  Parser --> OpenAI["openai_controller.py opcional"]
  Parser --> Router["command_router.py"]
  Router --> Exec["local_executor.py"]
  Router --> Spotify["spotify_controller.py"]
  Router --> Logs["assistant_logs.jsonl"]
  OpenAI --> Secrets["secrets_store.py"]
  Spotify --> Secrets
  Panel --> Google["auth_google.py opcional"]
  Panel --> Paths["app_paths.py"]
  Panel["assistant_panel.py"] --> Voice
  Panel --> Router
  Config["assistant_config.json"] --> Parser
  Config --> Router
```

## Modulos principais

- `assistant_panel.py`: janela principal profissional com Inicio, Assistente, Spotify, Rotinas, Voz e audio, Integracoes, Configuracoes e Logs.
- `app_paths.py`: identidade do produto, AppData, LocalAppData, cache, sessao e assets.
- `auth_google.py`: login Google opcional via OAuth PKCE local, com sessao temporaria.
- `voice_engine.py`: captura de audio, Vosk offline, wake words, hotkey, selecao de microfone e nivel de entrada.
- `intent_parser.py`: converte texto em `CommandIntent`, incluindo comandos compostos como "abrir Chrome e Spotify".
- `command_router.py`: valida intencao, cria `ActionSpec`, pede confirmacao quando necessario e executa com seguranca.
- `local_executor.py`: integra com Windows para apps, sites, YouTube, pastas, fechar apps, volume e clipboard.
- `spotify_controller.py`: controla Spotify por URI/teclas de midia e Web API via OAuth PKCE quando conectado.
- `openai_controller.py`: usa OpenAI Responses API opcionalmente para interpretar frases vagas e gerar respostas curtas.
- `secrets_store.py`: salva chaves/tokens fora do JSON, usando keyring quando disponivel ou DPAPI no Windows.
- `installer/PCControl.iss`: instalador Inno Setup per-user.

## Contrato de comando

1. Voz ou texto entra como frase natural.
2. Parser cria uma intencao estruturada.
3. Router cria uma acao segura.
4. Executor roda a acao ou pede confirmacao.
5. Resultado e salvo nos logs.

## Regras de seguranca

- A IA nao executa shell livre.
- Comandos desconhecidos falham de forma segura.
- Tokens e chaves nao sao gravados nos logs.
- OpenAI e Spotify sao opcionais; comandos essenciais continuam locais.
- Google e opcional, nao bloqueia o app e nao persiste sessao por padrao.
- Rotinas salvas guardam `ActionSpec`, nao texto arbitrario para shell.
