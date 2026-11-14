@echo off
REM CANdata quick packaging script
echo ========================================
echo CANdata packaging tool
echo ========================================
echo.

REM Call the PowerShell script
powershell -ExecutionPolicy Bypass -File "%~dp0package.ps1" -Version "1.1.0"

echo.
echo Press any key to exit...
pause >nul
