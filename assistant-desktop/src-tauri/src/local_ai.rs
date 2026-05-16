use crate::models::{
    ActionRisk, ActionSpec, AssistantSettings, ChatMessage, LocalAiStatus, MemoryEntry, TaskState,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::time::sleep;

const MODEL_NAME: &str = "Qwen3-8B-Q5_0.gguf";
const MODEL_LABEL: &str = "Qwen3-8B-Q5_0 embutido";
const SERVER_PORT: u16 = 18181;
const SERVER_HOST: &str = "127.0.0.1";
const SERVER_READY_TIMEOUT_SECS: u64 = 180;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAiOutput {
    pub assistant_reply: String,
    #[serde(default)]
    pub actions: Vec<ActionSpec>,
}

pub struct LlamaRuntimeState {
    child: Mutex<Option<Child>>,
    last_error: Mutex<Option<String>>,
}

impl LlamaRuntimeState {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            last_error: Mutex::new(None),
        }
    }

    fn set_error(&self, error: Option<String>) {
        if let Ok(mut current) = self.last_error.lock() {
            *current = error;
        }
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|error| error.clone())
    }
}

impl Drop for LlamaRuntimeState {
    fn drop(&mut self) {
        if let Ok(mut child_guard) = self.child.lock() {
            if let Some(child) = child_guard.as_mut() {
                let _ = child.kill();
            }
        }
    }
}

pub async fn send_chat(
    app: &AppHandle,
    runtime: &LlamaRuntimeState,
    settings: AssistantSettings,
    history: Vec<ChatMessage>,
    memories: Vec<MemoryEntry>,
    task_state: TaskState,
    content: String,
    mode: String,
) -> Result<LocalAiOutput, String> {
    let fast_result = LocalRuleProvider::send(&content, &mode);
    if settings.local_ai_provider == "rules" {
        return Ok(fast_result);
    }

    if !fast_result.actions.is_empty() {
        return Ok(fast_result);
    }

    LlamaCppProvider::ensure_ready(app, runtime, &settings).await?;
    let result = LlamaCppProvider::send(app, &settings, history, memories, task_state, content).await?;
    validate_output(result)
}

pub async fn status(
    app: &AppHandle,
    runtime: &LlamaRuntimeState,
    settings: &AssistantSettings,
    start_if_possible: bool,
) -> LocalAiStatus {
    let resources = match LlamaResources::resolve(app, settings) {
        Ok(resources) => resources,
        Err(error) => {
            return LocalAiStatus {
                ready: false,
                provider: "qwen_embedded".to_string(),
                model: MODEL_LABEL.to_string(),
                state: "model_missing".to_string(),
                error: Some(error),
            };
        }
    };

    if !resources.server_path.exists() {
        return LocalAiStatus {
            ready: false,
            provider: "qwen_embedded".to_string(),
            model: MODEL_LABEL.to_string(),
            state: "runtime_missing".to_string(),
            error: Some(format!(
                "llama-server.exe nao encontrado. Rode npm run prepare:llama para preparar o runtime."
            )),
        };
    }

    if server_ready().await {
        runtime.set_error(None);
        return LocalAiStatus {
            ready: true,
            provider: "qwen_embedded".to_string(),
            model: MODEL_LABEL.to_string(),
            state: "ready".to_string(),
            error: None,
        };
    }

    if start_if_possible {
        if let Err(error) = LlamaCppProvider::ensure_started_with_resources(runtime, &resources) {
            runtime.set_error(Some(error));
        }
    }

    let state = if process_running(runtime) {
        "loading"
    } else {
        "stopped"
    };

    LocalAiStatus {
        ready: false,
        provider: "qwen_embedded".to_string(),
        model: MODEL_LABEL.to_string(),
        state: state.to_string(),
        error: runtime.last_error(),
    }
}

struct LlamaResources {
    server_path: PathBuf,
    model_path: PathBuf,
    prompt_path: Option<PathBuf>,
}

