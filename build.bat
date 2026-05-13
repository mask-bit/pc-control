@echo off
setlocal

echo === Building PC Control ===
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
set "ASSETS=%ROOT%assets"
set "ICON=%ASSETS%\pc-control.ico"

if not exist "%ICON%" (
    echo ERROR: Missing icon: %ICON%
    exit /b 1
)

echo [1/4] Installing dependencies...
"%PYTHON%" -m pip install -r "%ROOT%requirements.txt" pyinstaller
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [2/4] Cleaning previous PC Control build...
if not exist "%DIST%" mkdir "%DIST%"
if not exist "%BUILD%" mkdir "%BUILD%"
if exist "%DIST%\PCControl.exe" del /Q "%DIST%\PCControl.exe"
if exist "%DIST%\PCControlSetup.exe" del /Q "%DIST%\PCControlSetup.exe"
if exist "%DIST%\JarvisAssistant.exe" del /Q "%DIST%\JarvisAssistant.exe"
if exist "%DIST%\WorkspaceLauncher.exe" del /Q "%DIST%\WorkspaceLauncher.exe"
if exist "%DIST%\WorkspaceConfig.exe" del /Q "%DIST%\WorkspaceConfig.exe"
if exist "%DIST%\workspace-config.json" del /Q "%DIST%\workspace-config.json"
if exist "%DIST%\assets" rmdir /S /Q "%DIST%\assets"

echo.
echo [3/4] Building PCControl.exe...
"%PYTHON%" -m PyInstaller ^
    --onefile ^
    --name PCControl ^
    --windowed ^
    --icon "%ICON%" ^
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
    --hidden-import app_paths ^
    --hidden-import auth_google ^
    "%ROOT%assistant_panel.py"
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [4/4] Copying app defaults and assets...
copy /Y "%ROOT%assistant_config.json" "%DIST%\assistant_config.json" >nul
xcopy /E /I /Y "%ASSETS%" "%DIST%\assets" >nul

echo.
echo DONE
echo   %DIST%\PCControl.exe
echo   %DIST%\assistant_config.json
echo   %DIST%\assets\
exit /b 0
