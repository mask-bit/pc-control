# Contribuindo

Obrigado por ajudar o Jarvis Assistant a ficar mais real e confiavel.

## Ambiente

```bat
python -m venv .venv
.venv\Scripts\activate
python -m pip install -r requirements-dev.txt
```

## Validacao local

```bat
python -m py_compile assistant_panel.py voice_engine.py intent_parser.py command_router.py local_executor.py spotify_controller.py workspace.py config_gui.py config_utils.py clap-trigger.py voice-trigger.py
python -m pytest
```

## Diretrizes

- O produto principal e o core Python de voz e execucao.
- O app Tauri em `assistant-desktop/` e experimental.
- Nao adicione comandos de shell livre executados pela IA sem confirmacao explicita.
- Nao salve chaves, tokens ou dados pessoais em arquivos versionados.
- Priorize comandos reais e testaveis antes de telas ou simulacoes.
