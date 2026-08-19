$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    cargo test -p tarrowyn-server phase5 -- --nocapture
    cargo test --workspace
    Write-Host "Regional travel, event, market, household, and identity fixture passed."
} finally {
    Pop-Location
}
