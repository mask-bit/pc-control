@echo off
setlocal
cd /d "%~dp0"

echo === PC Control - instalacao simples ===
echo.
echo Este instalador prepara o app em modo fonte:
echo - cria .venv
echo - instala dependencias
echo - cria atalhos no Desktop e Menu Iniciar
echo.

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-pc-control.ps1"
set "RESULT=%ERRORLEVEL%"
echo.
if "%RESULT%"=="0" (
    echo Instalacao concluida. Use o atalho "PC Control" no Desktop ou Menu Iniciar.
) else (
    echo A instalacao falhou. Veja a mensagem acima.
)
echo.
pause
exit /b %RESULT%
