param(
    [string]$ServerTarget
)

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
    Invoke-NativeChecked "client cargo fmt" { cargo fmt --package years_of_tarrowyn -- --check }
    Invoke-NativeChecked "protocol cargo fmt" { cargo fmt --manifest-path protocol\Cargo.toml -- --check }
    Invoke-NativeChecked "server cargo fmt" { cargo fmt --manifest-path server\Cargo.toml -- --check }
    Invoke-NativeChecked "cargo test" { cargo test --workspace }
    Invoke-NativeChecked "cargo clippy" { cargo clippy --workspace --all-targets --all-features -- -D warnings }
    & "$projectRoot\publish.ps1"
    if (-not $?) { throw "Publishing failed." }
    $packageArguments = @()
    if ($PSBoundParameters.ContainsKey('ServerTarget')) {
        $packageArguments += @('-Target', $ServerTarget)
    }
    & "$projectRoot\scripts\package_server_release.ps1" @packageArguments
    if (-not $?) { throw "Server release packaging failed." }
    & "$projectRoot\scripts\verify_server_release.ps1" -ArchivePath 'dist\tarrowyn_server.zip'
    if (-not $?) { throw "Packaged server launch verification failed." }
    & "$projectRoot\scripts\write_release_manifest.ps1"
    if (-not $?) { throw "Release manifest generation failed." }
    Write-Host "Tarrowyn release gate passed."
} finally {
    Pop-Location
}
