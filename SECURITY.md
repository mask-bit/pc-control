# Politica de Seguranca

## Modelo de seguranca

O Jarvis Assistant executa acoes locais no computador. Por isso, toda mudanca deve preservar estes limites:

- comandos desconhecidos nao devem executar nada;
- acoes sensiveis devem pedir confirmacao;
- logs nao devem armazenar chaves, tokens ou segredos;
- a IA, quando habilitada, deve propor intencoes estruturadas, nao comandos arbitrarios;
- o modo offline com Vosk deve continuar funcionando sem OpenAI.

## Segredos

- Use variaveis de ambiente para chaves opcionais, como `OPENAI_API_KEY`.
- Nunca commite arquivos `.env`, tokens OAuth, cookies, dumps de logs sensiveis ou modelos baixados.

## Reportar problema

Abra uma issue com:

- versao do Windows;
- versao do Python;
- comando de voz ou texto usado;
- resultado esperado;
- resultado obtido;
- logs sem dados sensiveis.
