# Roadmap

## V1 Atual

- App Tauri/React como produto principal.
- 9 telas funcionais: Inicio, Assistente IA, Controle do PC, Aplicativos, Musica, Voz, Automacoes, Historico e Configuracoes.
- IA local real com Qwen GGUF rodando em `llama-server.exe` embutido.
- Uso sem conta; Google opcional para sincronizacao futura.
- Execucao automatica protegida de acoes comuns do Windows.
- Voz push-to-talk com captura/teste de microfone e aviso quando a transcricao local ainda nao existe.

## Proximas Melhorias

- Melhorar prompt/JSON schema do Qwen para mais tipos de tarefa.
- Edicao visual de rotinas com acoes estruturadas.
- Melhor deteccao de apps instalados no Windows.
- Controle de janelas com foco, minimizar e maximizar reais.
- Testes automatizados para filtro de seguranca Rust.

## Futuro

- Wake word/escuta continua opcional.
- Memoria local de preferencias.
- Sincronizacao opcional via Google.
- Plugin system para novas ferramentas do assistente.
