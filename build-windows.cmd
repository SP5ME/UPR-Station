@echo off
setlocal
chcp 65001 >nul
PowerShell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-windows.ps1" %*
if errorlevel 1 (
    echo.
    echo Budowanie nie powiodlo sie. Zamknij aplikacje i uruchom skrypt ponownie.
    pause
    exit /b 1
)
echo.
pause
