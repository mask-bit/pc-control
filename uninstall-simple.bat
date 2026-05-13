@echo off
setlocal
cd /d "%~dp0"

echo === PC Control - remover atalhos simples ===
echo.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0uninstall-simple.ps1"
echo.
pause
