# Starts SiteMonitor on Windows: the portable PostgreSQL (port 54329) and the
# app (http://localhost:8737), both in the background.
# Usage:  powershell -ExecutionPolicy Bypass -File scripts\start-windows.ps1
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$pg = if ($env:SITEMONITOR_PG_BIN) { $env:SITEMONITOR_PG_BIN } else { "$env:USERPROFILE\tools\pgsql\bin" }
$data = if ($env:SITEMONITOR_PG_DATA) { $env:SITEMONITOR_PG_DATA } else { "$env:USERPROFILE\tools\sitemonitor-pgdata" }

& "$pg\pg_ctl.exe" -D $data status *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Starting PostgreSQL on port 54329..."
    Start-Process -FilePath "$pg\pg_ctl.exe" -ArgumentList @('-D', "`"$data`"", '-l', "`"$data\..\sitemonitor-pg.log`"", 'start') -WindowStyle Hidden
    Start-Sleep -Seconds 4
}

$exe = Join-Path $root "target\release\backend.exe"
if (-not (Test-Path $exe)) {
    Write-Host "Building first (this takes a few minutes)..."
    node (Join-Path $root "scripts\build.mjs")
}
New-Item -ItemType Directory -Force (Join-Path $root "data") | Out-Null
Start-Process -FilePath $exe -WorkingDirectory $root -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $root "data\backend.log") `
    -RedirectStandardError (Join-Path $root "data\backend.err.log")
Write-Host "SiteMonitor is starting at http://localhost:8737"
