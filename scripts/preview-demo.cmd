@echo off
setlocal
cd /d "%~dp0.."
echo.
echo Rest Reminder Pet - Demo Preview
echo   1) Cat jumps up from bottom
echo   2) After ~3s: snow + cat climbs down
echo   3) Stay still ~30s continuously to succeed; any input resets idle timer
echo   Tray dismiss to end; Ctrl+C to exit
echo.
set REST_REMINDER_DEMO=1
cargo run --release
endlocal
