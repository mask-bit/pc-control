# PC Control AI

Aplicativo desktop Windows em Tauri + React para usar o Qwen local como assistente de PC. A V1 abre sem conta, roda `Qwen3-8B-Q5_0.gguf` via `llama.cpp` embutido e executa acoes comuns do Windows com filtro de seguranca.

## Status

- App principal: `assistant-desktop/`.
- Interface: React + TypeScript + Vite.
- Nucleo local: Tauri 2 + Rust.
- Dados locais: SQLite no AppData do Tauri.
- IA: `Qwen3-8B-Q5_0.gguf` + `llama-server.exe` local.
- Login: sem conta por padrao; Google opcional para sincronizacao futura.
- Controle do PC: apps, volume, screenshots, processos, clipboard, Spotify e rotinas.

## Rodar em desenvolvimento

```bat
npm install
npm run prepare:llama
npm run tauri:dev
```

## Recuperar do GitHub

Se voce perder a pasta local, clone o repositorio e rode o preparo novamente:

```bat
git clone https://github.com/mask-bit/pc-control.git
cd pc-control
npm install
npm run prepare:llama
npm run tauri:dev
```

`npm run prepare:llama` baixa o runtime `llama.cpp` e, se o GGUF nao existir localmente, baixa `Qwen3-8B-Q5_0.gguf` do Hugging Face. Dados privados do Windows, tokens OAuth, logs e caches continuam fora do GitHub de proposito.

Para testar apenas a interface web:

```bat
npm run dev
```

## Build e instalador

```bat
npm run build:frontend
npm run tauri:build
```

`npm run tauri:build` precisa de Rust/Cargo instalado no PATH. O build Tauri roda `prepare:llama`, prepara o `llama-server.exe` e gera instalador NSIS `.exe`.

O instalador inclui o runtime nativo, mas nao embute o GGUF de 5,7 GB porque o NSIS nao suporta um setup desse tamanho. O app usa o caminho configurado em `Configuracoes > IA local` ou, neste ambiente, detecta `C:\Users\ezile\Downloads\ia\Qwen3-8B-Q5_0.gguf`.

## Como o app funciona

1. O usuario conclui o primeiro acesso com **Entrar sem conta**.
2. O app inicia `llama-server.exe` localmente em `127.0.0.1`.
3. O chat envia prompt + ferramentas para o Qwen local.
4. A IA retorna JSON com resposta e acoes estruturadas.
5. Acoes comuns executam automaticamente.
6. Acoes perigosas pedem confirmacao unica.
7. Tudo que foi feito, bloqueado ou falhou aparece no chat e no historico.

## Seguranca

- A IA nao executa shell livre.
- Acoes irreversiveis ou sensiveis exigem confirmacao.
- Se o modelo responder JSON invalido, nada e executado.
- Tokens OAuth ficam no armazenamento seguro do sistema.
- O app nao pede Google nem chave de API para funcionar na V1.

## Estrutura

```text
assistant-desktop/                  App principal Tauri + React
assistant-desktop/src/App.tsx       Interface V1 com 9 telas
assistant-desktop/src-tauri/src/    Backend local em Rust
assistant-desktop/scripts/          Preparo do llama.cpp e modelo GGUF
docs/ARCHITECTURE.md                Visao geral tecnica
```
