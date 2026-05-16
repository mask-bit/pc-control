use crate::models::{InstalledApp, PcControlState, ProcessInfo, WindowInfo};
use arboard::Clipboard;
use chrono::Utc;
use serde::Deserialize;
use std::process::Command;
use sysinfo::System;
use tauri::{AppHandle, Manager};

pub fn get_pc_state(volume: u8) -> PcControlState {
    PcControlState {
        volume: get_system_volume().unwrap_or(volume),
        brightness: get_brightness().unwrap_or_default(),
        clipboard_preview: clipboard_preview(),
        processes: list_processes(),
        windows: list_windows(),
    }
}

pub fn list_processes() -> Vec<ProcessInfo> {
    let mut system = System::new_all();
    system.refresh_all();
    let mut processes = system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let name = process.name().to_string_lossy().to_string();
            ProcessInfo {
                pid: pid.to_string().parse::<u32>().unwrap_or_default(),
                exe: process.exe().map(|path| path.display().to_string()),
                cpu: process.cpu_usage(),
                memory: process.memory(),
                critical: is_critical_process(&name),
                name,
            }
        })
        .collect::<Vec<_>>();
    processes.sort_by(|a, b| b.memory.cmp(&a.memory));
    processes.truncate(80);
    processes
}

pub fn close_process(pid: u32) -> Result<bool, String> {
    let process = list_processes()
        .into_iter()
        .find(|process| process.pid == pid)
        .ok_or("Processo nao encontrado.")?;

    if process.critical {
        return Err("Processo critico bloqueado por seguranca.".to_string());
    }

    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn list_installed_apps() -> Vec<InstalledApp> {
    detected_apps().unwrap_or_default()
}

pub fn open_app(target: &str) -> Result<bool, String> {
    let resolved = resolve_app_target(target);
    if resolved.starts_with("shell:") || resolved.contains(':') {
        open::that(&resolved).map_err(|err| err.to_string())?;
    } else {
        Command::new("cmd")
            .args(["/C", "start", "", &resolved])
            .spawn()
            .map_err(|err| err.to_string())?;
    }
    Ok(true)
}

pub fn spotify_installed() -> bool {
    detected_apps()
        .map(|apps| {
            apps.into_iter()
                .any(|app| normalize_name(&app.name).contains("spotify"))
        })
        .unwrap_or(false)
}

pub fn open_spotify_target(target: Option<&str>) -> Result<bool, String> {
    let uri = target.unwrap_or("spotify:");
    open::that(uri).map_err(|err| err.to_string())?;
    Ok(true)
}

pub fn focus_spotify_window() -> Result<bool, String> {
    let window = list_windows()
        .into_iter()
        .find(|window| window.app.eq_ignore_ascii_case("spotify"))
        .ok_or("Janela do Spotify nao encontrada.".to_string())?;
    focus_window(&window.id)
}

pub fn send_media_command(command: &str) -> Result<bool, String> {
    let app_command = match command {
        "play_pause" => 14,
        "next" => 11,
        "previous" => 12,
        _ => return Err("Comando de midia desconhecido.".to_string()),
    };
    let script = format!(
        r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class MediaKeys {{
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, int Msg, IntPtr wParam, IntPtr lParam);
}}
"@
$hwndBroadcast = [IntPtr]0xffff
$wmAppCommand = 0x319
$lParam = [IntPtr]({app_command} -shl 16)
[MediaKeys]::SendMessage($hwndBroadcast, $wmAppCommand, [IntPtr]::Zero, $lParam) | Out-Null
"#
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn close_app(target: &str) -> Result<bool, String> {
    let exe = normalize_process_name(target);
    let output = Command::new("taskkill")
        .args(["/IM", &exe, "/F"])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn set_system_volume(level: u8) -> Result<bool, String> {
    let level = level.min(100);
    let script = format!(
        r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

[Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioEndpointVolume {{
  int RegisterControlChangeNotify(IntPtr pNotify);
  int UnregisterControlChangeNotify(IntPtr pNotify);
  int GetChannelCount(out uint channelCount);
  int SetMasterVolumeLevel(float levelDB, Guid eventContext);
  int SetMasterVolumeLevelScalar(float level, Guid eventContext);
  int GetMasterVolumeLevel(out float levelDB);
  int GetMasterVolumeLevelScalar(out float level);
  int SetChannelVolumeLevel(uint channelNumber, float levelDB, Guid eventContext);
  int SetChannelVolumeLevelScalar(uint channelNumber, float level, Guid eventContext);
  int GetChannelVolumeLevel(uint channelNumber, out float levelDB);
  int GetChannelVolumeLevelScalar(uint channelNumber, out float level);
  int SetMute(bool isMuted, Guid eventContext);
  int GetMute(out bool isMuted);
  int GetVolumeStepInfo(out uint step, out uint stepCount);
  int VolumeStepUp(Guid eventContext);
  int VolumeStepDown(Guid eventContext);
  int QueryHardwareSupport(out uint hardwareSupportMask);
  int GetVolumeRange(out float volumeMinDB, out float volumeMaxDB, out float volumeIncrementDB);
}}

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice {{
  int Activate(ref Guid iid, int clsCtx, IntPtr activationParams, out IAudioEndpointVolume endpointVolume);
}}

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator {{
  int NotImpl1();
  int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice device);
}}

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
class MMDeviceEnumeratorComObject {{}}

public static class Audio {{
  public static void SetVolume(float level) {{
    var enumerator = (IMMDeviceEnumerator)(new MMDeviceEnumeratorComObject());
    IMMDevice device;
    Marshal.ThrowExceptionForHR(enumerator.GetDefaultAudioEndpoint(0, 1, out device));
    var iid = typeof(IAudioEndpointVolume).GUID;
    IAudioEndpointVolume endpoint;
    Marshal.ThrowExceptionForHR(device.Activate(ref iid, 23, IntPtr.Zero, out endpoint));
    Marshal.ThrowExceptionForHR(endpoint.SetMasterVolumeLevelScalar(level, Guid.Empty));
  }}
}}
"@
[Audio]::SetVolume({level}.0 / 100.0)
"#
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn get_system_volume() -> Result<u8, String> {
    let script = r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

[Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioEndpointVolume {
  int RegisterControlChangeNotify(IntPtr pNotify);
  int UnregisterControlChangeNotify(IntPtr pNotify);
  int GetChannelCount(out uint channelCount);
  int SetMasterVolumeLevel(float levelDB, Guid eventContext);
  int SetMasterVolumeLevelScalar(float level, Guid eventContext);
  int GetMasterVolumeLevel(out float levelDB);
  int GetMasterVolumeLevelScalar(out float level);
  int SetChannelVolumeLevel(uint channelNumber, float levelDB, Guid eventContext);
  int SetChannelVolumeLevelScalar(uint channelNumber, float level, Guid eventContext);
  int GetChannelVolumeLevel(uint channelNumber, out float levelDB);
  int GetChannelVolumeLevelScalar(uint channelNumber, out float level);
  int SetMute(bool isMuted, Guid eventContext);
  int GetMute(out bool isMuted);
  int GetVolumeStepInfo(out uint step, out uint stepCount);
  int VolumeStepUp(Guid eventContext);
  int VolumeStepDown(Guid eventContext);
  int QueryHardwareSupport(out uint hardwareSupportMask);
  int GetVolumeRange(out float volumeMinDB, out float volumeMaxDB, out float volumeIncrementDB);
}

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice {
  int Activate(ref Guid iid, int clsCtx, IntPtr activationParams, out IAudioEndpointVolume endpointVolume);
}

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator {
  int NotImpl1();
  int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice device);
}

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
class MMDeviceEnumeratorComObject {}

public static class Audio {
  public static float GetVolume() {
    var enumerator = (IMMDeviceEnumerator)(new MMDeviceEnumeratorComObject());
    IMMDevice device;
    Marshal.ThrowExceptionForHR(enumerator.GetDefaultAudioEndpoint(0, 1, out device));
    var iid = typeof(IAudioEndpointVolume).GUID;
    IAudioEndpointVolume endpoint;
    Marshal.ThrowExceptionForHR(device.Activate(ref iid, 23, IntPtr.Zero, out endpoint));
    float level;
    Marshal.ThrowExceptionForHR(endpoint.GetMasterVolumeLevelScalar(out level));
    return level;
  }
}
"@
[Math]::Round([Audio]::GetVolume() * 100)
"#;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    raw.trim()
        .parse::<u8>()
        .map_err(|err| format!("Volume invalido retornado pelo Windows: {err}"))
}

pub fn brightness_supported() -> bool {
    get_brightness().is_ok()
}

pub fn get_brightness() -> Result<u8, String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction Stop | Select-Object -First 1 -ExpandProperty CurrentBrightness)",
        ])
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u8>()
        .map_err(|err| format!("Brilho invalido retornado pelo Windows: {err}"))
}

pub fn set_brightness(level: u8) -> Result<bool, String> {
    let level = level.min(100);
    let script = format!(
        "$monitor = Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop | Select-Object -First 1; \
         if (-not $monitor) {{ throw 'Monitor sem suporte a brilho por WMI.' }}; \
         Invoke-CimMethod -InputObject $monitor -MethodName WmiSetBrightness -Arguments @{{Timeout=1; Brightness={level}}} | Out-Null"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn take_screenshot(app: &AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?
        .join("screenshots");
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let file_name = format!("screenshot-{}.png", Utc::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(file_name);
    let escaped_path = path.display().to_string().replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -AssemblyName System.Drawing; \
         $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \
         $bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height; \
         $graphics = [System.Drawing.Graphics]::FromImage($bitmap); \
         $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size); \
         $bitmap.Save('{escaped_path}', [System.Drawing.Imaging.ImageFormat]::Png); \
         $graphics.Dispose(); $bitmap.Dispose();"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(path.display().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn power(operation: &str) -> Result<bool, String> {
    let args = match operation {
        "restart" => ["/r", "/t", "0"],
        "shutdown" => ["/s", "/t", "0"],
        _ => return Err("Operacao de energia desconhecida.".to_string()),
    };
    Command::new("shutdown")
        .args(args)
        .spawn()
        .map_err(|err| err.to_string())?;
    Ok(true)
}

pub fn set_start_with_windows(enabled: bool) -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let exe = exe.display().to_string().replace('\'', "''");
    let script = if enabled {
        format!(
            "New-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'PC Control AI' -Value '\"{exe}\"' -PropertyType String -Force | Out-Null"
        )
    } else {
        "Remove-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'PC Control AI' -ErrorAction SilentlyContinue"
            .to_string()
    };
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn speak_text(text: &str) -> Result<bool, String> {
    let escaped = text.replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Speech; \
         $voice = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $voice.Speak('{escaped}')"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn window_control_supported() -> bool {
    !list_windows().is_empty()
}

pub fn list_windows() -> Vec<WindowInfo> {
    let script = r#"
$foreground = @'
using System;
using System.Runtime.InteropServices;
public static class NativeWindow {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
'@
Add-Type $foreground -ErrorAction SilentlyContinue
$active = [NativeWindow]::GetForegroundWindow().ToInt64()
Get-Process |
  Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle } |
  Select-Object @{Name='id';Expression={$_.MainWindowHandle.ToInt64().ToString()}},
                @{Name='title';Expression={$_.MainWindowTitle}},
                @{Name='app';Expression={$_.ProcessName}},
                @{Name='focused';Expression={$_.MainWindowHandle.ToInt64() -eq $active}} |
  ConvertTo-Json -Compress
"#;
    run_json_script::<DetectedWindow>(script)
        .unwrap_or_default()
        .into_iter()
        .map(|window| WindowInfo {
            id: window.id,
            title: window.title,
            app: window.app,
            focused: window.focused,
        })
        .collect()
}

pub fn focus_window(id: &str) -> Result<bool, String> {
    run_window_command(id, "focus")
}

pub fn minimize_window(id: &str) -> Result<bool, String> {
    run_window_command(id, "minimize")
}

pub fn maximize_window(id: &str) -> Result<bool, String> {
    run_window_command(id, "maximize")
}

fn clipboard_preview() -> String {
    Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map(|text| {
            let mut preview = text.chars().take(120).collect::<String>();
            if text.chars().count() > 120 {
                preview.push_str("...");
            }
            preview
        })
        .unwrap_or_default()
}

#[derive(Debug, Deserialize)]
struct DetectedApp {
    id: String,
    name: String,
    command: String,
}

#[derive(Debug, Deserialize)]
struct DetectedWindow {
    id: String,
    title: String,
    app: String,
    focused: bool,
}

fn detected_apps() -> Result<Vec<InstalledApp>, String> {
    let script = r#"
$apps = @()
Get-StartApps -ErrorAction SilentlyContinue | ForEach-Object {
  if ($_.Name -and $_.AppID) {
    $apps += [PSCustomObject]@{
      id = ($_.AppID -replace '[^a-zA-Z0-9._-]', '_')
      name = $_.Name
      command = ('shell:AppsFolder\' + $_.AppID)
    }
  }
}
$roots = @(
  'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\*',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\*'
)
foreach ($root in $roots) {
  Get-ItemProperty -Path $root -ErrorAction SilentlyContinue | ForEach-Object {
    $exe = $_.'(default)'
    if (-not $exe) { $exe = $_.PSChildName }
    if ($exe) {
      $apps += [PSCustomObject]@{
        id = ($_.PSChildName -replace '[^a-zA-Z0-9._-]', '_')
        name = [System.IO.Path]::GetFileNameWithoutExtension($_.PSChildName)
        command = $exe
      }
    }
  }
}
$apps |
  Where-Object { $_.name -and $_.command } |
  Sort-Object name, command -Unique |
  ConvertTo-Json -Compress
"#;
    let mut apps = run_json_script::<DetectedApp>(script)?
        .into_iter()
        .map(|detected| {
            let normalized = normalize_name(&detected.name);
            InstalledApp {
                id: detected.id,
                favorite: is_favorite_app(&normalized),
                recent: is_recent_app(&normalized),
                name: detected.name,
                command: detected.command,
            }
        })
        .collect::<Vec<_>>();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}

fn normalize_name(value: &str) -> String {
    value.trim().to_lowercase()
}

fn resolve_app_target(target: &str) -> String {
    let normalized = normalize_name(target);
    detected_apps()
        .ok()
        .and_then(|apps| {
            apps.into_iter().find(|app| {
                let name = normalize_name(&app.name);
                let id = normalize_name(&app.id);
                name == normalized
                    || id == normalized
                    || name.contains(&normalized)
                    || normalized.contains(&name)
            })
        })
        .map(|app| app.command)
        .unwrap_or_else(|| target.to_string())
}

fn is_favorite_app(value: &str) -> bool {
    ["chrome", "spotify", "visual studio code", "arquivos", "explorer"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn is_recent_app(value: &str) -> bool {
    ["chrome", "edge", "spotify", "arquivos", "explorer"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn run_window_command(id: &str, command: &str) -> Result<bool, String> {
    let script = format!(
        r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class NativeWindow {{
  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
}}
"@
$handle = [IntPtr]::new([Int64]{id})
switch ('{command}') {{
  'focus' {{ [NativeWindow]::SetForegroundWindow($handle) | Out-Null }}
  'minimize' {{ [NativeWindow]::ShowWindowAsync($handle, 6) | Out-Null }}
  'maximize' {{ [NativeWindow]::ShowWindowAsync($handle, 3) | Out-Null; [NativeWindow]::SetForegroundWindow($handle) | Out-Null }}
  default {{ throw 'Comando de janela desconhecido.' }}
}}
"#
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(true)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn run_json_script<T>(script: &str) -> Result<Vec<T>, String>
where
    T: for<'de> Deserialize<'de>,
{
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    if text.starts_with('[') {
        serde_json::from_str(&text).map_err(|err| err.to_string())
    } else {
        serde_json::from_str::<T>(&text)
            .map(|item| vec![item])
            .map_err(|err| err.to_string())
    }
}

fn normalize_process_name(value: &str) -> String {
    let lower = value.to_lowercase();
    let mapped = match lower.as_str() {
        "chrome" => "chrome.exe",
        "edge" | "msedge" => "msedge.exe",
        "spotify" => "Spotify.exe",
        "notepad" | "bloco de notas" => "notepad.exe",
        "calc" | "calculadora" => "CalculatorApp.exe",
        "code" | "vscode" | "vs code" => "Code.exe",
        other => other,
    };
    if mapped.ends_with(".exe") {
        mapped.to_string()
    } else {
        format!("{mapped}.exe")
    }
}

fn is_critical_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    [
        "system",
        "registry",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "winlogon.exe",
        "services.exe",
        "lsass.exe",
        "svchost.exe",
        "dwm.exe",
        "explorer.exe",
    ]
    .iter()
    .any(|item| lower == *item)
}
