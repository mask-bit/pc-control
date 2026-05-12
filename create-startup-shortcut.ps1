$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WshShell = New-Object -ComObject WScript.Shell
$ShortcutPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\WorkspaceLauncher.lnk"

$RootExe = Join-Path $ScriptDir "WorkspaceLauncher.exe"
$DistExe = Join-Path $ScriptDir "dist\WorkspaceLauncher.exe"
$WorkspacePy = Join-Path $ScriptDir "workspace.py"

$Target = $null
$Arguments = ""
$WorkingDirectory = $ScriptDir

if (Test-Path $RootExe) {
    $Target = $RootExe
} elseif (Test-Path $DistExe) {
    $Target = $DistExe
    $WorkingDirectory = Split-Path -Parent $DistExe
} elseif (Test-Path $WorkspacePy) {
    $Pythonw = (Get-Command pythonw.exe -ErrorAction SilentlyContinue).Source
    if (-not $Pythonw) {
        $Pythonw = (Get-Command python.exe -ErrorAction SilentlyContinue).Source
    }
    if (-not $Pythonw) {
        Write-Host "ERROR: pythonw.exe/python.exe not found in PATH." -ForegroundColor Red
        exit 1
    }
    $Target = $Pythonw
    $Arguments = "`"$WorkspacePy`""
} else {
    Write-Host "ERROR: WorkspaceLauncher.exe or workspace.py not found." -ForegroundColor Red
    exit 1
}

$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = $Target
$Shortcut.Arguments = $Arguments
$Shortcut.WorkingDirectory = $WorkingDirectory
$Shortcut.Description = "Workspace Launcher"
$Shortcut.WindowStyle = 7
$Shortcut.Save()
Write-Host "Shortcut created: $ShortcutPath" -ForegroundColor Green
