#define MyAppName "PC Control"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "PC Control"
#define MyAppExeName "PCControl.exe"

[Setup]
AppId={{A54F3D0B-C5B8-4A3A-9C8E-CA7A1B63DA7B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\PC Control
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=PCControlSetup
SetupIconFile=..\assets\pc-control.ico
Compression=lzma
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "Criar atalho na Area de Trabalho"; GroupDescription: "Atalhos:"; Flags: unchecked
Name: "startup"; Description: "Iniciar o PC Control com o Windows"; GroupDescription: "Inicializacao:"; Flags: unchecked

[Files]
Source: "..\dist\PCControl.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\assistant_config.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\PC Control"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\pc-control.ico"
Name: "{autodesktop}\PC Control"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\pc-control.ico"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "PC Control"; ValueData: """{app}\{#MyAppExeName}"""; Tasks: startup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Abrir PC Control"; Flags: nowait postinstall skipifsilent
