$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Venv = Join-Path $Root ".venv"
$Python = Get-Command python -ErrorAction SilentlyContinue

if (-not $Python) {
    throw "Python nao foi encontrado. Instale Python 3.10+ e marque 'Add python.exe to PATH'."
}

Write-Host "[1/5] Criando ambiente Python..."
if (-not (Test-Path $Venv)) {
    & python -m venv $Venv
}

$VenvPython = Join-Path $Venv "Scripts\python.exe"
$VenvPythonw = Join-Path $Venv "Scripts\pythonw.exe"
if (-not (Test-Path $VenvPython)) {
    throw "Ambiente .venv invalido: $VenvPython nao existe."
}

Write-Host "[2/5] Atualizando pip..."
& $VenvPython -m pip install --upgrade pip

Write-Host "[3/5] Instalando dependencias..."
& $VenvPython -m pip install -r (Join-Path $Root "requirements.txt")

Write-Host "[4/5] Preparando pastas do PC Control..."
& $VenvPython -c "from app_paths import ensure_app_dirs, ensure_user_config; ensure_app_dirs(); ensure_user_config(); print('AppData OK')"

Write-Host "[5/5] Criando atalhos..."
$ShortcutTarget = if (Test-Path $VenvPythonw) { $VenvPythonw } else { $VenvPython }
$Assistant = Join-Path $Root "assistant_panel.py"
$Icon = Join-Path $Root "assets\pc-control.ico"
$Desktop = [Environment]::GetFolderPath("Desktop")
$Programs = [Environment]::GetFolderPath("Programs")
$StartMenuShortcut = Join-Path $Programs "PC Control.lnk"
$DesktopShortcut = Join-Path $Desktop "PC Control.lnk"

function New-PCControlShortcut {
    param(
        [string]$Path
    )
    $Shell = New-Object -ComObject WScript.Shell
    $Shortcut = $Shell.CreateShortcut($Path)
    $Shortcut.TargetPath = $ShortcutTarget
    $Shortcut.Arguments = "`"$Assistant`""
    $Shortcut.WorkingDirectory = $Root
    if (Test-Path $Icon) {
        $Shortcut.IconLocation = $Icon
    }
    $Shortcut.Description = "PC Control"
    $Shortcut.Save()
}

New-PCControlShortcut -Path $DesktopShortcut
New-PCControlShortcut -Path $StartMenuShortcut

Write-Host ""
Write-Host "PC Control instalado com sucesso."
Write-Host "Atalho Desktop: $DesktopShortcut"
Write-Host "Atalho Menu Iniciar: $StartMenuShortcut"
