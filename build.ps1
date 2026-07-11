# ====== Tie - Build & Launch ======
# Usage:  .\build.ps1            (build + launch)
#         .\build.ps1 -SkipBuild (launch only)

param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$appName = "tie"
$exePath = Join-Path $PSScriptRoot "src-tauri\target\release\$appName.exe"

Write-Host ""
Write-Host "========== Tie ==========" -ForegroundColor Cyan

# 1. Kill old process
Write-Host ""
Write-Host "[1/4] Stopping old process..." -ForegroundColor Yellow
$procs = Get-Process -Name $appName -ErrorAction SilentlyContinue
if ($procs) {
    $procs | Stop-Process -Force
    Write-Host "  Stopped $($procs.Count) process(es)" -ForegroundColor Green
    Start-Sleep -Seconds 2
} else {
    Write-Host "  No running process" -ForegroundColor Gray
}

# 2. Delete old exe
Write-Host ""
Write-Host "[2/4] Cleaning old build..." -ForegroundColor Yellow
if (Test-Path $exePath) {
    Remove-Item $exePath -Force -ErrorAction SilentlyContinue
    Write-Host "  Deleted old exe" -ForegroundColor Green
}

# 3. Build
if (-not $SkipBuild) {
    Write-Host ""
    Write-Host "[3/4] Building (may take 8-15 min)..." -ForegroundColor Yellow
    Set-Location $PSScriptRoot
    $savedEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & npx tauri build 2>&1 | ForEach-Object {
        $line = $_.ToString()
        if ($line -match "error|Error|Finished|Compiling|Built|Bundle|NSIS|MSI") {
            Write-Host "  $line" -ForegroundColor White
        }
    }
    $ErrorActionPreference = $savedEAP
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "  BUILD FAILED! Exit code: $LASTEXITCODE" -ForegroundColor Red
        Write-Host "  Run manually: npx tauri build" -ForegroundColor Gray
        exit 1
    }
    Write-Host "  Build OK" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "[3/4] Skipped (-SkipBuild)" -ForegroundColor Gray
}

# 4. Launch
Write-Host ""
Write-Host "[4/4] Launching..." -ForegroundColor Yellow
if (Test-Path $exePath) {
    Start-Process -FilePath $exePath
    Write-Host "  Started: $exePath" -ForegroundColor Green
} else {
    Write-Host "  exe not found, build first" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "========== Done ==========" -ForegroundColor Cyan
Write-Host "Tip: launch only -> .\build.ps1 -SkipBuild"
Write-Host ""