impl LlamaResources {
    fn resolve(app: &AppHandle, settings: &AssistantSettings) -> Result<Self, String> {
        let configured_model = settings
            .local_ai_path
            .as_ref()
            .map(PathBuf::from)
            .filter(|path| path.exists());
        let model_path = configured_model
        .or_else(|| find_existing_path(app, &[
            &["models", MODEL_NAME][..],
            &["resources", "models", MODEL_NAME][..],
            &["src-tauri", "resources", "models", MODEL_NAME][..],
        ]))
        .or_else(|| {
            let fallback = PathBuf::from(r"C:\Users\ezile\Downloads\ia\Qwen3-8B-Q5_0.gguf");
            fallback.exists().then_some(fallback)
        })
        .ok_or_else(|| {
            format!(
                "Modelo {MODEL_NAME} nao encontrado no bundle nem em C:\\Users\\ezile\\Downloads\\ia."
            )
        })?;

        let server_path = find_existing_path(app, &[
            &["bin", "llama-server.exe"][..],
            &["resources", "bin", "llama-server.exe"][..],
            &["llama-server-x86_64-pc-windows-msvc.exe"][..],
            &["binaries", "llama-server-x86_64-pc-windows-msvc.exe"][..],
            &["src-tauri", "binaries", "llama-server-x86_64-pc-windows-msvc.exe"][..],
            &["src-tauri", "resources", "bin", "llama-server.exe"][..],
        ])
        .unwrap_or_else(|| PathBuf::from("llama-server-x86_64-pc-windows-msvc.exe"));

        let prompt_path = find_existing_path(app, &[
            &["prompts", "system_prompt.md"][..],
            &["resources", "prompts", "system_prompt.md"][..],
            &["src-tauri", "resources", "prompts", "system_prompt.md"][..],
        ])
        .or_else(|| {
            let fallback = PathBuf::from(r"C:\Users\ezile\Downloads\ia\assistant_jarvis\prompts\system_prompt.md");
            fallback.exists().then_some(fallback)
        });

        Ok(Self {
            server_path,
            model_path,
            prompt_path,
        })
    }
}

struct LlamaCppProvider;

impl LlamaCppProvider {
    async fn ensure_ready(
        app: &AppHandle,
        runtime: &LlamaRuntimeState,
        settings: &AssistantSettings,
    ) -> Result<(), String> {
        let resources = LlamaResources::resolve(app, settings)?;
        Self::ensure_started_with_resources(runtime, &resources)?;

        let deadline = std::time::Instant::now() + Duration::from_secs(SERVER_READY_TIMEOUT_SECS);
        while std::time::Instant::now() < deadline {
            if server_ready().await {
                runtime.set_error(None);
                return Ok(());
            }
            sleep(Duration::from_millis(700)).await;
        }

        let error = "Tempo esgotado carregando o Qwen local. Confira RAM disponivel e arquivos do runtime.".to_string();
        runtime.set_error(Some(error.clone()));
        Err(error)
    }

