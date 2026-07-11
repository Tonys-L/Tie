# ====== Tie - Dev Mode (Fast) ======
# Usage: .\dev.ps1

$appName = "tie"

Write-Host ""
Write-Host "========== Tie - Dev Mode ==========" -ForegroundColor Cyan

# 1. Kill old process
Write-Host ""
Write-Host "[1/2] Stopping old process..." -ForegroundColor Yellow
$procs = Get-Process -Name $appName -ErrorAction SilentlyContinue
if ($procs) {
    $procs | Stop-Process -Force
    Write-Host "  Stopped $($procs.Count) process(es)" -ForegroundColor Green
    Start-Sleep -Seconds 1
} else {
    Write-Host "  No running process" -ForegroundColor Gray
}

# 2. Run tauri dev
Write-Host ""
Write-Host "[2/2] Starting tauri dev..." -ForegroundColor Yellow
Write-Host "  First run: ~3 min (compiles debug binaries)"
Write-Host "  After that: ~10-30s (incremental)"
Write-Host ""

Set-Location $PSScriptRoot
# Start-Process with -NoNewWindow streams output directly to console
$proc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "npm run tauri dev" -WorkingDirectory $PSScriptRoot -NoNewWindow -PassThru
$proc.WaitForExit()
