$ErrorActionPreference = "Stop"

function Assert-F6([bool]$condition, [string]$message) {
    if (-not $condition) { throw "F6 acceptance failed: $message" }
}

function Get-DescendantProcessIds([int]$parentId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId")
    foreach ($child in $children) {
        Get-DescendantProcessIds $child.ProcessId
        $child.ProcessId
    }
}

function Wait-ForHealth([System.Diagnostics.Process]$process, [string]$stderrPath) {
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        if ($process.HasExited) {
            $detail = if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw } else { "no server error log" }
            throw "The F6 server exited before becoming healthy: $detail"
        }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The F6 server did not become healthy in time."
}

function Start-F6Server([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -WorkingDirectory $projectRoot -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process $stderrPath
    return $process
}

function Stop-F6Server([System.Diagnostics.Process]$process) {
    if ($null -eq $process -or $process.HasExited) { return }
    $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 1
            Start-Sleep -Milliseconds 100
        } catch {
            return
        }
    }
    throw "The previous F6 server did not stop."
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

function Read-Infrastructure([string]$token) {
    return Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/infrastructure" `
        -Headers @{ Authorization = "Bearer $token" }
}

function Read-Events([string]$token) {
    return Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/events?since=0" `
        -Headers @{ Authorization = "Bearer $token" }
}

function Storehouse-CompletionEventCount([string]$token) {
    $events = Read-Events $token
    return @($events.data.events | Where-Object {
        $_.event.kind -eq "Chronicle" -and $_.event.value.kind -eq "storehouse completed"
    }).Count
}

function Move-Player([string]$token, [string]$requestId, [int]$dx, [int]$dy) {
    $response = Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/movement" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; dx = $dx; dy = $dy } | ConvertTo-Json -Compress)
    Assert-F6 $response.data.accepted "movement '$requestId' was rejected"
}

function Move-Path([string]$token, [string]$prefix, [array]$steps) {
    $index = 0
    foreach ($step in $steps) {
        $index++
        Move-Player $token "$prefix-$index" $step[0] $step[1]
    }
}

function Use-Resource([string]$token, [string]$requestId, [string]$nodeId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/resources" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; node_id = $nodeId; action = $action } | ConvertTo-Json -Compress)
}

function Use-Storehouse([string]$token, [hashtable]$body) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/storehouse" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Depth 10 -Compress)
}

function New-MaterialContribution([string]$requestId, [string]$kind, [int]$amount) {
    return @{
        request_id = $requestId
        action = "contribute"
        landmark_id = "storehouse-site"
        contribution = @{ source = "material"; kind = $kind; amount = $amount }
    }
}

function New-GoldContribution([string]$requestId, [string]$kind, [int]$amount) {
    return @{
        request_id = $requestId
        action = "contribute"
        landmark_id = "builder-mara"
        contribution = @{ source = "gold"; toward = $kind; amount = $amount }
    }
}

