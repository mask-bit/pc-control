# Assistente Inteligente Desktop Experimental

Prototipo visual em Tauri + React. Ele nao e o produto principal no momento.

O app real de controle por voz agora fica no core Python da raiz do projeto, iniciado por:

```bat
python assistant_panel.py
```

Use este pacote Tauri apenas como referencia visual/experimental enquanto o motor real de voz, comandos e execucao evolui em Python.

## Stack

- Tauri 2
- React 19 + TypeScript
- Vite
- Rust no nucleo local
- SQLite para dados locais
- Windows Credential Manager via camada nativa para credenciais

## Rodar em desenvolvimento

```bat
npm install
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

## Credenciais

O app foi desenhado para BYO local: cada usuario informa a propria OpenAI API key e conecta a propria conta Spotify por OAuth PKCE. Tokens nao devem ser salvos no SQLite.
