@echo off
setlocal

echo === Building Workspace Launcher / Jarvis Assistant ===
echo.

where python >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Python not found in PATH.
    exit /b 1
)

set "PYTHON=python"
set "ROOT=%~dp0"
set "DIST=%ROOT%dist"
set "BUILD=%ROOT%build"

echo [1/5] Installing dependencies...
"%PYTHON%" -m pip install -r "%ROOT%requirements.txt" pyinstaller
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [2/5] Building WorkspaceLauncher.exe...
"%PYTHON%" -m PyInstaller ^
    --onefile ^
    --name WorkspaceLauncher ^
    --console ^
    --distpath "%DIST%" ^
    --workpath "%BUILD%" ^
    --specpath "%BUILD%" ^
    --hidden-import numpy ^
    --hidden-import sounddevice ^
    --hidden-import pystray ^
    --hidden-import PIL ^
    --hidden-import panns_inference ^
    --hidden-import vosk ^
    "%ROOT%workspace.py"
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [3/5] Building WorkspaceConfig.exe...
"%PYTHON%" -m PyInstaller ^
    --onefile ^
    --name WorkspaceConfig ^
    --windowed ^
    --distpath "%DIST%" ^
    --workpath "%BUILD%" ^
    --specpath "%BUILD%" ^
    --hidden-import customtkinter ^
    "%ROOT%config_gui.py"
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [4/5] Building JarvisAssistant.exe...
"%PYTHON%" -m PyInstaller ^
    --onefile ^
    --name JarvisAssistant ^
    --windowed ^
    --distpath "%DIST%" ^
    --workpath "%BUILD%" ^
    --specpath "%BUILD%" ^
    --hidden-import customtkinter ^
    --hidden-import sounddevice ^
    --hidden-import vosk ^
    --hidden-import keyboard ^
    --hidden-import pystray ^
    --hidden-import PIL ^
    --hidden-import openai_controller ^
    --hidden-import spotify_controller ^
    --hidden-import secrets_store ^
    "%ROOT%assistant_panel.py"
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [5/5] Copying configuration...
if exist "%ROOT%workspace-config.json" (
    copy /Y "%ROOT%workspace-config.json" "%DIST%\workspace-config.json" >nul
) else (
    copy /Y "%ROOT%workspace-config.example.json" "%DIST%\workspace-config.json" >nul
)
if exist "%ROOT%assistant_config.json" (
    copy /Y "%ROOT%assistant_config.json" "%DIST%\assistant_config.json" >nul
)

echo.
echo DONE
echo   %DIST%\WorkspaceLauncher.exe
echo   %DIST%\WorkspaceConfig.exe
echo   %DIST%\JarvisAssistant.exe
echo   %DIST%\workspace-config.json
echo   %DIST%\assistant_config.json
exit /b 0
