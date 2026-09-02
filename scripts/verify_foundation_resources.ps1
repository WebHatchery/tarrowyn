$ErrorActionPreference = "Stop"

function Assert-F2([bool]$condition, [string]$message) {
    if (-not $condition) { throw "F2 acceptance failed: $message" }
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
        if ($process.HasExited) { throw "The F2 server exited before becoming healthy." }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The F2 server did not become healthy in time."
}

function Start-F2Server([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process
    return $process
}

function Stop-F2Server([System.Diagnostics.Process]$process) {
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
    throw "The previous F2 server did not stop."
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
    Assert-F2 $response.data.accepted "movement '$requestId' was rejected"
    return $response
}

function Gather-Resource([string]$token, [string]$requestId, [string]$nodeId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/resources" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; node_id = $nodeId; action = $action } | ConvertTo-Json -Compress)
}

function Use-Cache([string]$token, [string]$requestId, [string]$action, [string]$resource, [int]$amount) {
    $body = @{ request_id = $requestId; action = $action; amount = $amount }
    if (-not [string]::IsNullOrWhiteSpace($resource)) { $body.resource = $resource }
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/cache" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Compress)
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f2-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-F2 ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "temporary path escaped the system temp directory"
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8871"
$environmentNames = @("TARROWYN_SERVER_ADDR", "TARROWYN_STATE_PATH", "TARROWYN_BACKUP_PATH", "TARROWYN_MOVEMENT_COOLDOWN_TICKS", "TARROWYN_TICK_MS", "DB_DRIVER")
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$server = $null

try {
    Push-Location $projectRoot
    cargo test -p years_of_tarrowyn foundation
    if ($LASTEXITCODE -ne 0) { throw "F2 client touch-contract tests failed." }
    cargo test -p tarrowyn-server foundation
    if ($LASTEXITCODE -ne 0) { throw "F2 authority tests failed." }

    $statePath = Join-Path $temporaryRoot "foundation-state.json"
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:8871"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "250"
    $env:DB_DRIVER = "json"
    Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
    $server = Start-F2Server $statePath (Join-Path $temporaryRoot "first.out.log") (Join-Path $temporaryRoot "first.err.log")

    $guest = New-Guest "f2-connected-resource-scenario"
    $token = $guest.account_token
    $state = Read-State $token
    $tools = @($state.data.world.foundation_activity.crude_tool_access)
    Assert-F2 ($tools.Count -eq 1 -and $tools[0].available_to_all) "shared crude-tool access was not projected"
    Assert-F2 (@($tools[0].tools) -contains "hand_axe") "the shared crude axe was unavailable"
    Assert-F2 (@($tools[0].tools) -contains "stone_pick") "the shared crude pick was unavailable"

    Move-Player $token "f2-mine-north-1" 0 -1 | Out-Null
    Move-Player $token "f2-mine-north-2" 0 -1 | Out-Null
    Move-Player $token "f2-mine-east-1" 1 0 | Out-Null
    Move-Player $token "f2-mine-east-2" 1 0 | Out-Null
    $mine = Gather-Resource $token "f2-mine-once" "shallow-stone-seam-node" "mine"
    Assert-F2 $mine.data.accepted "nearby mining was rejected"
    Assert-F2 ($mine.data.player.inventory.stone -eq 2 -and $mine.data.player.inventory.iron_ore -eq 1) "mining did not award stone and iron ore"

    Move-Player $token "f2-wood-east-1" 1 0 | Out-Null
    Move-Player $token "f2-wood-east-2" 1 0 | Out-Null
    Move-Player $token "f2-wood-north-1" 0 -1 | Out-Null
    $firstLog = Gather-Resource $token "f2-log-01" "whisperwood-edge-node" "log"
    Assert-F2 $firstLog.data.accepted "nearby logging was rejected"
    for ($index = 2; $index -le 12; $index++) {
        $log = Gather-Resource $token ("f2-log-{0:d2}" -f $index) "whisperwood-edge-node" "log"
        Assert-F2 $log.data.accepted "logging stopped before the timber node depleted"
    }
    Assert-F2 ($log.data.node.deposits[0].remaining -eq 0) "the timber node did not reach deterministic depletion"
    $emptyLog = Gather-Resource $token "f2-log-empty" "whisperwood-edge-node" "log"
    Assert-F2 (-not $emptyLog.data.accepted) "a depleted timber node still granted material"
    Start-Sleep -Milliseconds 1750
    $recoveredLog = Gather-Resource $token "f2-log-recovered" "whisperwood-edge-node" "log"
    Assert-F2 $recoveredLog.data.accepted "the depleted timber node did not recover after its interval"

    foreach ($step in @(
        @{ id = "f2-cache-west-1"; dx = -1; dy = 0 },
        @{ id = "f2-cache-west-2"; dx = -1; dy = 0 },
        @{ id = "f2-cache-west-3"; dx = -1; dy = 0 },
        @{ id = "f2-cache-west-4"; dx = -1; dy = 0 },
        @{ id = "f2-cache-south-1"; dx = 0; dy = 1 },
        @{ id = "f2-cache-south-2"; dx = 0; dy = 1 },
        @{ id = "f2-cache-south-3"; dx = 0; dy = 1 }
    )) {
        Move-Player $token $step.id $step.dx $step.dy | Out-Null
    }
    $inspect = Use-Cache $token "f2-cache-inspect" "inspect" "" 0
    Assert-F2 $inspect.data.accepted "the nearby shared cache could not be inspected"
    $deposit = Use-Cache $token "f2-cache-deposit" "deposit" "timber" 2
    Assert-F2 $deposit.data.accepted "timber could not be deposited in the shared cache"
    $depositReplay = Use-Cache $token "f2-cache-deposit" "deposit" "timber" 2
    Assert-F2 (($depositReplay.data | ConvertTo-Json -Depth 20 -Compress) -eq ($deposit.data | ConvertTo-Json -Depth 20 -Compress)) "same-ID cache replay did not return the original result"
    $withdraw = Use-Cache $token "f2-cache-withdraw" "withdraw" "timber" 1
    Assert-F2 $withdraw.data.accepted "timber could not be collected from the shared cache"
    $beforeRestart = Read-State $token
    $timberBeforeRestart = $beforeRestart.data.player.inventory.timber
    $cacheBeforeRestart = $beforeRestart.data.world.foundation_activity.shared_cache.inventory.timber
    $nodeBeforeRestart = @($beforeRestart.data.world.foundation_activity.resource_nodes | Where-Object { $_.node_id -eq "whisperwood-edge-node" })[0].deposits[0].remaining

    Stop-F2Server $server
    $server = $null
    $server = Start-F2Server $statePath (Join-Path $temporaryRoot "restart.out.log") (Join-Path $temporaryRoot "restart.err.log")
    $resumed = New-Guest "f2-connected-resource-scenario"
    Assert-F2 ($resumed.character_id -eq $guest.character_id) "the connected character identity did not survive restart"
    $resumedState = Read-State $resumed.account_token
    Assert-F2 ($resumedState.data.player.inventory.timber -eq $timberBeforeRestart) "carried timber did not survive restart"
    Assert-F2 ($resumedState.data.world.foundation_activity.shared_cache.inventory.timber -eq $cacheBeforeRestart) "shared-cache timber did not survive restart"
    $resumedNode = @($resumedState.data.world.foundation_activity.resource_nodes | Where-Object { $_.node_id -eq "whisperwood-edge-node" })[0]
    Assert-F2 ($resumedNode.deposits[0].remaining -ge $nodeBeforeRestart) "resource recovery state regressed across restart"
    $replayAfterRestart = Use-Cache $resumed.account_token "f2-cache-deposit" "deposit" "timber" 2
    Assert-F2 (($replayAfterRestart.data | ConvertTo-Json -Depth 20 -Compress) -eq ($deposit.data | ConvertTo-Json -Depth 20 -Compress)) "cache replay was lost across restart"
    $afterReplay = Read-State $resumed.account_token
    Assert-F2 ($afterReplay.data.player.inventory.timber -eq $timberBeforeRestart) "replayed cache deposit changed carried inventory"
    Assert-F2 ($afterReplay.data.world.foundation_activity.shared_cache.inventory.timber -eq $cacheBeforeRestart) "replayed cache deposit changed shared inventory"
    $resourceReplay = Gather-Resource $resumed.account_token "f2-log-01" "whisperwood-edge-node" "log"
    Assert-F2 (($resourceReplay.data | ConvertTo-Json -Depth 20 -Compress) -eq ($firstLog.data | ConvertTo-Json -Depth 20 -Compress)) "gathering replay was lost across restart"
    $afterResourceReplay = Read-State $resumed.account_token
    Assert-F2 ($afterResourceReplay.data.player.inventory.timber -eq $timberBeforeRestart) "replayed logging changed carried inventory"
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-F2 ($ops.data.ready -and $ops.data.integrity_ok) "the restarted F2 fixture failed readiness checks"

    Write-Host "F2 resources passed: crude tools, mining, timber depletion/recovery, cache transfer, same-ID replay, touch contracts, persistence, and restart recovery." -ForegroundColor Green
} finally {
    Stop-F2Server $server
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
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f2-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
