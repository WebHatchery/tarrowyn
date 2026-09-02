$ErrorActionPreference = "Stop"

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) { throw $message }
}

function Wait-ForHealth([string]$baseUrl, [System.Diagnostics.Process]$process) {
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        if ($process.HasExited) { throw "The authoritative server exited before becoming healthy." }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$baseUrl/health" -TimeoutSec 2
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
    Wait-ForHealth "http://127.0.0.1:8870" $process
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

function Get-DescendantProcessIds([int]$parentId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId")
    foreach ($child in $children) {
        Get-DescendantProcessIds $child.ProcessId
        $child.ProcessId
    }
}

function New-Guest([string]$clientKey) {
    $response = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8870/v1/session/guest" `
        -ContentType "application/json" `
        -Body (@{ client_key = $clientKey; reset = $true } | ConvertTo-Json -Compress)
    return $response.data
}

function Read-State([string]$token) {
    return Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8870/v1/state" `
        -Headers @{ Authorization = "Bearer $token" }
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f0-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-True ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "The F0 temporary path escaped the system temp directory."
$null = New-Item -ItemType Directory -Path $temporaryRoot
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
    cargo test --workspace foundation_baseline
    if ($LASTEXITCODE -ne 0) { throw "Deterministic F0 Rust tests failed." }

    $runOneState = Join-Path $temporaryRoot "run-one.json"
    $server = Start-FixtureServer $runOneState (Join-Path $temporaryRoot "run-one.out.log") (Join-Path $temporaryRoot "run-one.err.log")
    $firstGuest = New-Guest "foundation-live-fixture"
    $firstState = Read-State $firstGuest.account_token
    $firstFoundation = $firstState.data.world.foundation
    $firstCanonical = $firstFoundation | ConvertTo-Json -Depth 20 -Compress

    Assert-True ($firstFoundation.fixture_id -eq "first-beacon-baseline-v1") "The live authority returned the wrong F0 fixture ID."
    Assert-True (@($firstFoundation.landmarks).Count -eq 12) "The live authority did not return all required First Beacon landmarks."
    Assert-True (@($firstFoundation.interactions).Count -eq 12) "The live authority did not return all required First Beacon interactions."
    $beacon = @($firstFoundation.landmarks | Where-Object id -eq "first-beacon")[0]
    Assert-True ($beacon.permanent -and $beacon.visible -and $beacon.position.x -eq 8 -and $beacon.position.y -eq 6) "The live First Beacon is not the stable permanent arrival record."
    $requiredIds = @("first-beacon", "first-beacon-tents", "first-beacon-fire", "builder-mara", "first-beacon-noticeboard", "first-beacon-cache", "first-beacon-tool-rack", "first-beacon-fields", "whisperwood-edge", "first-beacon-mine", "first-beacon-forge", "storehouse-site")
    foreach ($requiredId in $requiredIds) {
        Assert-True (@($firstFoundation.landmarks | Where-Object id -eq $requiredId).Count -eq 1) "The live F0 fixture is missing stable landmark $requiredId."
    }
    $headers = @{ Authorization = "Bearer $($firstGuest.account_token)" }
    $null = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8870/v1/chat" -Headers $headers `
        -ContentType "application/json" -Body (@{ request_id = "foundation-leak-probe"; channel = "settlement"; text = "This belongs only to F0 run one." } | ConvertTo-Json -Compress)
    Stop-FixtureServer $server
    $server = $null

    $runTwoState = Join-Path $temporaryRoot "run-two.json"
    $server = Start-FixtureServer $runTwoState (Join-Path $temporaryRoot "run-two.out.log") (Join-Path $temporaryRoot "run-two.err.log")
    $secondGuest = New-Guest "foundation-live-fixture"
    $secondState = Read-State $secondGuest.account_token
    $secondCanonical = $secondState.data.world.foundation | ConvertTo-Json -Depth 20 -Compress
    Assert-True ($secondCanonical -ceq $firstCanonical) "Repeated fixture creation changed important First Beacon state or persistent IDs."
    Assert-True (@($secondState.data.feed.chat | Where-Object text -eq "This belongs only to F0 run one.").Count -eq 0) "A clean fixture leaked chat state from the previous run."
    Assert-True (@($secondState.data.world.players).Count -eq 1) "A clean fixture leaked a previous-run player record."
    $ops = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8870/v1/ops/health"
    Assert-True ($ops.data.ready -and $ops.data.integrity_ok) "The F0 fixture is incompatible with authoritative readiness checks."

    Write-Host "F0 foundation baseline passed: 12 landmarks, 12 interactions, stable IDs, clean reset, authoritative readiness." -ForegroundColor Green
} finally {
    Stop-FixtureServer $server
    Pop-Location -ErrorAction SilentlyContinue
    if ($null -eq $oldAddress) { Remove-Item Env:TARROWYN_SERVER_ADDR -ErrorAction SilentlyContinue } else { $env:TARROWYN_SERVER_ADDR = $oldAddress }
    if ($null -eq $oldStatePath) { Remove-Item Env:TARROWYN_STATE_PATH -ErrorAction SilentlyContinue } else { $env:TARROWYN_STATE_PATH = $oldStatePath }
    if ($null -eq $oldBackupPath) { Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue } else { $env:TARROWYN_BACKUP_PATH = $oldBackupPath }
    if ($null -eq $oldDbDriver) { Remove-Item Env:DB_DRIVER -ErrorAction SilentlyContinue } else { $env:DB_DRIVER = $oldDbDriver }
    $resolvedTemporaryRoot = [System.IO.Path]::GetFullPath($temporaryRoot)
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f0-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
