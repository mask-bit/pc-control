# Contribuindo

Obrigado por ajudar o PC Control AI a ficar mais confiavel.

## Ambiente

```bat
npm install
```

Para rodar o app nativo, instale Rust/Cargo:

```bat
npm run tauri:dev
```

## Validacao Local

```bat
npm run build:frontend
```

Quando Rust/Cargo estiver disponivel:

```bat
npm run tauri:build
```

## Diretrizes

- O produto principal e o app Tauri/React em `assistant-desktop/`.
- Nao adicione execucao de shell livre controlada pela IA.
- Nao salve chaves, tokens ou dados pessoais em arquivos versionados.
- Acoes sensiveis devem continuar bloqueadas ou exigir confirmacao explicita.
