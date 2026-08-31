$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot

function Invoke-NativeChecked {
    param(
        [string]$Label,
        [scriptblock]$Command
    )

    & $Command
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "$Label failed with exit code $exitCode."
    }
}

try {
    & "$PSScriptRoot\validate_content.ps1"
    if (-not $?) { throw "Content validation failed." }
    Invoke-NativeChecked "cargo fmt" { cargo fmt --all -- --check }
    Invoke-NativeChecked "cargo test" { cargo test --workspace }
    Invoke-NativeChecked "cargo clippy" { cargo clippy --workspace --all-targets --all-features -- -D warnings }
    & "$projectRoot\publish.ps1"
    if (-not $?) { throw "Publishing failed." }
    & "$projectRoot\scripts\write_release_manifest.ps1"
    if (-not $?) { throw "Release manifest generation failed." }
    Write-Host "Tarrowyn release gate passed."
} finally {
    Pop-Location
}
