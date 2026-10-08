# Preview demo: boot jump, then auto rest reminder (snow + cat)
# Usage from project root:
#   powershell -ExecutionPolicy Bypass -File .\scripts\preview-demo.ps1
#   npm run demo

$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

Write-Host ""
Write-Host "Rest Reminder Pet - Demo Preview" -ForegroundColor Cyan
Write-Host "  1) Cat jumps up from bottom (boot intro)" -ForegroundColor Gray
Write-Host "  2) After ~3s: snow on all screens + cat climbs down" -ForegroundColor Gray
Write-Host "  3) Stay still ~30s to climb away; move mouse/keyboard to fall" -ForegroundColor Gray
Write-Host "  Tray: click Know / dismiss to end preview" -ForegroundColor Gray
Write-Host "  Ctrl+C to exit" -ForegroundColor Gray
Write-Host ""

$env:REST_REMINDER_DEMO = "1"
npm run tauri -- dev