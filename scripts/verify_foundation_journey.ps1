$ErrorActionPreference = "Stop"

function Assert-F7([bool]$condition, [string]$message) {
    if (-not $condition) { throw "F7 acceptance failed: $message" }
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
            throw "The F7 server exited before becoming healthy: $detail"
        }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The F7 server did not become healthy in time."
}

function Start-F7Server([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -WorkingDirectory $projectRoot -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process $stderrPath
    return $process
}

function Stop-F7Server([System.Diagnostics.Process]$process) {
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
    throw "The previous F7 server did not stop."
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

function Read-Journey([string]$token) {
    return Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/foundation/journey" `
        -Headers @{ Authorization = "Bearer $token" }
}

function Assert-Journey([string]$token, [int]$completed, [string]$nextMilestone) {
    $journey = Read-Journey $token
    Assert-F7 ($journey.data.completed_milestones -eq $completed) "journey recorded $($journey.data.completed_milestones) milestones instead of $completed"
    if ($nextMilestone) {
        Assert-F7 ($journey.data.next_milestone.milestone_id -eq $nextMilestone) "next milestone was not '$nextMilestone'"
    } else {
        Assert-F7 ($null -eq $journey.data.next_milestone) "completed journey still projected a milestone"
    }
    return $journey
}

function Move-Player([string]$token, [string]$requestId, [int]$dx, [int]$dy) {
    $response = Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/movement" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; dx = $dx; dy = $dy } | ConvertTo-Json -Compress)
    Assert-F7 $response.data.accepted "movement '$requestId' was rejected"
}

function Move-Path([string]$token, [string]$prefix, [array]$steps) {
    $index = 0
    foreach ($step in $steps) {
        $index++
        Move-Player $token "$prefix-$index" $step[0] $step[1]
    }
}

function Use-Interaction([string]$token, [string]$requestId, [string]$interactionId) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/interactions" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; interaction_id = $interactionId } | ConvertTo-Json -Compress)
}

function Use-Field([string]$token, [string]$requestId, [string]$action, [int]$x, [int]$y) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/farming/actions" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; action = $action; position = @{ x = $x; y = $y } } | ConvertTo-Json -Compress)
}

function Use-Resource([string]$token, [string]$requestId, [string]$nodeId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/resources" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; node_id = $nodeId; action = $action } | ConvertTo-Json -Compress)
}

function Use-Forge([string]$token, [string]$requestId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/forge" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; action = $action } | ConvertTo-Json -Compress)
}

function Use-Trade([string]$token, [hashtable]$body) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/trades" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Depth 10 -Compress)
}

function New-TradeBundle([int]$stone, [int]$gold) {
    return @{ wheat = 0; turnips = 0; moonberries = 0; seeds = 0; timber = 0; stone = $stone; iron_ore = 0; charcoal = 0; tool_handles = 0; gold = $gold }
}

function Use-Storehouse([string]$token, [string]$requestId) {
    $body = @{
        request_id = $requestId
        action = "contribute"
        landmark_id = "storehouse-site"
        contribution = @{ source = "material"; kind = "stone"; amount = 1 }
    }
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/storehouse" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Depth 10 -Compress)
}