    fn ensure_started_with_resources(
        runtime: &LlamaRuntimeState,
        resources: &LlamaResources,
    ) -> Result<(), String> {
        if server_ready_blocking() {
            runtime.set_error(None);
            return Ok(());
        }

        {
            let mut child_guard = runtime.child.lock().map_err(|err| err.to_string())?;
            if let Some(child) = child_guard.as_mut() {
                match child.try_wait().map_err(|err| err.to_string())? {
                    None => return Ok(()),
                    Some(_) => {
                        *child_guard = None;
                    }
                }
            }

            if !resources.server_path.exists() {
                return Err(format!(
                    "llama-server.exe nao encontrado em {}.",
                    resources.server_path.display()
                ));
            }
            if !resources.model_path.exists() {
                return Err(format!("Modelo GGUF nao encontrado em {}.", resources.model_path.display()));
            }

            let threads = std::thread::available_parallelism()
                .map(|count| count.get().saturating_sub(1).max(1))
                .unwrap_or(4)
                .to_string();

            let model_path = resources.model_path.to_string_lossy().to_string();
            let port = SERVER_PORT.to_string();
            let child = Command::new(&resources.server_path)
                .args([
                    "-m",
                    &model_path,
                    "--host",
                    SERVER_HOST,
                    "--port",
                    &port,
                    "-c",
                    "2048",
                    "--parallel",
                    "1",
                    "--cache-ram",
                    "0",
                    "--no-warmup",
                    "--reasoning",
                    "off",
                    "--timeout",
                    "900",
                    "--n-gpu-layers",
                    "0",
                    "--threads",
                    &threads,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|err| format!("Falha ao iniciar llama-server: {err}"))?;
            *child_guard = Some(child);
        }

        runtime.set_error(None);
        Ok(())
    }

    async fn send(
        app: &AppHandle,
        settings: &AssistantSettings,
        history: Vec<ChatMessage>,
        memories: Vec<MemoryEntry>,
        task_state: TaskState,
        content: String,
    ) -> Result<LocalAiOutput, String> {
        let resources = LlamaResources::resolve(app, settings)?;
        let prompt = build_completion_prompt(&resources, history, memories, task_state, content);
        let body = json!({
            "prompt": prompt,
            "temperature": 0.1,
            "top_p": 0.85,
            "n_predict": 180,
            "stop": ["<|im_end|>"]
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(360))
            .build()
            .map_err(|err| err.to_string())?;
        let response = client
            .post(format!("http://{SERVER_HOST}:{SERVER_PORT}/completion"))
            .json(&body)
            .send()
            .await
            .map_err(|err| format!("Falha ao falar com llama-server: {err}"))?;

        let status = response.status();
        let text = response.text().await.map_err(|err| err.to_string())?;
        if !status.is_success() {
            return Err(format!("llama-server retornou erro {status}: {text}"));
        }

        let value: Value = serde_json::from_str(&text)
            .map_err(|err| format!("Resposta HTTP do llama-server nao e JSON: {err}"))?;
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or("llama-server nao retornou content.")?;

        parse_model_output(content)
    }
}

struct LocalRuleProvider;

impl LocalRuleProvider {
    fn send(content: &str, mode: &str) -> LocalAiOutput {
        let mut actions = Vec::new();
        let lower = normalize(content);

        if lower.contains("desliga") || lower.contains("desligar") {
            actions.push(action(
                "power",
                "Desligar o PC",
                "shutdown",
                json!({ "operation": "shutdown" }),
                ActionRisk::High,
                true,
                "Desligar o computador pode interromper trabalhos abertos.",
            ));
        } else if lower.contains("reinicia") || lower.contains("reiniciar") {
            actions.push(action(
                "power",
                "Reiniciar o PC",
                "restart",
                json!({ "operation": "restart" }),
                ActionRisk::High,
                true,
                "Reiniciar o computador fecha aplicativos abertos.",
            ));
        }

        if lower.contains("print") || lower.contains("screenshot") || lower.contains("captura de tela") {
            actions.push(action(
                "take_screenshot",
                "Tirar captura de tela",
                "screen",
                json!({}),
                ActionRisk::Low,
                false,
                "Captura de tela solicitada.",
            ));
        }

        if lower.contains("copie") || lower.contains("copiar") || lower.contains("copia ") {
            let text = extract_after(content, &["copie", "copiar", "copia"]).unwrap_or(content);
            actions.push(action(
                "copy_text",
                "Copiar texto",
                "clipboard",
                json!({ "text": text.trim() }),
                ActionRisk::Low,
                false,
                "Copiar texto para a area de transferencia.",
            ));
        }

        if is_memory_save_request(&lower) {
            let memory = extract_memory_text(content).unwrap_or(content).trim().to_string();
            if !memory.is_empty() {
                let category = if contains_music_terms(&memory.to_lowercase()) {
                    "musica"
                } else {
                    "preferencia"
                };
                actions.push(action(
                    "save_memory",
                    "Salvar memoria",
                    "memory",
                    json!({
                        "title": summarize_title(&memory),
                        "content": memory,
                        "category": category
                    }),
                    ActionRisk::Low,
                    false,
                    "Guardar informacao para a IA usar depois.",
                ));
            }
        }

        if let Some((name, query)) = extract_music_favorite(content, &lower) {
            actions.push(action(
                "save_music_favorite",
                "Salvar favorito musical",
                "music_favorite",
                json!({
                    "name": name,
                    "query": query,
                    "kind": "playlist"
                }),
                ActionRisk::Low,
                false,
                "Guardar atalho musical para usar em comandos futuros.",
            ));
        }

        if lower.contains("volume") {
            let level = find_percent(&lower).unwrap_or(50);
            actions.push(action(
                "system_volume",
                "Ajustar volume",
                "volume",
                json!({ "level": level }),
                ActionRisk::Low,
                false,
                "Ajuste de volume do Windows.",
            ));
        }

        if lower.contains("brilho") {
            let level = find_percent(&lower).unwrap_or(50);
            actions.push(action(
                "brightness",
                "Ajustar brilho",
                "brightness",
                json!({ "level": level }),
                ActionRisk::Low,
                false,
                "Ajuste de brilho do monitor.",
            ));
        }

        if lower.contains("listar processos") || lower.contains("ver processos") || lower.contains("processos abertos") {
            actions.push(action(
                "list_processes",
                "Listar processos",
                "processes",
                json!({}),
                ActionRisk::Low,
                false,
                "Listar processos em execucao.",
            ));
        }

        if lower.contains("youtube") {
            let query = extract_after(content, &["youtube", "procura", "pesquisa", "buscar"]).unwrap_or("");
            let url = if query.trim().is_empty() {
                "https://www.youtube.com".to_string()
            } else {
                format!(
                    "https://www.youtube.com/results?search_query={}",
                    url_encode(query.trim())
                )
            };
            actions.push(action(
                "open_url",
                "Abrir YouTube",
                "youtube",
                json!({ "url": url }),
                ActionRisk::Low,
                false,
                "Abrir YouTube no navegador.",
            ));
        } else if lower.contains("google") || lower.contains("pesquisa") || lower.contains("pesquisar") {
            let query = extract_after(content, &["google", "pesquisa", "pesquisar", "procura"]).unwrap_or("");
            let url = if query.trim().is_empty() {
                "https://www.google.com".to_string()
            } else {
                format!("https://www.google.com/search?q={}", url_encode(query.trim()))
            };
            actions.push(action(
                "open_url",
                "Abrir Google",
                "google",
                json!({ "url": url }),
                ActionRisk::Low,
                false,
                "Abrir pesquisa no navegador.",
            ));
        }

        if let Some(app_name) = detect_app(&lower) {
            actions.push(action(
                "open_app",
                &format!("Abrir {app_name}"),
                app_name,
                json!({ "app": app_name }),
                ActionRisk::Low,
                false,
                "Abrir aplicativo instalado.",
            ));
        }

        if lower.contains("pausa")
            || lower.contains("pause")
            || lower.contains("parar musica")
            || lower.contains("para a musica")
        {
            actions.push(action(
                "pause_music",
                "Pausar musica",
                "spotify",
                json!({}),
                ActionRisk::Low,
                false,
                "Pausar Spotify.",
            ));
        } else if lower.contains("proxima musica")
            || lower.contains("proxima faixa")
            || lower.contains("pular musica")
            || lower.contains("passa a musica")
        {
            actions.push(action(
                "next_track",
                "Proxima musica",
                "spotify",
                json!({}),
                ActionRisk::Low,
                false,
                "Avancar Spotify.",
            ));
        } else if lower.contains("musica anterior") || lower.contains("volta a musica") {
            actions.push(action(
                "previous_track",
                "Musica anterior",
                "spotify",
                json!({}),
                ActionRisk::Low,
                false,
                "Voltar faixa no Spotify.",
            ));
        }

        if lower.contains("curtidas")
            || lower.contains("musicas favoritas")
            || lower.contains("minhas favoritas")
            || lower.contains("liked songs")
        {
            actions.push(action(
                "play_liked_music",
                "Tocar musicas curtidas",
                "spotify",
                json!({}),
                ActionRisk::Low,
                false,
                "Tocar musicas salvas/curtidas no Spotify.",
            ));
        } else if lower.contains("musica")
            || lower.contains("playlist")
            || lower.contains("toca ")
            || lower.contains("toque ")
            || (lower.contains("spotify") && (lower.contains("tocar") || lower.contains("toca") || lower.contains("toque")))
        {
            let query = extract_after(content, &["toca", "toque", "playlist", "spotify"]).unwrap_or(content);
            actions.push(action(
                "play_music",
                "Tocar no Spotify",
                "spotify",
                json!({ "query": query.trim() }),
                ActionRisk::Low,
                false,
                "Controle de musica solicitado.",
            ));
        }

        let reply = if actions.is_empty() {
            if mode == "comando" {
                "Nao encontrei uma acao automatica para esse comando ainda. Posso responder e manter isso no historico.".to_string()
            } else {
                "Entendi. O Qwen local e o caminho principal; este modo de regras fica so como fallback de desenvolvimento.".to_string()
            }
        } else {
            "Entendi o comando e preparei as acoes locais permitidas.".to_string()
        };

        LocalAiOutput {
            assistant_reply: reply,
            actions,
        }
    }
}

fn build_system_prompt(resources: &LlamaResources) -> String {
    let base = resources
        .prompt_path
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(default_system_prompt);

    format!(
        "{base}\n\n{}\n\n{}",
        tool_catalog(),
        "Responda somente com JSON valido, sem markdown, sem cercas de codigo. Formato exato: {\"assistant_reply\":\"texto curto\",\"actions\":[{\"type\":\"open_app\",\"label\":\"Abrir Chrome\",\"target\":\"chrome\",\"args\":{\"app\":\"chrome\"},\"risk\":\"low\",\"requires_confirmation\":false,\"reason\":\"Abrir aplicativo\"}]}. Use actions vazio quando so responder. Nunca diga que uma acao foi executada; o app executa depois."
    )
}

fn build_completion_prompt(
    resources: &LlamaResources,
    history: Vec<ChatMessage>,
    memories: Vec<MemoryEntry>,
    task_state: TaskState,
    content: String,
) -> String {
    let mut prompt = format!("<|im_start|>system\n{}<|im_end|>\n", build_system_prompt(resources));
    if !memories.is_empty() {
        prompt.push_str("<|im_start|>system\nMemoria persistente do usuario:\n");
        for memory in memories.into_iter().take(24) {
            prompt.push_str(&format!(
                "- [{}] {}: {}\n",
                memory.category, memory.title, memory.content
            ));
        }
        prompt.push_str("Use essa memoria para entender preferencias e escolher melhores acoes. Se o usuario pedir para lembrar algo, use save_memory.\n<|im_end|>\n");
    }
    if task_state.objective.is_some() || !task_state.completed_steps.is_empty() || task_state.next_step.is_some() {
        prompt.push_str("<|im_start|>system\nEstado atual da tarefa:\n");
        if let Some(objective) = task_state.objective {
            prompt.push_str(&format!("- objetivo: {objective}\n"));
        }
        if !task_state.completed_steps.is_empty() {
            prompt.push_str(&format!(
                "- passos concluidos: {}\n",
                task_state.completed_steps.join("; ")
            ));
        }
        if let Some(next_step) = task_state.next_step {
            prompt.push_str(&format!("- proximo passo: {next_step}\n"));
        }
        prompt.push_str("Use esse estado para continuar a mesma tarefa sem se perder.\n<|im_end|>\n");
    }
    for message in history.into_iter().rev().take(6).collect::<Vec<_>>().into_iter().rev() {
        if message.role == "user" || message.role == "assistant" {
            prompt.push_str(&format!(
                "<|im_start|>{}\n{}<|im_end|>\n",
                message.role,
                message.content
            ));
        }
    }
    prompt.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", content));
    prompt
}

fn default_system_prompt() -> String {
    "Voce e PC Control AI, um assistente local em portugues do Brasil para entender comandos do usuario e controlar o PC por ferramentas validadas.".to_string()
}

fn tool_catalog() -> &'static str {
    r#"Ferramentas permitidas:
- open_app: abrir aplicativo. args: {"app":"chrome|msedge|notepad|calc|explorer|spotify|code"} risk low.
- close_app: fechar app comum. args: {"app":"chrome|msedge|notepad|spotify|code"} risk medium.
- open_url: abrir site http/https. args: {"url":"https://..."} risk low.
- copy_text: copiar texto. args: {"text":"..."} risk low.
- system_volume: ajustar volume Windows. args: {"level":0-100} risk low.
- brightness: ajustar brilho do monitor quando suportado. args: {"level":0-100} risk low.
- take_screenshot: capturar tela. args: {} risk low.
- list_processes: listar processos. args: {} risk low.
- close_process: encerrar processo por pid conhecido. args: {"pid":1234} risk high se critico.
- power: desligar/reiniciar. args: {"operation":"shutdown|restart"} risk high requires_confirmation true.
- play_music: tocar musica ou playlist. args: {"query":"..."} risk low.
- play_liked_music: tocar musicas curtidas/salvas do Spotify. args: {} risk low.
- pause_music, next_track, previous_track: controle de reproducao. args: {} risk low.
- spotify_volume: volume Spotify quando conectado.
- save_memory: salvar algo importante para lembrar depois. args: {"title":"curto","content":"texto completo","category":"preferencia|musica|app|rotina|pessoal"} risk low.
- save_music_favorite: salvar apelido musical. args: {"name":"foco","query":"lo-fi beats","kind":"playlist|track"} risk low.
Regras: comandos perigosos devem ter risk high e requires_confirmation true. Nao use shell, cmd, powershell nem comandos arbitrarios."#
}

fn parse_model_output(content: &str) -> Result<LocalAiOutput, String> {
    let trimmed = content.trim();
    let json_text = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    serde_json::from_str::<LocalAiOutput>(json_text).map_err(|err| {
        format!(
            "O Qwen local respondeu fora do JSON esperado; nenhuma acao foi executada. Erro: {err}"
        )
    })
}

fn validate_output(mut output: LocalAiOutput) -> Result<LocalAiOutput, String> {
    if output.assistant_reply.trim().is_empty() {
        output.assistant_reply = "Entendi.".to_string();
    }

    for action in &output.actions {
        validate_action(action)?;
    }

    Ok(output)
}

fn validate_action(action: &ActionSpec) -> Result<(), String> {
    if action.label.trim().is_empty() {
        return Err("O modelo retornou uma acao sem label; nada foi executado.".to_string());
    }

    let allowed = [
        "open_app",
        "close_app",
        "open_url",
        "open_path",
        "copy_text",
        "system_volume",
        "brightness",
        "take_screenshot",
        "list_processes",
        "close_process",
        "power",
        "window_minimize",
        "window_maximize",
        "window_focus",
        "play_music",
        "play_liked_music",
        "pause_music",
        "next_track",
        "previous_track",
        "spotify_play",
        "spotify_pause",
        "spotify_next",
        "spotify_previous",
        "spotify_volume",
        "spotify_liked",
        "run_routine",
        "save_memory",
        "save_music_favorite",
    ];

    if !allowed.contains(&action.action_type.as_str()) {
        return Err(format!(
            "O modelo retornou tipo de acao nao permitido: {}. Nada foi executado.",
            action.action_type
        ));
    }

    Ok(())
}

fn extract_music_favorite(content: &str, lower: &str) -> Option<(String, String)> {
    if !lower.contains("playlist") && !lower.contains("musica") {
        return None;
    }
    for separator in [" é ", " e ", " = ", " chama ", " seja "] {
        if let Some(index) = lower.find(separator) {
            let left = content[..index].trim();
            let right = content[index + separator.len()..].trim();
            let name = left
                .split_whitespace()
                .rev()
                .take_while(|part| !matches!(part.to_lowercase().as_str(), "playlist" | "musica" | "minha" | "meu" | "a" | "o" | "que"))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" ");
            if !name.is_empty() && !right.is_empty() {
                return Some((name, right.to_string()));
            }
        }
    }
    None
}

