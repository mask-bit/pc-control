@echo off
setlocal

set "ROOT=%~dp0"
set "ISCC="

where iscc >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    for /f "delims=" %%I in ('where iscc') do (
        set "ISCC=%%I"
        goto :found_iscc
    )
)

if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" (
    set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
    goto :found_iscc
)

if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" (
    set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
    goto :found_iscc
)

echo ERROR: Inno Setup compiler (ISCC.exe) not found.
echo Install Inno Setup 6 from https://jrsoftware.org/isinfo.php and run this script again.
exit /b 1

:found_iscc
echo === Building PC Control installer ===
echo ISCC: %ISCC%
echo.

call "%ROOT%build.bat"
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo [Installer] Building dist\PCControlSetup.exe...
"%ISCC%" "%ROOT%installer\PCControl.iss"
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
echo DONE
echo   %ROOT%dist\PCControlSetup.exe
exit /b 0