function Advance-OfflineCrop([string]$statePath, [int]$minutes) {
    $stored = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    Assert-F7 ($stored.storage_version -eq 27) "journey scenario did not persist storage version 27"
    $stored.persisted_at_unix_millis = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - ($minutes * 60 * 1000)
    [System.IO.File]::WriteAllText($statePath, ($stored | ConvertTo-Json -Depth 100), [System.Text.UTF8Encoding]::new($false))
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f7-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-F7 ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "temporary path escaped the system temp directory"
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8876"
$environmentNames = @("TARROWYN_SERVER_ADDR", "TARROWYN_STATE_PATH", "TARROWYN_BACKUP_PATH", "TARROWYN_MOVEMENT_COOLDOWN_TICKS", "TARROWYN_TICK_MS", "TARROWYN_WORLD_SECONDS_PER_TICK", "TARROWYN_CROP_STAGE_SECONDS", "DB_DRIVER")
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$server = $null

try {
    Push-Location $projectRoot
    $auditRows = @(Get-Content (Join-Path $projectRoot "docs/FOUNDATIONAL_PLAYABILITY_AUDIT.md") | Where-Object { $_ -match '^\| [A-Z]{2}-\d{2} \|' })
    $auditStatuses = @($auditRows | ForEach-Object { ($_ -split '\|')[3].Trim() })
    Assert-F7 ($auditRows.Count -eq 132) "the foundational audit no longer contains exactly 132 rows"
    Assert-F7 ((@($auditStatuses | Where-Object { $_ -eq "usable" })).Count -eq 60) "the F7 usable audit total is not 60"
    Assert-F7 ((@($auditStatuses | Where-Object { $_ -eq "partial" })).Count -eq 31) "the F7 partial audit total is not 31"
    Assert-F7 ((@($auditStatuses | Where-Object { $_ -eq "missing" })).Count -eq 27) "the F7 missing audit total is not 27"
    Assert-F7 ((@($auditStatuses | Where-Object { $_ -eq "conflicting" })).Count -eq 1) "the audit conflict count changed during F7"
    Assert-F7 ((@($auditStatuses | Where-Object { $_ -eq "deliberately deferred" })).Count -eq 13) "the audit deferral count changed during F7"
    cargo test -p years_of_tarrowyn journey_guidance
    if ($LASTEXITCODE -ne 0) { throw "F7 client journey guidance tests failed." }
    cargo test -p tarrowyn-server journey_contract
    if ($LASTEXITCODE -ne 0) { throw "F7 journey authority tests failed." }

    $statePath = Join-Path $temporaryRoot "journey-state.json"
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:8876"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "100"
    $env:TARROWYN_WORLD_SECONDS_PER_TICK = "1"
    $env:TARROWYN_CROP_STAGE_SECONDS = "300"
    $env:DB_DRIVER = "json"
    Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
    $server = Start-F7Server $statePath (Join-Path $temporaryRoot "first.out.log") (Join-Path $temporaryRoot "first.err.log")

    $short = New-Guest "f7-short-visit"
    $shortStart = Assert-Journey $short.account_token 1 "consult-first-need"
    Assert-F7 ($shortStart.data.contract.rhythms[0].target_minutes -eq 15) "short-visit contract lost its 15-minute target"
    Move-Player $short.account_token "f7-short-to-needs" 1 0
    $shortConsult = Use-Interaction $short.account_token "f7-short-consult" "read-local-needs"
    Assert-F7 $shortConsult.data.accepted "short visitor could not consult the local need"
    Move-Path $short.account_token "f7-short-to-field" @(@(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(0, 1), @(0, 1))
    $shortPlant = Use-Field $short.account_token "f7-short-plant" "plant" 2 8
    Assert-F7 $shortPlant.data.accepted "short visitor could not leave a crop growing"
    $shortDone = Assert-Journey $short.account_token 3 "explore-whisperwood"
    Assert-F7 ($shortDone.data.contract.rhythms[0].required_milestone_ids -join ',' -eq "consult-first-need,plant-common-field") "short rhythm no longer names consult and plant"

    $resident = New-Guest "f7-first-hour"
    $neighbour = New-Guest "f7-neighbour"
    Move-Player $resident.account_token "f7-to-needs" 1 0
    Assert-F7 (Use-Interaction $resident.account_token "f7-consult" "read-local-needs").data.accepted "resident could not consult the local need"
    Move-Path $resident.account_token "f7-to-field" @(@(1, 0), @(0, 1), @(0, 1))
    Assert-F7 (Use-Field $resident.account_token "f7-plant" "plant" 10 8).data.accepted "resident could not plant the journey crop"
    Move-Path $resident.account_token "f7-to-wood" @(@(1, 0), @(1, 0), @(0, -1), @(0, -1), @(0, -1), @(0, -1), @(0, -1))
    $timber = Use-Resource $resident.account_token "f7-log" "whisperwood-edge-node" "log"
    Assert-F7 ($timber.data.accepted -and $timber.data.player.inventory.timber -eq 2) "journey logging did not supply two timber"
    Move-Path $resident.account_token "f7-to-mine" @(@(-1, 0), @(-1, 0), @(0, 1))
    $mineOne = Use-Resource $resident.account_token "f7-mine-1" "shallow-stone-seam-node" "mine"
    $mineTwo = Use-Resource $resident.account_token "f7-mine-2" "shallow-stone-seam-node" "mine"
    Assert-F7 ($mineOne.data.accepted -and $mineTwo.data.accepted -and $mineTwo.data.player.inventory.iron_ore -eq 2) "journey mining did not supply two ore"
    Move-Player $resident.account_token "f7-to-forge" 0 1
    foreach ($step in @(@{ id = "f7-charcoal"; action = "burn_charcoal" }, @{ id = "f7-handle"; action = "shape_handle" }, @{ id = "f7-tool"; action = "forge_field_tool" })) {
        Assert-F7 (Use-Forge $resident.account_token $step.id $step.action).data.accepted "journey forge step '$($step.action)' was rejected"
    }
    $created = Use-Trade $resident.account_token @{ request_id = "f7-trade-create"; action = "create"; recipient_account_id = $neighbour.account_id; offer = New-TradeBundle 1 0; request = New-TradeBundle 0 1 }
    Assert-F7 $created.data.accepted "journey barter was not created"
    $tradeId = $created.data.trade.trade_id
    $accepted = Use-Trade $neighbour.account_token @{ request_id = "f7-trade-accept"; action = "accept"; trade_id = $tradeId }
    $acceptedReplay = Use-Trade $neighbour.account_token @{ request_id = "f7-trade-accept"; action = "accept"; trade_id = $tradeId }
    Assert-F7 ($accepted.data.accepted -and (($accepted.data | ConvertTo-Json -Depth 30 -Compress) -eq ($acceptedReplay.data | ConvertTo-Json -Depth 30 -Compress))) "journey barter was not atomic and replay-safe"
    Move-Path $resident.account_token "f7-to-site" @(@(-1, 0), @(-1, 0), @(-1, 0), @(-1, 0), @(0, 1), @(0, 1))
    Assert-F7 (Use-Storehouse $resident.account_token "f7-storehouse").data.accepted "journey contribution was rejected"
    $beforeAway = Assert-Journey $resident.account_token 10 "harvest-common-field"
    Assert-F7 ($beforeAway.data.progress.revision -eq 11) "first-hour progress revision was not 11 before the return harvest"

    Stop-F7Server $server
    $server = $null
    Advance-OfflineCrop $statePath 15
    $server = Start-F7Server $statePath (Join-Path $temporaryRoot "return.out.log") (Join-Path $temporaryRoot "return.err.log")
    $resident = New-Guest "f7-first-hour"
    Assert-F7 ((Assert-Journey $resident.account_token 10 "harvest-common-field").data.progress.revision -eq 11) "restart changed the journey ledger"
    Move-Path $resident.account_token "f7-return-to-field" @(@(1, 0), @(1, 0), @(1, 0), @(1, 0), @(0, 1))
    Assert-F7 (Use-Field $resident.account_token "f7-harvest" "harvest" 10 8).data.accepted "mature journey crop could not be harvested"
    $replant = Use-Field $resident.account_token "f7-replant" "plant" 10 8
    $replantReplay = Use-Field $resident.account_token "f7-replant" "plant" 10 8
    Assert-F7 ($replant.data.accepted -and (($replant.data | ConvertTo-Json -Depth 30 -Compress) -eq ($replantReplay.data | ConvertTo-Json -Depth 30 -Compress))) "replant was not replay-safe"
    $complete = Assert-Journey $resident.account_token 12 ""
    Assert-F7 ($complete.data.progress.revision -eq 13 -and $complete.data.progress.future_goal_state -eq "active") "complete first hour did not activate the return goal exactly once"
    Assert-F7 ($complete.data.next_action -match "replanted|matures|harvest") "return goal did not expose a concrete next action"

    Stop-F7Server $server
    $server = $null
    Advance-OfflineCrop $statePath 15
    $server = Start-F7Server $statePath (Join-Path $temporaryRoot "future.out.log") (Join-Path $temporaryRoot "future.err.log")
    $resident = New-Guest "f7-first-hour"
    $futureHarvest = Use-Field $resident.account_token "f7-future-harvest" "harvest" 10 8
    $futureReplay = Use-Field $resident.account_token "f7-future-harvest" "harvest" 10 8
    Assert-F7 ($futureHarvest.data.accepted -and (($futureHarvest.data | ConvertTo-Json -Depth 30 -Compress) -eq ($futureReplay.data | ConvertTo-Json -Depth 30 -Compress))) "future harvest was not replay-safe"
    $fulfilled = Assert-Journey $resident.account_token 12 ""
    Assert-F7 ($fulfilled.data.progress.revision -eq 14 -and $fulfilled.data.progress.future_goal_state -eq "complete") "future-session goal did not complete exactly once"
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-F7 ($ops.data.ready -and $ops.data.integrity_ok) "restarted F7 fixture failed readiness checks"

    Write-Host "F7 journey passed: useful short visit, ordered first-hour world actions, barter and construction, restart/replay recovery, and one durable return harvest." -ForegroundColor Green
} finally {
    Stop-F7Server $server
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
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f7-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
