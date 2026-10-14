@echo off
PowerShell -NoProfile -ExecutionPolicy Bypass -File "%~dp0station-control.ps1"
if errorlevel 1 pause
