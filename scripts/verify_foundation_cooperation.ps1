$ErrorActionPreference = "Stop"

function Assert-F5([bool]$condition, [string]$message) {
    if (-not $condition) { throw "F5 acceptance failed: $message" }
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
            throw "The F5 server exited before becoming healthy: $detail"
        }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The F5 server did not become healthy in time."
}

function Start-F5Server([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -WorkingDirectory $projectRoot -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process $stderrPath
    return $process
}

function Stop-F5Server([System.Diagnostics.Process]$process) {
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
    throw "The previous F5 server did not stop."
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

function Move-Player([string]$token, [string]$requestId, [int]$dx, [int]$dy) {
    $response = Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/movement" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; dx = $dx; dy = $dy } | ConvertTo-Json -Compress)
    Assert-F5 $response.data.accepted "movement '$requestId' was rejected"
}

function Move-Path([string]$token, [string]$prefix, [array]$steps) {
    $index = 0
    foreach ($step in $steps) {
        $index++
        Move-Player $token "$prefix-$index" $step[0] $step[1]
    }
}

function Practice-Skill([string]$token, [string]$requestId, [string]$skillId) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/skills" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; action = "practice"; skill_id = $skillId } | ConvertTo-Json -Compress)
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

function New-TradeBundle([int]$ironOre) {
    return @{
        wheat = 0
        turnips = 0
        moonberries = 0
        seeds = 0
        timber = 0
        stone = 0
        iron_ore = $ironOre
        charcoal = 0
        tool_handles = 0
        gold = 0
    }
}

function Complete-Forge([string]$token, [string]$prefix) {
    foreach ($step in @(
        @{ id = "$prefix-charcoal"; action = "burn_charcoal" },
        @{ id = "$prefix-handle"; action = "shape_handle" },
        @{ id = "$prefix-tool"; action = "forge_field_tool" }
    )) {
        $result = Use-Forge $token $step.id $step.action
        Assert-F5 $result.data.accepted "forge step '$($step.action)' was rejected"
    }
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f5-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-F5 ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "temporary path escaped the system temp directory"
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8874"
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
    Assert-F5 ($auditRows.Count -eq 132) "the foundational audit no longer contains exactly 132 rows"
    Assert-F5 ((@($auditStatuses | Where-Object { $_ -eq "usable" })).Count -eq 39) "the F5 usable audit total is not 39"
    Assert-F5 ((@($auditStatuses | Where-Object { $_ -eq "partial" })).Count -eq 43) "the F5 partial audit total is not 43"
    Assert-F5 ((@($auditStatuses | Where-Object { $_ -eq "missing" })).Count -eq 36) "the F5 missing audit total is not 36"
    Assert-F5 ((@($auditStatuses | Where-Object { $_ -eq "conflicting" })).Count -eq 1) "the audit conflict count changed during F5"
    Assert-F5 ((@($auditStatuses | Where-Object { $_ -eq "deliberately deferred" })).Count -eq 13) "the audit deferral count changed during F5"
    cargo test -p years_of_tarrowyn cooperation
    if ($LASTEXITCODE -ne 0) { throw "F5 touch-cooperation tests failed." }
    cargo test -p tarrowyn-server cooperation_contract
    if ($LASTEXITCODE -ne 0) { throw "F5 cooperation-authority tests failed." }

    $statePath = Join-Path $temporaryRoot "cooperation-state.json"
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:8874"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "100"
    $env:DB_DRIVER = "json"
    Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
    $server = Start-F5Server $statePath (Join-Path $temporaryRoot "first.out.log") (Join-Path $temporaryRoot "first.err.log")

    $miner = New-Guest "f5-connected-miner"
    $smith = New-Guest "f5-connected-smith"
    for ($index = 1; $index -le 4; $index++) {
        $practice = Practice-Skill $miner.account_token "f5-mining-practice-$index" "mining"
        Assert-F5 $practice.data.accepted "voluntary Mining practice $index was rejected"
    }
    $skills = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/skills" -Headers @{ Authorization = "Bearer $($miner.account_token)" }
    $mining = @($skills.data.skills | Where-Object skill_id -eq "mining")[0]
    Assert-F5 ($mining.mastery -eq 2 -and $mining.usable) "voluntary Mining commitment did not reach the fixed efficiency threshold"

    Move-Path $miner.account_token "miner-to-mine" @(@(1, 0), @(1, 0), @(0, -1), @(0, -1))
    $ore = Use-Resource $miner.account_token "f5-efficient-ore" "shallow-stone-seam-node" "mine"
    Assert-F5 $ore.data.accepted "the practised miner's extraction was rejected"
    Assert-F5 ($ore.data.player.inventory.iron_ore -eq 2) "the practised miner did not extract the fixed two ore in one action"

    Move-Path $smith.account_token "smith-to-wood" @(@(1, 0), @(1, 0), @(1, 0), @(1, 0), @(0, -1), @(0, -1), @(0, -1))
    $timber = Use-Resource $smith.account_token "f5-goal-timber" "whisperwood-edge-node" "log"
    Assert-F5 $timber.data.accepted "the smith's timber action was rejected"
    Assert-F5 ($timber.data.player.inventory.timber -eq 2) "the smith did not gather the fixed two timber"

    $created = Use-Trade $miner.account_token @{
        request_id = "f5-offer-ore"
        action = "create"
        recipient_account_id = $smith.account_id
        offer = New-TradeBundle 2
        request = New-TradeBundle 0
    }
    Assert-F5 $created.data.accepted "the exact two-ore offer was rejected"
    $tradeId = $created.data.trade.trade_id
    $accepted = Use-Trade $smith.account_token @{
        request_id = "f5-accept-ore"
        action = "accept"
        trade_id = $tradeId
    }
    $acceptedReplay = Use-Trade $smith.account_token @{
        request_id = "f5-accept-ore"
        action = "accept"
        trade_id = $tradeId
    }
    Assert-F5 $accepted.data.accepted "the exact two-ore offer was not accepted atomically"
    Assert-F5 (($acceptedReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($accepted.data | ConvertTo-Json -Depth 30 -Compress)) "trade retry did not return its original result"
    $afterTrade = Read-State $smith.account_token
    Assert-F5 ($afterTrade.data.player.inventory.iron_ore -eq 2 -and $afterTrade.data.player.inventory.timber -eq 2) "accepted barter did not leave the coordinator with exact goal inputs"
    $attempt = @($afterTrade.data.world.foundation_activity.cooperation.active_attempts)[0]
    Assert-F5 ($attempt.trade_id -eq $tradeId -and $attempt.work_actions -eq 2) "accepted barter did not open the two-action cooperation ledger"

    Move-Path $smith.account_token "smith-to-forge" @(@(-1, 0), @(-1, 0), @(0, 1))
    Complete-Forge $smith.account_token "f5-cooperative"
    $cooperativeState = Read-State $smith.account_token
    $result = $cooperativeState.data.world.foundation_activity.cooperation.latest_result
    Assert-F5 ($result.trade_id -eq $tradeId) "the completed result lost its atomic trade reference"
    Assert-F5 ($result.work_actions -eq 5 -and $result.saved_work_actions -eq 1) "cooperation did not record exactly five actions and one saved"
    Assert-F5 (@($result.participant_account_ids).Count -eq 2) "the cooperation result did not retain both voluntary participants"
    Assert-F5 ((@($result.contributions | Measure-Object -Property work_actions -Sum).Sum) -eq 5) "participant work attribution did not sum to the result"

    $solo = New-Guest "f5-connected-solo"
    Move-Path $solo.account_token "solo-to-mine" @(@(1, 0), @(1, 0), @(0, -1), @(0, -1))
    for ($index = 1; $index -le 2; $index++) {
        $soloOre = Use-Resource $solo.account_token "f5-solo-mine-$index" "shallow-stone-seam-node" "mine"
        Assert-F5 $soloOre.data.accepted "solo crude mining action $index was rejected"
    }
    Assert-F5 ($soloOre.data.player.inventory.iron_ore -eq 2) "uncommitted solo mining did not retain its two-action fallback"
    Move-Path $solo.account_token "solo-to-wood" @(@(1, 0), @(1, 0), @(0, -1))
    $soloTimber = Use-Resource $solo.account_token "f5-solo-log" "whisperwood-edge-node" "log"
    Assert-F5 ($soloTimber.data.accepted -and $soloTimber.data.player.inventory.timber -eq 2) "solo timber fallback was unavailable"
    Move-Path $solo.account_token "solo-to-forge" @(@(-1, 0), @(-1, 0), @(0, 1))
    Complete-Forge $solo.account_token "f5-solo"
    $soloState = Read-State $solo.account_token
    Assert-F5 ($soloState.data.player.field_tool_kind -eq "iron") "the uncommitted solo player could not finish the same tool"
    Assert-F5 ($soloState.data.world.foundation_activity.cooperation.latest_result.trade_id -eq $tradeId) "solo fallback incorrectly replaced the measured cooperative result"

    Stop-F5Server $server
    $server = $null
    $server = Start-F5Server $statePath (Join-Path $temporaryRoot "restart.out.log") (Join-Path $temporaryRoot "restart.err.log")
    $restartedMiner = New-Guest "f5-connected-miner"
    $restartedSmith = New-Guest "f5-connected-smith"
    Assert-F5 ($restartedMiner.character_id -eq $miner.character_id -and $restartedSmith.character_id -eq $smith.character_id) "cooperation identities did not survive restart"
    $restartReplay = Use-Trade $restartedSmith.account_token @{
        request_id = "f5-accept-ore"
        action = "accept"
        trade_id = $tradeId
    }
    Assert-F5 (($restartReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($accepted.data | ConvertTo-Json -Depth 30 -Compress)) "accepted trade replay was lost across restart"
    $forgeReplay = Use-Forge $restartedSmith.account_token "f5-cooperative-tool" "forge_field_tool"
    Assert-F5 $forgeReplay.data.accepted "completed forge replay was lost across restart"
    $restartedState = Read-State $restartedSmith.account_token
    $restartedResult = $restartedState.data.world.foundation_activity.cooperation.latest_result
    Assert-F5 (($restartedResult | ConvertTo-Json -Depth 30 -Compress) -eq ($result | ConvertTo-Json -Depth 30 -Compress)) "the measured cooperation result changed across restart/replay"
    Assert-F5 ($restartedState.data.player.inventory.iron_ore -eq 0) "replayed work duplicated the coordinator's ore"
    $stored = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    Assert-F5 ($stored.storage_version -eq 25) "the scenario did not persist the current cooperation contract"
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-F5 ($ops.data.ready -and $ops.data.integrity_ok) "the restarted F5 fixture failed readiness checks"

    Write-Host "F5 cooperation passed: voluntary Mining efficiency, exact two-player barter, 5-vs-6 accepted actions, attribution, retries, restart persistence, and unrestricted solo fallback." -ForegroundColor Green
} finally {
    Stop-F5Server $server
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
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f5-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
