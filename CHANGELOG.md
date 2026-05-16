# Changelog

## 0.4.0

- App reestruturado para a V1 do PC Control AI local.
- Removida a interface de chaves de API externas.
- Google deixou de ser obrigatorio e ficou opcional.
- Adicionadas as 9 telas principais da V1.
- Adicionado `local_ai.rs` com provider Qwen/llama.cpp embutido e fallback de regras.
- Adicionado preparo de `Qwen3-8B-Q5_0.gguf` e `llama-server.exe` para instalador NSIS.
- Adicionado controle real/semiautomatico do Windows para apps, volume, screenshots e processos.
- Adicionada tela de voz com push-to-talk e medidor de microfone.

## 0.3.0

- Tauri/React virou o app principal.
- Removidos o app antigo, instaladores antigos e testes antigos.
- Chat passou a executar automaticamente acoes seguras e bloquear acoes sensiveis.
