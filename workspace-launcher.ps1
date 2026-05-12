# Workspace Launcher wrapper.
# The Python launcher is the single source of truth; this script only locates it.

param(
    [string]$Profile = "",
    [switch]$ListMonitors,
    [switch]$Config
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

$RootExe = Join-Path $ScriptDir "WorkspaceLauncher.exe"
$DistExe = Join-Path $ScriptDir "dist\WorkspaceLauncher.exe"
$WorkspacePy = Join-Path $ScriptDir "workspace.py"

$LauncherArgs = @()
if ($ListMonitors) {
    $LauncherArgs += "--list-monitors"
} elseif ($Config) {
    $LauncherArgs += "--config"
} else {
    $LauncherArgs += "--launch"
    if ($Profile) {
        $LauncherArgs += $Profile
    }
}

if (Test-Path $RootExe) {
    & $RootExe @LauncherArgs
    exit $LASTEXITCODE
}

if (Test-Path $DistExe) {
    & $DistExe @LauncherArgs
    exit $LASTEXITCODE
}

if (-not (Test-Path $WorkspacePy)) {
    Write-Host "ERROR: workspace.py not found in $ScriptDir" -ForegroundColor Red
    exit 1
}

$Python = Get-Command python -ErrorAction SilentlyContinue
if (-not $Python) {
    $Python = Get-Command py -ErrorAction SilentlyContinue
}
if (-not $Python) {
    Write-Host "ERROR: Python not found in PATH." -ForegroundColor Red
    exit 1
}

& $Python.Source $WorkspacePy @LauncherArgs
exit $LASTEXITCODE