fn contains_music_terms(value: &str) -> bool {
    ["playlist", "musica", "spotify", "lo-fi", "lofi", "album", "faixa"]
        .iter()
        .any(|needle| value.contains(needle))
}

async fn server_ready() -> bool {
    Client::new()
        .get(format!("http://{SERVER_HOST}:{SERVER_PORT}/health"))
        .timeout(Duration::from_millis(800))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn server_ready_blocking() -> bool {
    reqwest::blocking::Client::new()
        .get(format!("http://{SERVER_HOST}:{SERVER_PORT}/health"))
        .timeout(Duration::from_millis(500))
        .send()
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn process_running(runtime: &LlamaRuntimeState) -> bool {
    let Ok(mut child_guard) = runtime.child.lock() else {
        return false;
    };
    if let Some(child) = child_guard.as_mut() {
        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => {
                *child_guard = None;
                false
            }
            Err(_) => false,
        }
    } else {
        false
    }
}

fn find_existing_path(app: &AppHandle, relative_candidates: &[&[&str]]) -> Option<PathBuf> {
    let mut bases = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        bases.push(resource_dir);
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            bases.push(parent.to_path_buf());
        }
    }
    if let Ok(current_dir) = std::env::current_dir() {
        bases.push(current_dir.clone());
        bases.push(current_dir.join("assistant-desktop"));
    }

    for base in bases {
        for parts in relative_candidates {
            let candidate = parts.iter().fold(base.clone(), |path, part| path.join(part));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn action(
    action_type: &str,
    label: &str,
    target: &str,
    args: serde_json::Value,
    risk: ActionRisk,
    requires_confirmation: bool,
    reason: &str,
) -> ActionSpec {
    ActionSpec {
        action_type: action_type.to_string(),
        label: label.to_string(),
        target: target.to_string(),
        args,
        risk,
        requires_confirmation,
        reason: reason.to_string(),
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn extract_after<'a>(content: &'a str, needles: &[&str]) -> Option<&'a str> {
    let lower = content.to_lowercase();
    needles.iter().find_map(|needle| {
        lower
            .find(needle)
            .map(|index| {
                content[index + needle.len()..]
                    .trim_matches(|ch| matches!(ch, ' ' | ':' | '-' | '"' | '\''))
            })
            .filter(|value| !value.is_empty())
    })
}

fn find_percent(value: &str) -> Option<u8> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u8>().ok())
        .find(|number| *number <= 100)
}

