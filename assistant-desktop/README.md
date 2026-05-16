# PC Control AI Desktop

App principal em Tauri + React para o assistente local de PC. Ele abre sem login, roda `Qwen3-8B-Q5_0.gguf` via `llama-server.exe` embutido e executa automaticamente acoes comuns do Windows.

## Stack

- Tauri 2
- React 19 + TypeScript
- Vite
- Rust no nucleo local
- SQLite para dados locais
- `llama.cpp` como runtime local do modelo
- Windows Credential Manager para tokens opcionais

## Rodar em desenvolvimento

```bat
npm install
npm run prepare:llama
npm run dev
```

Para rodar como aplicativo Tauri, instale Rust/Cargo e depois execute:

```bat
npm run tauri:dev
```

## Build

```bat
npm run build
npm run tauri:build
```

O build nativo gera setup `.exe` via NSIS e inclui o runtime local. O GGUF fica como arquivo externo configuravel porque o NSIS nao suporta um setup unico com esse modelo de 5,7 GB.

## IA local

A V1 nao pede chave de API. O provider padrao e `qwen_embedded`, que inicia `llama-server.exe` em `127.0.0.1:18181`, carrega `Qwen3-8B-Q5_0.gguf` e exige resposta JSON antes de executar qualquer acao.

Google e Spotify sao opcionais. Google fica reservado para sincronizacao futura; Spotify usa OAuth PKCE quando o usuario decide conectar.
