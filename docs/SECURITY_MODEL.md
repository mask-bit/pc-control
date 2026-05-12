# Modelo de seguranca

## Principios

- O comando de voz nunca deve virar shell livre automaticamente.
- O parser local cobre comandos essenciais sem depender de IA externa.
- Acoes desconhecidas devem falhar de forma segura.
- Acoes sensiveis devem exigir confirmacao.

## Riscos controlados

- Abrir apps/sites e considerado baixo risco.
- Copiar texto e controlar volume sao baixo risco.
- Executar comandos arbitrarios nao faz parte do MVP.
- OpenAI e opcional e nao deve bloquear o funcionamento offline.

## Logs

`assistant_logs.jsonl` e local e ignorado pelo Git. Ele serve para depuracao, mas nao deve receber segredos.
