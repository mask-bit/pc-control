; Workspace Launcher - AutoHotkey v2 shortcuts
; The PowerShell wrapper calls the Python launcher, which owns all profile logic.

^!w:: {
    Run('powershell.exe -ExecutionPolicy Bypass -File "' A_ScriptDir '\workspace-launcher.ps1"')
}

^!c:: {
    Run('powershell.exe -ExecutionPolicy Bypass -File "' A_ScriptDir '\workspace-launcher.ps1" -Config')
}

^!m:: {
    Run('powershell.exe -ExecutionPolicy Bypass -File "' A_ScriptDir '\workspace-launcher.ps1" -ListMonitors')
}