fn detect_app(value: &str) -> Option<&'static str> {
    let wants_open = value.contains("abre")
        || value.contains("abrir")
        || value.contains("inicia")
        || value.contains("executa");
    if !wants_open {
        return None;
    }

    [
        ("chrome", "chrome"),
        ("edge", "msedge"),
        ("notepad", "notepad"),
        ("bloco de notas", "notepad"),
        ("calculadora", "calc"),
        ("calculator", "calc"),
        ("explorer", "explorer"),
        ("arquivos", "explorer"),
        ("spotify", "spotify"),
        ("vscode", "code"),
        ("vs code", "code"),
    ]
    .iter()
    .find_map(|(needle, command)| value.contains(needle).then_some(*command))
}

fn is_memory_save_request(value: &str) -> bool {
    value.contains("lembre que")
        || value.contains("lembra que")
        || value.contains("memoriza")
        || value.contains("salve na memoria")
        || value.contains("salva na memoria")
        || value.contains("guardar na memoria")
        || value.contains("guarde na memoria")
}

fn extract_memory_text<'a>(content: &'a str) -> Option<&'a str> {
    extract_after(
        content,
        &[
            "lembre que",
            "lembra que",
            "memoriza",
            "salve na memoria",
            "salva na memoria",
            "guardar na memoria",
            "guarde na memoria",
        ],
    )
}

fn summarize_title(value: &str) -> String {
    let mut title = value
        .split_whitespace()
        .take(7)
        .collect::<Vec<_>>()
        .join(" ");
    if title.chars().count() > 64 {
        title = title.chars().take(64).collect();
    }
    if title.is_empty() {
        "Memoria".to_string()
    } else {
        title
    }
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect::<Vec<_>>(),
        })
        .collect()
}
