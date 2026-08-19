$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    & "$PSScriptRoot\validate_content.ps1"
    cargo fmt --all -- --check
    cargo test --workspace
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    & "$projectRoot\publish.ps1"
    Write-Host "Tarrowyn release gate passed."
} finally {
    Pop-Location
}
