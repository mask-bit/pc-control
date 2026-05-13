@echo off
cd /d "%~dp0"
if exist ".venv\Scripts\pythonw.exe" (
    start "" ".venv\Scripts\pythonw.exe" "%~dp0assistant_panel.py"
    exit /b 0
)
if exist ".venv\Scripts\python.exe" (
    ".venv\Scripts\python.exe" "%~dp0assistant_panel.py"
    exit /b %ERRORLEVEL%
)
python assistant_panel.py
