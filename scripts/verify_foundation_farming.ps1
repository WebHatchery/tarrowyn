$ErrorActionPreference = "Stop"

function Assert-F3([bool]$condition, [string]$message) {
    if (-not $condition) { throw "F3 acceptance failed: $message" }
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
        if ($process.HasExited) { throw "The F3 server exited before becoming healthy." }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/health" -TimeoutSec 2
            if ($health.data.status -eq "ok" -and $health.data.protocol_version -eq "7") { return }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    throw "The F3 server did not become healthy in time."
}

function Start-F3Server([string]$statePath, [string]$stdoutPath, [string]$stderrPath) {
    $env:TARROWYN_STATE_PATH = $statePath
    $process = Start-Process -FilePath "cargo.exe" -ArgumentList @("run", "-q", "-p", "tarrowyn-server") `
        -WorkingDirectory $projectRoot -PassThru -WindowStyle Hidden `
        -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    Wait-ForHealth $process
    return $process
}

function Stop-F3Server([System.Diagnostics.Process]$process) {
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
    throw "The previous F3 server did not stop."
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
    Assert-F3 $response.data.accepted "movement '$requestId' was rejected"
}

function Use-Field([string]$token, [string]$requestId, [string]$action) {
    return Invoke-RestMethod -Method Post -Uri "$script:baseUrl/v1/farming/actions" `
        -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
        -Body (@{ request_id = $requestId; action = $action; position = @{ x = 10; y = 8 } } | ConvertTo-Json -Compress)
}

function Find-ScenarioPlot($state) {
    return @($state.data.world.plots | Where-Object { $_.position.x -eq 10 -and $_.position.y -eq 8 })[0]
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryRoot = [System.IO.Path]::GetFullPath((Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-f3-" + [guid]::NewGuid().ToString("N"))))
$expectedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
Assert-F3 ($temporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)) "temporary path escaped the system temp directory"
$null = New-Item -ItemType Directory -Path $temporaryRoot
$script:baseUrl = "http://127.0.0.1:8872"
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
    if ($LASTEXITCODE -ne 0) { throw "F3 client touch/outlook tests failed." }
    cargo test -p tarrowyn-server farming
    if ($LASTEXITCODE -ne 0) { throw "F3 farming authority tests failed." }
    cargo test -p tarrowyn-server offline_crop_growth
    if ($LASTEXITCODE -ne 0) { throw "F3 offline-growth tests failed." }

    $statePath = Join-Path $temporaryRoot "farming-state.json"
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:8872"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "1000"
    $env:TARROWYN_WORLD_SECONDS_PER_TICK = "1"
    $env:TARROWYN_CROP_STAGE_SECONDS = "300"
    $env:DB_DRIVER = "json"
    Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue
    $server = Start-F3Server $statePath (Join-Path $temporaryRoot "first.out.log") (Join-Path $temporaryRoot "first.err.log")

    $guest = New-Guest "f3-connected-farming-scenario"
    $token = $guest.account_token
    $initial = Read-State $token
    Assert-F3 ($initial.data.world.plots.Count -eq 3) "the three shared plots were not projected"
    Assert-F3 ($initial.data.player.field_tool_condition -eq 3) "field-tool condition was not readable"
    Assert-F3 ($null -ne $initial.data.player.field_weather) "field weather was not readable"
    Assert-F3 ($null -ne $initial.data.player.field_pest_pressure) "pest pressure was not readable"

    foreach ($step in @(
        @{ id = "f3-east-1"; dx = 1; dy = 0 },
        @{ id = "f3-east-2"; dx = 1; dy = 0 },
        @{ id = "f3-south-1"; dx = 0; dy = 1 },
        @{ id = "f3-south-2"; dx = 0; dy = 1 }
    )) {
        Move-Player $token $step.id $step.dx $step.dy
    }

    $seedsBeforePlant = $initial.data.player.inventory.seeds
    $plant = Use-Field $token "f3-plant-before-away" "plant"
    Assert-F3 $plant.data.accepted "planting was rejected"
    Assert-F3 ($plant.data.plot.crop.stage -eq 0) "the new crop did not begin at stage zero"
    Assert-F3 ($plant.data.player.inventory.seeds -eq ($seedsBeforePlant - 1)) "planting did not consume exactly one seed"
    $inspected = Read-State $token
    $inspectedPlot = Find-ScenarioPlot $inspected
    Assert-F3 ($inspectedPlot.crop.stage -eq 0) "the planted crop was absent from state inspection"
    Assert-F3 ($null -eq $inspectedPlot.crop.last_tended_tick) "the untouched crop was unexpectedly maintained"

    Stop-F3Server $server
    $server = $null
    $stored = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    Assert-F3 ($stored.storage_version -eq 22) "the scenario did not persist the current storage contract"
    $stored.persisted_at_unix_millis = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - (15 * 60 * 1000)
    $stored | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $statePath -Encoding utf8

    $server = Start-F3Server $statePath (Join-Path $temporaryRoot "offline.out.log") (Join-Path $temporaryRoot "offline.err.log")
    $resumed = New-Guest "f3-connected-farming-scenario"
    Assert-F3 ($resumed.character_id -eq $guest.character_id) "the farmer identity did not survive the offline interval"
    $token = $resumed.account_token
    $afterAway = Read-State $token
    $maturePlot = Find-ScenarioPlot $afterAway
    Assert-F3 ($maturePlot.crop.stage -eq 3) "15 offline minutes did not mature the crop"
    Assert-F3 ($maturePlot.crop.growth_ticks -ge 900) "offline growth did not record the modeled interval"
    Assert-F3 ($null -eq $maturePlot.crop.last_tended_tick) "maturity incorrectly required repetitive tending"

    $wheatBeforeHarvest = $afterAway.data.player.inventory.wheat
    $harvest = Use-Field $token "f3-harvest-after-away" "harvest"
    $harvestReplay = Use-Field $token "f3-harvest-after-away" "harvest"
    Assert-F3 $harvest.data.accepted "the offline-grown crop could not be harvested"
    Assert-F3 (($harvestReplay.data | ConvertTo-Json -Depth 20 -Compress) -eq ($harvest.data | ConvertTo-Json -Depth 20 -Compress)) "harvest retry did not return the original result"
    Assert-F3 ($harvest.data.player.inventory.wheat -eq ($wheatBeforeHarvest + 1)) "harvest did not award exactly one wheat"

    $replant = Use-Field $token "f3-replant" "plant"
    Assert-F3 $replant.data.accepted "the harvested plot could not be replanted"
    $tend = Use-Field $token "f3-optional-tend" "tend"
    $tendReplay = Use-Field $token "f3-optional-tend" "tend"
    Assert-F3 $tend.data.accepted "optional tend/water action was rejected"
    Assert-F3 ($tend.data.plot.crop.stage -eq 1 -and $tend.data.plot.crop.quality -eq 2) "tending did not visibly improve the young crop"
    Assert-F3 ($tend.data.player.field_tool_condition -eq 2) "tending did not expose its tool-maintenance cost"
    Assert-F3 (($tendReplay.data | ConvertTo-Json -Depth 20 -Compress) -eq ($tend.data | ConvertTo-Json -Depth 20 -Compress)) "tend retry did not return the original result"

    Stop-F3Server $server
    $server = $null
    $server = Start-F3Server $statePath (Join-Path $temporaryRoot "restart.out.log") (Join-Path $temporaryRoot "restart.err.log")
    $restarted = New-Guest "f3-connected-farming-scenario"
    Assert-F3 ($restarted.character_id -eq $guest.character_id) "the farmer identity changed after restart"
    $restartedState = Read-State $restarted.account_token
    $restartedPlot = Find-ScenarioPlot $restartedState
    Assert-F3 ($restartedPlot.crop.stage -ge 1 -and $null -ne $restartedPlot.crop.last_tended_tick) "replanted and tended crop state did not survive restart"
    Assert-F3 ($restartedState.data.player.field_tool_condition -eq 2) "field-tool condition did not survive restart"
    $tendAfterRestart = Use-Field $restarted.account_token "f3-optional-tend" "tend"
    Assert-F3 (($tendAfterRestart.data | ConvertTo-Json -Depth 20 -Compress) -eq ($tend.data | ConvertTo-Json -Depth 20 -Compress)) "tend replay was lost across restart"
    $afterReplay = Read-State $restarted.account_token
    Assert-F3 ($afterReplay.data.player.field_tool_condition -eq 2) "replayed tending consumed tool condition"
    $ops = Invoke-RestMethod -Method Get -Uri "$script:baseUrl/v1/ops/health"
    Assert-F3 ($ops.data.ready -and $ops.data.integrity_ok) "the restarted F3 fixture failed readiness checks"

    Write-Host "F3 farming passed: touch outlook, 15-minute offline growth, untended maturity, harvest, replant, optional tend/water, replay, persistence, and restart." -ForegroundColor Green
} finally {
    Stop-F3Server $server
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
    if ($resolvedTemporaryRoot.StartsWith($expectedTempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolvedTemporaryRoot).StartsWith("tarrowyn-f3-")) {
        Remove-Item -LiteralPath $resolvedTemporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
