@echo off
setlocal
cd /d "%~dp0.."
echo.
echo Rest Reminder Pet - Demo Preview
echo   1) Cat jumps up from bottom
echo   2) After ~3s: snow + cat climbs down
echo   3) Stay still ~30s to climb away; move input to fall
echo   Tray dismiss to end; Ctrl+C to exit
echo.
set REST_REMINDER_DEMO=1
call npm run tauri -- dev
endlocal