function Assert-Stage([string]$token, [string]$expectedStage, [int]$expectedRevision, [int]$expectedContributions) {
    $state = Read-State $token
    $storehouse = $state.data.world.foundation_activity.storehouse
    Assert-F6 ($storehouse.current_stage -eq $expectedStage) "observer saw '$($storehouse.current_stage)' instead of '$expectedStage'"
    Assert-F6 ($storehouse.revision -eq $expectedRevision) "storehouse revision was not $expectedRevision at '$expectedStage'"
    Assert-F6 (@($storehouse.contributions).Count -eq $expectedContributions) "contribution ledger count was not $expectedContributions at '$expectedStage'"
    return $storehouse
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f6-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-F6 ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "temporary path escaped the system temp directory"
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8875"
$environmentNames = @("TARROWYN_SERVER_ADDR", "TARROWYN_STATE_PATH", "TARROWYN_BACKUP_PATH", "TARROWYN_MOVEMENT_COOLDOWN_TICKS", "TARROWYN_TICK_MS", "DB_DRIVER")
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$server = $null

try {
    Push-Location $projectRoot
    $auditRows = @(Get-Content (Join-Path $projectRoot "docs/FOUNDATIONAL_PLAYABILITY_AUDIT.md") | Where-Object { $_ -match '^\| [A-Z]{2}-\d{2} \|' })
    $auditStatuses = @($auditRows | ForEach-Object { ($_ -split '\|')[3].Trim() })
    Assert-F6 ($auditRows.Count -eq 132) "the foundational audit no longer contains exactly 132 rows"
    Assert-F6 ((@($auditStatuses | Where-Object { $_ -eq "usable" })).Count -eq 49) "the F6 usable audit total is not 49"
    Assert-F6 ((@($auditStatuses | Where-Object { $_ -eq "partial" })).Count -eq 40) "the F6 partial audit total is not 40"
    Assert-F6 ((@($auditStatuses | Where-Object { $_ -eq "missing" })).Count -eq 29) "the F6 missing audit total is not 29"
    Assert-F6 ((@($auditStatuses | Where-Object { $_ -eq "conflicting" })).Count -eq 1) "the audit conflict count changed during F6"
    Assert-F6 ((@($auditStatuses | Where-Object { $_ -eq "deliberately deferred" })).Count -eq 13) "the audit deferral count changed during F6"
    cargo test -p years_of_tarrowyn storehouse
    if ($LASTEXITCODE -ne 0) { throw "F6 touch-storehouse tests failed." }
    cargo test -p tarrowyn-server storehouse_contract
    if ($LASTEXITCODE -ne 0) { throw "F6 storehouse-authority tests failed." }

    $statePath = Join-Path $temporaryRoot "storehouse-state.json"
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:8875"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "100"
    $env:DB_DRIVER = "json"
    Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
    $server = Start-F6Server $statePath (Join-Path $temporaryRoot "first.out.log") (Join-Path $temporaryRoot "first.err.log")

    $hauler = New-Guest "f6-connected-hauler"
    $patron = New-Guest "f6-connected-patron"
    $observer = New-Guest "f6-connected-observer"
    $patronStartGold = (Read-State $patron.account_token).data.player.gold
    Assert-F6 ($patronStartGold -ge 12) "the fixture patron cannot fund the planned exact substitutions"
    Move-Path $hauler.account_token "hauler-to-mine" @(@(1, 0), @(1, 0), @(0, -1), @(0, -1))
    $stone = Use-Resource $hauler.account_token "f6-gather-stone" "shallow-stone-seam-node" "mine"
    Assert-F6 ($stone.data.accepted -and $stone.data.player.inventory.stone -eq 2) "the hauler did not gather exactly two stone through the real node"
    Move-Path $hauler.account_token "hauler-to-wood" @(@(1, 0), @(1, 0), @(0, -1))
    for ($index = 1; $index -le 4; $index++) {
        $timber = Use-Resource $hauler.account_token "f6-gather-timber-$index" "whisperwood-edge-node" "log"
        Assert-F6 $timber.data.accepted "timber gathering action $index was rejected"
    }
    Assert-F6 ($timber.data.player.inventory.timber -eq 8) "the hauler did not gather exactly eight timber through the real node"
    Move-Path $hauler.account_token "hauler-to-site" @(@(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(0, 1), @(0, 1), @(0, 1), @(0, 1))
    Move-Player $patron.account_token "patron-to-mara" -1 0
    Move-Player $observer.account_token "observer-to-board" 1 0

    $inspect = Use-Storehouse $observer.account_token @{ request_id = "f6-observer-inspect"; action = "inspect"; landmark_id = "first-beacon-noticeboard" }
    Assert-F6 $inspect.data.accepted "the connected observer could not read the local need"
    Assert-F6 ($inspect.data.storehouse.requirements[0].units_required -eq 8 -and $inspect.data.storehouse.requirements[1].units_required -eq 6) "the projected storehouse requirement changed"

    $stoneRequest = New-MaterialContribution "f6-contribute-stone-2" "stone" 2
    $stoneContribution = Use-Storehouse $hauler.account_token $stoneRequest
    Assert-F6 ($stoneContribution.data.accepted -and $stoneContribution.data.player.inventory.stone -eq 0) "the material contribution was not atomic"
    $stoneReplay = Use-Storehouse $hauler.account_token $stoneRequest
    Assert-F6 (($stoneReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($stoneContribution.data | ConvertTo-Json -Depth 30 -Compress)) "material retry did not return its original result"
    $null = Assert-Stage $observer.account_token "site_marked" 2 1

    $timberTwo = Use-Storehouse $hauler.account_token (New-MaterialContribution "f6-contribute-timber-2" "timber" 2)
    Assert-F6 $timberTwo.data.accepted "the first timber contribution was rejected"
    $foundation = Use-Storehouse $patron.account_token (New-GoldContribution "f6-fund-stone-1" "stone" 3)
    Assert-F6 ($foundation.data.accepted -and $foundation.data.player.gold -eq ($patronStartGold - 3)) "the exact foundation gold substitution was not charged once"
    $null = Assert-Stage $observer.account_token "foundation_laid" 4 3

    $timberFour = Use-Storehouse $hauler.account_token (New-MaterialContribution "f6-contribute-timber-4" "timber" 4)
    Assert-F6 $timberFour.data.accepted "the frame timber contribution was rejected"
    $frame = Use-Storehouse $patron.account_token (New-GoldContribution "f6-fund-stone-frame" "stone" 3)
    Assert-F6 ($frame.data.accepted -and $frame.data.player.gold -eq ($patronStartGold - 6)) "the exact frame gold substitution was not charged once"
    $null = Assert-Stage $observer.account_token "frame_raised" 6 5

    $timberFinal = Use-Storehouse $hauler.account_token (New-MaterialContribution "f6-contribute-timber-final" "timber" 2)
    Assert-F6 ($timberFinal.data.accepted -and $timberFinal.data.player.inventory.timber -eq 0) "the final timber was not consumed exactly once"
    $finalRequest = New-GoldContribution "f6-fund-stone-final" "stone" 6
    $completed = Use-Storehouse $patron.account_token $finalRequest
    Assert-F6 ($completed.data.accepted -and $completed.data.player.gold -eq ($patronStartGold - 12)) "the final gold substitution was not charged exactly once"
    Assert-F6 ($completed.data.storehouse.current_stage -eq "operational") "the exact contribution total did not make the storehouse operational"
    Assert-F6 (@($completed.data.storehouse.completion.contributor_account_ids).Count -eq 2) "completion did not retain both contributors"
    $operational = Assert-Stage $observer.account_token "operational" 8 7
    Assert-F6 ($operational.completion.operational_infrastructure_id -eq "first-beacon-storehouse") "observer state did not expose durable completion"
    $infrastructure = Read-Infrastructure $observer.account_token
    Assert-F6 (@($infrastructure.data.records | Where-Object infrastructure_id -eq "first-beacon-storehouse").Count -eq 1) "completion did not create exactly one public infrastructure record"

    $completionReplay = Use-Storehouse $patron.account_token $finalRequest
    Assert-F6 (($completionReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($completed.data | ConvertTo-Json -Depth 30 -Compress)) "completion retry did not return its original result"
    Assert-F6 ((Storehouse-CompletionEventCount $observer.account_token) -eq 1) "completion did not emit exactly one chronicle event"
    Stop-F6Server $server
    $server = $null
    $server = Start-F6Server $statePath (Join-Path $temporaryRoot "restart.out.log") (Join-Path $temporaryRoot "restart.err.log")
    $restartedHauler = New-Guest "f6-connected-hauler"
    $restartedPatron = New-Guest "f6-connected-patron"
    $restartedObserver = New-Guest "f6-connected-observer"
    Assert-F6 ($restartedHauler.character_id -eq $hauler.character_id -and $restartedPatron.character_id -eq $patron.character_id) "contributor identities did not survive restart"
    $restartReplay = Use-Storehouse $restartedPatron.account_token $finalRequest
    Assert-F6 (($restartReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($completed.data | ConvertTo-Json -Depth 30 -Compress)) "completion replay was lost across restart"
    $restartStorehouse = Assert-Stage $restartedObserver.account_token "operational" 8 7
    Assert-F6 (@($restartStorehouse.completion.contributor_account_ids).Count -eq 2) "restart changed the contributor record"
    $restartInfrastructure = Read-Infrastructure $restartedObserver.account_token
    Assert-F6 (@($restartInfrastructure.data.records | Where-Object infrastructure_id -eq "first-beacon-storehouse").Count -eq 1) "restart or replay duplicated the public storehouse"
    Assert-F6 ((Storehouse-CompletionEventCount $restartedObserver.account_token) -eq 1) "restart or replay emitted a duplicate completion event"
    Assert-F6 ($restartReplay.data.player.gold -eq ($patronStartGold - 12)) "restart or replay duplicated a charge or reward"
    $stored = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    Assert-F6 ($stored.storage_version -eq 26) "the scenario did not persist the current storehouse contract"
    Assert-F6 (@($stored.phase4.infrastructure | Where-Object infrastructure_id -eq "first-beacon-storehouse").Count -eq 1) "persisted state contains a duplicate storehouse"
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-F6 ($ops.data.ready -and $ops.data.integrity_ok) "the restarted F6 fixture failed readiness checks"

    Write-Host "F6 storehouse passed: real gathering, mixed goods/gold contributions, four observed stages, retries, restart persistence, one public structure, and no duplicate charges, events, or rewards." -ForegroundColor Green
} finally {
    Stop-F6Server $server
    Pop-Location -ErrorAction SilentlyContinue
    foreach ($name in $environmentNames) {
        $value = $previousEnvironment[$name]
        if ($null -eq $value) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            Set-Item -Path "Env:$name" -Value $value
        }
    }
    $resolvedTemporaryRoot = [System.IO.Path]::GetFullPath($temporaryRoot)
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f6-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
