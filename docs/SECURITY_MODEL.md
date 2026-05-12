# Modelo de seguranca

## Principios

- O comando de voz nunca deve virar shell livre automaticamente.
- O parser local cobre comandos essenciais sem depender de IA externa.
- Acoes desconhecidas devem falhar de forma segura.
- Acoes sensiveis devem exigir confirmacao.
- Chaves e tokens devem ficar fora de arquivos versionados.

## Riscos controlados

- Abrir apps/sites e considerado baixo risco.
- Copiar texto e controlar volume sao baixo risco.
- Executar comandos arbitrarios nao faz parte do MVP.
- OpenAI e opcional e nao deve bloquear o funcionamento offline.
- Spotify OAuth usa PKCE e nao armazena client secret.
- Rotinas armazenam acoes estruturadas (`ActionSpec`), nao scripts livres.

## Credenciais

- OpenAI API key pode vir da variavel `OPENAI_API_KEY` ou ser salva pelo painel.
- Token Spotify e chave OpenAI salvos pelo painel usam keyring quando disponivel ou DPAPI no Windows.
- `assistant_config.json` guarda apenas configuracao operacional, como Client ID Spotify, apps, sites e rotinas.

## Logs

`assistant_logs.jsonl` e local e ignorado pelo Git. Ele serve para depuracao, mas nao deve receber segredos.
