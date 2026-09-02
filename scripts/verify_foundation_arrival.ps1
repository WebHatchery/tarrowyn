$ErrorActionPreference = "Stop"

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) { throw $message }
}

function Get-DescendantProcessIds([int]$parentId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId")
    foreach ($child in $children) {
        Get-DescendantProcessIds $child.ProcessId
        $child.ProcessId
    }
}

function Wait-ForHealth([System.Diagnostics.Process]$process) {
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        if ($process.HasExited) { throw "The authoritative server exited before becoming healthy." }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The authoritative server did not become healthy in time."
}

function Start-FixtureServer([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process
    return $process
}

function Stop-FixtureServer([System.Diagnostics.Process]$process) {
    if ($null -ne $process -and -not $process.HasExited) {
        $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
        foreach ($processId in $processIds) {
            Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
        }
    }
}

function New-Guest([string]$clientKey) {
    $response = Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/session/guest" `
        -ContentType "application/json" `
        -Body (@{ client_key = $clientKey; reset = $false } | ConvertTo-Json -Compress)
    return $response.data
}

function Read-State([string]$token) {
    return Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/state" `
        -Headers @{ Authorization = "Bearer $token" }
}

function Step-North([string]$token, [string]$requestId) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/movement" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; dx = 0; dy = -1 } | ConvertTo-Json -Compress)
}

function Use-FoundationInteraction([string]$token, [string]$requestId, [string]$interactionId) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/interactions" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; interaction_id = $interactionId } | ConvertTo-Json -Compress)
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f1-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-True ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "The F1 temporary path escaped the system temp directory."
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8870"
$oldAddress = $env:TARROWYN_SERVER_ADDR
$oldStatePath = $env:TARROWYN_STATE_PATH
$oldBackupPath = $env:TARROWYN_BACKUP_PATH
$oldDbDriver = $env:DB_DRIVER
$env:TARROWYN_SERVER_ADDR = "127.0.0.1:8870"
$env:DB_DRIVER = "json"
Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
$server = $null

try {
    Push-Location $projectRoot
    cargo test --workspace foundation
    if ($LASTEXITCODE -ne 0) { throw "Focused F1 Rust tests failed." }

    $statePath = Join-Path $temporaryRoot "arrival-state.json"
    $server = Start-FixtureServer $statePath (Join-Path $temporaryRoot "arrival.out.log") (Join-Path $temporaryRoot "arrival.err.log")
    $keys = @("f1-arrival-one", "f1-arrival-two", "f1-arrival-three")
    $guests = @($keys | ForEach-Object { New-Guest $_ })

    foreach ($guest in $guests) {
        $state = Read-State $guest.account_token
        Assert-True (@($state.data.world.players).Count -eq 3) "A connected client did not see all three arrivals."
        Assert-True ($state.data.world.foundation.fixture_id -eq "first-beacon-baseline-v1") "A connected client did not receive the First Beacon fixture."
        Assert-True ($state.data.player.position.x -eq 8 -and $state.data.player.position.y -eq 6) "A fresh client did not arrive at the First Beacon."
        $step = Step-North $guest.account_token ("approach-camp-" + $guest.character_id)
        Assert-True ($step.data.accepted -and $step.data.position.x -eq 8 -and $step.data.position.y -eq 5) "A client could not walk into the tent camp."
        $builder = Use-FoundationInteraction $guest.account_token ("meet-builder-" + $guest.character_id) "speak-with-builder"
        Assert-True ($builder.data.accepted -and $builder.data.landmark_id -eq "builder-mara") "A client could not meet the authoritative builder."
        $board = Use-FoundationInteraction $guest.account_token ("read-need-" + $guest.character_id) "read-local-needs"
        Assert-True ($board.data.accepted -and $board.data.message.Contains("timber") -and $board.data.message.Contains("stone")) "A client could not read the authoritative local need."
    }

    Start-Sleep -Seconds 1
    Stop-FixtureServer $server
    $server = $null
    $server = Start-FixtureServer $statePath (Join-Path $temporaryRoot "return.out.log") (Join-Path $temporaryRoot "return.err.log")

    $resumed = @($keys | ForEach-Object { New-Guest $_ })
    foreach ($guest in $resumed) {
        $state = Read-State $guest.account_token
        Assert-True (@($state.data.world.players).Count -eq 3) "A returning client did not recover the shared three-player state."
        Assert-True ($state.data.player.position.x -eq 8 -and $state.data.player.position.y -eq 5) "A returning client did not recover its authoritative camp position."
        $board = Use-FoundationInteraction $guest.account_token ("return-need-" + $guest.character_id) "read-local-needs"
        Assert-True ($board.data.accepted) "A returning client could not resume the First Beacon context."
    }
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-True ($ops.data.ready -and $ops.data.integrity_ok) "The restarted F1 fixture failed readiness checks."

    Write-Host "F1 arrival passed: three clients shared the camp, met Mara, read the local need, restarted, and returned to their authoritative positions." -ForegroundColor Green
} finally {
    Stop-FixtureServer $server
    Pop-Location -ErrorAction SilentlyContinue
    if ($null -eq $oldAddress) { Remove-Item Env:TARROWYN_SERVER_ADDR -ErrorAction SilentlyContinue } else { $env:TARROWYN_SERVER_ADDR = $oldAddress }
    if ($null -eq $oldStatePath) { Remove-Item Env:TARROWYN_STATE_PATH -ErrorAction SilentlyContinue } else { $env:TARROWYN_STATE_PATH = $oldStatePath }
    if ($null -eq $oldBackupPath) { Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue } else { $env:TARROWYN_BACKUP_PATH = $oldBackupPath }
    if ($null -eq $oldDbDriver) { Remove-Item Env:DB_DRIVER -ErrorAction SilentlyContinue } else { $env:DB_DRIVER = $oldDbDriver }
    $resolvedTemporaryRoot = [System.IO.Path]::GetFullPath($temporaryRoot)
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f1-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
