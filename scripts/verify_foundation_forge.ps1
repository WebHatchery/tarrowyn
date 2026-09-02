$ErrorActionPreference = "Stop"

function Assert-F4([bool]$condition, [string]$message) {
    if (-not $condition) { throw "F4 acceptance failed: $message" }
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
        if ($process.HasExited) { throw "The F4 server exited before becoming healthy." }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The F4 server did not become healthy in time."
}

function Start-F4Server([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -WorkingDirectory $projectRoot -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process
    return $process
}

function Stop-F4Server([System.Diagnostics.Process]$process) {
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
    throw "The previous F4 server did not stop."
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
    Assert-F4 $response.data.accepted "movement '$requestId' was rejected"
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

function Use-Forge([string]$token, [string]$requestId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/foundation/forge" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; action = $action } | ConvertTo-Json -Compress)
}

function Use-Field([string]$token, [string]$requestId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/farming/actions" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; action = $action; position = @{ x = 10; y = 8 } } | ConvertTo-Json -Compress)
}

function Wait-ForNextTick([string]$token, [uint64]$tick) {
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        $state = Read-State $token
        if ([uint64]$state.meta.server_tick -gt $tick) { return $state }
        Start-Sleep -Milliseconds 25
    }
    throw "The authority did not advance to the next farming action window."
}

function Invoke-Tends([string]$token, [string]$prefix, [int]$count) {
    $accepted = 0
    for ($index = 1; $index -le $count; $index++) {
        $before = Read-State $token
        $null = Wait-ForNextTick $token ([uint64]$before.meta.server_tick)
        $result = Use-Field $token "$prefix-tend-$index" "tend"
        Assert-F4 $result.data.accepted "expected tend $index of $count to be accepted"
        $accepted++
        if (($index % 3) -eq 0 -and $index -lt $count) {
            $harvest = Use-Field $token "$prefix-harvest-$index" "harvest"
            Assert-F4 $harvest.data.accepted "mature comparison crop could not be harvested"
            $plant = Use-Field $token "$prefix-plant-$index" "plant"
            Assert-F4 $plant.data.accepted "comparison crop could not be replanted"
        }
    }
    return $accepted
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f4-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-F4 ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "temporary path escaped the system temp directory"
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8873"
$environmentNames = @(
    "TARROWYN_SERVER_ADDR",
    "TARROWYN_STATE_PATH",
    "TARROWYN_BACKUP_PATH",
    "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
    "TARROWYN_TICK_MS",
    "TARROWYN_WORLD_SECONDS_PER_TICK",
    "TARROWYN_CROP_STAGE_SECONDS",
    "DB_DRIVER"
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
$server = $null

try {
    Push-Location $projectRoot
    cargo test -p years_of_tarrowyn ui_foundation
    if ($LASTEXITCODE -ne 0) { throw "F4 touch-forge tests failed." }
    cargo test -p tarrowyn-server foundation::forge
    if ($LASTEXITCODE -ne 0) { throw "F4 forge-authority tests failed." }

    $statePath = Join-Path $temporaryRoot "forge-state.json"
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:8873"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_WORLD_SECONDS_PER_TICK = "1"
    $env:TARROWYN_CROP_STAGE_SECONDS = "3600"
    $env:DB_DRIVER = "json"
    Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
    $server = Start-F4Server $statePath (Join-Path $temporaryRoot "first.out.log") (Join-Path $temporaryRoot "first.err.log")

    $smith = New-Guest "f4-connected-smith"
    $smithToken = $smith.account_token
    Move-Path $smithToken "smith-to-mine" @(@(1, 0), @(1, 0), @(0, -1), @(0, -1))
    $mineOne = Use-Resource $smithToken "f4-mine-1" "shallow-stone-seam-node" "mine"
    $mineTwo = Use-Resource $smithToken "f4-mine-2" "shallow-stone-seam-node" "mine"
    Assert-F4 ($mineOne.data.accepted -and $mineTwo.data.accepted) "two mining actions did not supply forge ore"
    Assert-F4 ($mineTwo.data.player.inventory.iron_ore -eq 2) "mining did not supply exactly two iron ore"

    Move-Path $smithToken "smith-to-wood" @(@(1, 0), @(1, 0), @(0, -1))
    $log = Use-Resource $smithToken "f4-log-1" "whisperwood-edge-node" "log"
    Assert-F4 $log.data.accepted "logging did not supply forge timber"
    Assert-F4 ($log.data.player.inventory.timber -eq 2) "logging did not supply exactly two timber"
    Move-Path $smithToken "smith-to-forge" @(@(-1, 0), @(-1, 0), @(0, 1))

    $inspect = Use-Forge $smithToken "f4-inspect" "inspect"
    Assert-F4 ($inspect.data.accepted -and $inspect.data.forge.recipes.Count -eq 3) "forge needs and three recipes were not inspectable"
    Assert-F4 ($inspect.data.forge.crude_tool_action_capacity -eq 3) "crude capacity was not projected as three"
    Assert-F4 ($inspect.data.forge.improved_tool_action_capacity -eq 6) "iron capacity was not projected as six"

    $charcoal = Use-Forge $smithToken "f4-burn-charcoal" "burn_charcoal"
    $charcoalReplay = Use-Forge $smithToken "f4-burn-charcoal" "burn_charcoal"
    Assert-F4 $charcoal.data.accepted "charcoal preparation was rejected"
    Assert-F4 (($charcoalReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($charcoal.data | ConvertTo-Json -Depth 30 -Compress)) "charcoal retry did not return its original result"
    Assert-F4 ($charcoal.data.player.inventory.timber -eq 1 -and $charcoal.data.player.inventory.charcoal -eq 1) "charcoal preparation did not consume and produce exactly once"

    $handle = Use-Forge $smithToken "f4-shape-handle" "shape_handle"
    $handleReplay = Use-Forge $smithToken "f4-shape-handle" "shape_handle"
    Assert-F4 $handle.data.accepted "handle preparation was rejected"
    Assert-F4 (($handleReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($handle.data | ConvertTo-Json -Depth 30 -Compress)) "handle retry did not return its original result"
    Assert-F4 ($handle.data.player.inventory.timber -eq 0 -and $handle.data.player.inventory.tool_handles -eq 1) "handle preparation did not consume and produce exactly once"

    $forged = Use-Forge $smithToken "f4-forge-tool" "forge_field_tool"
    $forgeReplay = Use-Forge $smithToken "f4-forge-tool" "forge_field_tool"
    Assert-F4 $forged.data.accepted "iron field tool was not forged"
    Assert-F4 (($forgeReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($forged.data | ConvertTo-Json -Depth 30 -Compress)) "tool retry did not return its original result"
    Assert-F4 ($forged.data.player.inventory.iron_ore -eq 0 -and $forged.data.player.inventory.charcoal -eq 0 -and $forged.data.player.inventory.tool_handles -eq 0) "tool recipe did not consume exact gathered inputs"
    Assert-F4 ($forged.data.player.field_tool_kind -eq "iron" -and $forged.data.player.field_tool_condition -eq 6) "forge did not return a six-action iron tool"

    $crude = New-Guest "f4-connected-crude"
    Move-Path $crude.account_token "crude-to-field" @(@(1, 0), @(1, 0), @(0, 1))
    $crudePlant = Use-Field $crude.account_token "f4-crude-plant" "plant"
    Assert-F4 $crudePlant.data.accepted "crude comparison crop could not be planted"
    $crudeAccepted = Invoke-Tends $crude.account_token "f4-crude" 3
    $crudeHarvest = Use-Field $crude.account_token "f4-crude-harvest" "harvest"
    Assert-F4 $crudeHarvest.data.accepted "crude comparison crop could not be harvested"
    $crudeReplant = Use-Field $crude.account_token "f4-crude-replant" "plant"
    Assert-F4 $crudeReplant.data.accepted "crude exhaustion crop could not be planted"
    $crudeRejected = Use-Field $crude.account_token "f4-crude-tend-4" "tend"
    Assert-F4 (-not $crudeRejected.data.accepted) "crude fallback exceeded three useful actions"

    Move-Path $smithToken "smith-to-field" @(@(0, 1), @(0, 1), @(0, 1), @(0, 1))
    $ironAccepted = Invoke-Tends $smithToken "f4-iron" 6
    $ironHarvest = Use-Field $smithToken "f4-iron-harvest" "harvest"
    Assert-F4 $ironHarvest.data.accepted "iron comparison crop could not be harvested"
    $ironReplant = Use-Field $smithToken "f4-iron-replant" "plant"
    Assert-F4 $ironReplant.data.accepted "iron exhaustion crop could not be planted"
    $ironRejected = Use-Field $smithToken "f4-iron-tend-7" "tend"
    Assert-F4 (-not $ironRejected.data.accepted) "iron tool exceeded six useful actions"
    Assert-F4 ($crudeAccepted -eq 3 -and $ironAccepted -eq 6) "fixed comparison was not exactly three versus six actions"

    Stop-F4Server $server
    $server = $null
    $server = Start-F4Server $statePath (Join-Path $temporaryRoot "restart.out.log") (Join-Path $temporaryRoot "restart.err.log")
    $restarted = New-Guest "f4-connected-smith"
    Assert-F4 ($restarted.character_id -eq $smith.character_id) "smith identity did not survive restart"
    $restartedState = Read-State $restarted.account_token
    Assert-F4 ($restartedState.data.player.field_tool_kind -eq "iron" -and $restartedState.data.player.field_tool_condition -eq 0) "used iron tool state did not survive restart"
    $restartReplay = Use-Forge $restarted.account_token "f4-forge-tool" "forge_field_tool"
    Assert-F4 (($restartReplay.data | ConvertTo-Json -Depth 30 -Compress) -eq ($forged.data | ConvertTo-Json -Depth 30 -Compress)) "forge replay was lost across restart"
    $afterReplay = Read-State $restarted.account_token
    Assert-F4 ($afterReplay.data.player.field_tool_condition -eq 0) "replayed forge restored consumed condition"
    Assert-F4 ($afterReplay.data.player.inventory.iron_ore -eq 0 -and $afterReplay.data.player.inventory.charcoal -eq 0 -and $afterReplay.data.player.inventory.tool_handles -eq 0) "replayed forge duplicated materials"
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-F4 ($ops.data.ready -and $ops.data.integrity_ok) "the restarted F4 fixture failed readiness checks"

    Write-Host "F4 forge passed: gathered inputs, fuel/component preparation, typed needs, exact recipe costs, 3-vs-6 useful actions, retries, restart persistence, and crude fallback." -ForegroundColor Green
} finally {
    Stop-F4Server $server
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
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f4-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
