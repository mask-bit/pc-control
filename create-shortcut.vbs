Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")

scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)
rootExe = fso.BuildPath(scriptDir, "WorkspaceLauncher.exe")
distExe = fso.BuildPath(fso.BuildPath(scriptDir, "dist"), "WorkspaceLauncher.exe")
workspacePy = fso.BuildPath(scriptDir, "workspace.py")
shortcutPath = shell.ExpandEnvironmentStrings("%APPDATA%") & "\Microsoft\Windows\Start Menu\Programs\Startup\WorkspaceLauncher.lnk"

Function FindCommand(name)
    On Error Resume Next
    Set exec = shell.Exec("cmd /c where " & name)
    output = exec.StdOut.ReadAll
    If Err.Number <> 0 Then
        FindCommand = ""
        Err.Clear
    Else
        lines = Split(output, vbCrLf)
        FindCommand = Trim(lines(0))
    End If
    On Error GoTo 0
End Function

target = ""
arguments = ""
workingDir = scriptDir

If fso.FileExists(rootExe) Then
    target = rootExe
ElseIf fso.FileExists(distExe) Then
    target = distExe
    workingDir = fso.GetParentFolderName(distExe)
ElseIf fso.FileExists(workspacePy) Then
    target = FindCommand("pythonw.exe")
    If target = "" Then target = FindCommand("python.exe")
    If target = "" Then
        WScript.Echo "Python not found in PATH."
        WScript.Quit 1
    End If
    arguments = """" & workspacePy & """"
Else
    WScript.Echo "WorkspaceLauncher.exe or workspace.py not found."
    WScript.Quit 1
End If

Set s = shell.CreateShortcut(shortcutPath)
s.TargetPath = target
s.Arguments = arguments
s.WorkingDirectory = workingDir
s.Description = "Workspace Launcher"
s.WindowStyle = 7
s.Save
WScript.Echo "Startup shortcut created!"
