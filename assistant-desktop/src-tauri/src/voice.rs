use crate::models::{AssistantSettings, VoiceState};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct VoiceRuntimeState {
    current: Arc<Mutex<VoiceState>>,
}

impl VoiceRuntimeState {
    pub fn new(settings: &AssistantSettings) -> Self {
        Self {
            current: Arc::new(Mutex::new(VoiceState {
                state: "idle".to_string(),
                wake_word: settings.wake_word.clone(),
                engine_ready: false,
                microphone_ready: false,
                transcript: None,
                error: None,
            })),
        }
    }

    fn update(&self, next: VoiceState) -> VoiceState {
        if let Ok(mut current) = self.current.lock() {
            *current = next.clone();
        }
        next
    }

    pub fn get(&self) -> VoiceState {
        self.current.lock().map(|state| state.clone()).unwrap_or(VoiceState {
            state: "error".to_string(),
            wake_word: "assistente".to_string(),
            engine_ready: false,
            microphone_ready: false,
            transcript: None,
            error: Some("Falha ao ler estado de voz.".to_string()),
        })
    }
}

pub fn get_state(
    app: &AppHandle,
    runtime: &VoiceRuntimeState,
    settings: &AssistantSettings,
) -> VoiceState {
    let assets = VoiceAssets::resolve(app);
    let current = runtime.get();
    let ready = assets.ready() || windows_speech_available();
    let next_state = if !ready && current.state != "error" {
        "idle".to_string()
    } else {
        current.state
    };
    VoiceState {
        state: next_state,
        wake_word: settings.wake_word.clone(),
        engine_ready: ready,
        microphone_ready: settings.voice_enabled,
        transcript: current.transcript,
        error: if ready {
            current.error
        } else {
            Some(assets.missing_message())
        },
    }
}

pub fn start_listening(
    app: &AppHandle,
    runtime: &VoiceRuntimeState,
    settings: &AssistantSettings,
) -> Result<VoiceState, String> {
    let assets = VoiceAssets::resolve(app);
    let can_capture = assets.ready() || windows_speech_available();
    if !can_capture {
        let state = VoiceState {
            state: "error".to_string(),
            wake_word: settings.wake_word.clone(),
            engine_ready: false,
            microphone_ready: settings.voice_enabled,
            transcript: None,
            error: Some(assets.missing_message()),
        };
        runtime.update(state.clone());
        return Err(state.error.unwrap_or_else(|| "Motor de voz local indisponivel.".to_string()));
    }

    let wake_word = settings.wake_word.clone();
    let runtime_for_capture = runtime.clone();
    runtime.update(VoiceState {
        state: "waiting_wake_word".to_string(),
        wake_word: wake_word.clone(),
        engine_ready: true,
        microphone_ready: true,
        transcript: None,
        error: None,
    });

    std::thread::spawn(move || {
        runtime_for_capture.update(VoiceState {
            state: "listening".to_string(),
            wake_word: wake_word.clone(),
            engine_ready: true,
            microphone_ready: true,
            transcript: None,
            error: None,
        });
        runtime_for_capture.update(VoiceState {
            state: "transcribing".to_string(),
            wake_word: wake_word.clone(),
            engine_ready: true,
            microphone_ready: true,
            transcript: None,
            error: None,
        });

        let next = match capture_once_with_windows_speech() {
            Ok(transcript) => VoiceState {
                state: "idle".to_string(),
                wake_word,
                engine_ready: true,
                microphone_ready: true,
                transcript: Some(transcript),
                error: None,
            },
            Err(error) => VoiceState {
                state: "error".to_string(),
                wake_word,
                engine_ready: true,
                microphone_ready: true,
                transcript: None,
                error: Some(error),
            },
        };
        runtime_for_capture.update(next);
    });

    Ok(runtime.get())
}

pub fn stop_listening(runtime: &VoiceRuntimeState, settings: &AssistantSettings) -> VoiceState {
    runtime.update(VoiceState {
        state: "idle".to_string(),
        wake_word: settings.wake_word.clone(),
        engine_ready: runtime.get().engine_ready,
        microphone_ready: settings.voice_enabled,
        transcript: None,
        error: None,
    })
}

struct VoiceAssets {
    whisper_binary: Option<PathBuf>,
    whisper_model: Option<PathBuf>,
    kws_binary: Option<PathBuf>,
    kws_model: Option<PathBuf>,
}

impl VoiceAssets {
    fn resolve(app: &AppHandle) -> Self {
        Self {
            whisper_binary: find_existing_path(
                app,
                &[
                    &["voice", "whisper-server.exe"][..],
                    &["resources", "voice", "whisper-server.exe"][..],
                    &["src-tauri", "resources", "voice", "whisper-server.exe"][..],
                ],
            ),
            whisper_model: find_existing_path(
                app,
                &[
                    &["voice", "ggml-base.bin"][..],
                    &["resources", "voice", "ggml-base.bin"][..],
                    &["src-tauri", "resources", "voice", "ggml-base.bin"][..],
                ],
            ),
            kws_binary: find_existing_path(
                app,
                &[
                    &["voice", "sherpa-onnx-kws.exe"][..],
                    &["resources", "voice", "sherpa-onnx-kws.exe"][..],
                    &["src-tauri", "resources", "voice", "sherpa-onnx-kws.exe"][..],
                ],
            ),
            kws_model: find_existing_path(
                app,
                &[
                    &["voice", "kws-model.onnx"][..],
                    &["resources", "voice", "kws-model.onnx"][..],
                    &["src-tauri", "resources", "voice", "kws-model.onnx"][..],
                ],
            ),
        }
    }

    fn ready(&self) -> bool {
        self.whisper_binary.is_some()
            && self.whisper_model.is_some()
            && self.kws_binary.is_some()
            && self.kws_model.is_some()
    }

    fn missing_message(&self) -> String {
        if windows_speech_available() {
            "Motor de voz do Windows disponivel como fallback local.".to_string()
        } else {
            "Motor de voz local ainda nao esta instalado: faltam binarios/modelos de whisper.cpp e sherpa-onnx."
                .to_string()
        }
    }
}

fn windows_speech_available() -> bool {
    cfg!(target_os = "windows")
}

fn capture_once_with_windows_speech() -> Result<String, String> {
    if !windows_speech_available() {
        return Err("Captura de voz local disponivel apenas no Windows nesta V1.".to_string());
    }

    let script = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
try {
  $culture = [System.Globalization.CultureInfo]::GetCultureInfo('pt-BR')
  $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine($culture)
} catch {
  $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine
}
$grammar = New-Object System.Speech.Recognition.DictationGrammar
$recognizer.LoadGrammar($grammar)
$recognizer.SetInputToDefaultAudioDevice()
$result = $recognizer.Recognize([TimeSpan]::FromSeconds(8))
if ($result -and $result.Text) {
  [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
  Write-Output $result.Text
}
$recognizer.Dispose()
"#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|err| format!("Falha ao iniciar captura de voz: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Falha no motor de voz do Windows.".to_string()
        } else {
            stderr
        });
    }

    let transcript = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if transcript.is_empty() {
        Err("Nenhuma fala foi detectada no intervalo de escuta.".to_string())
    } else {
        Ok(transcript)
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
