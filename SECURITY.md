# Politica De Seguranca

## Modelo De Seguranca

O PC Control AI executa acoes locais no computador. Toda mudanca deve preservar estes limites:

- a IA nao pode executar PowerShell, CMD ou shell livre;
- acoes sensiveis ou irreversiveis devem exigir confirmacao explicita;
- logs nao devem armazenar chaves, tokens, senhas ou dados sensiveis;
- tokens OAuth opcionais devem ficar apenas no armazenamento seguro do sistema;
- o SQLite local deve guardar somente dados operacionais e perfil Google basico opcional.

## Segredos

- Configure `GOOGLE_OAUTH_CLIENT_ID` apenas se for testar o Google opcional.
- Nunca commite `.env`, tokens OAuth, cookies, dumps de logs sensiveis ou chaves.
- A V1 nao pede chave de API para funcionar.

## Reportar Problema

Abra uma issue com:

- versao do Windows;
- versao do app;
- acao solicitada no chat;
- resultado esperado;
- resultado obtido;
- logs sem dados sensiveis.
