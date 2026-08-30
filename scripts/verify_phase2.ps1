$ErrorActionPreference = "Stop"

$gameDir = Split-Path $PSScriptRoot -Parent
$statePath = Join-Path ([System.IO.Path]::GetTempPath()) "tarrowyn-phase2-$PID.json"
$server = $null
$environmentNames = @(
    "DB_DRIVER",
    "TARROWYN_STATE_PATH",
    "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
    "TARROWYN_TICK_MS",
    "TARROWYN_WORLD_SECONDS_PER_TICK",
    "TARROWYN_CROP_STAGE_SECONDS"
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) {
        throw "Phase 2 acceptance failed: $message"
    }
}

function Start-Phase2Server {
    Start-Process -FilePath "cargo.exe" `
        -ArgumentList @("run", "-p", "tarrowyn-server", "--quiet") `
        -WorkingDirectory $gameDir `
        -WindowStyle Hidden `
        -PassThru
}

function Get-DescendantProcessIds([int]$parentId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId")
    foreach ($child in $children) {
        Get-DescendantProcessIds $child.ProcessId
        $child.ProcessId
    }
}

function Stop-Phase2Server([System.Diagnostics.Process]$process) {
    if ($null -eq $process) { return }
    $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
            Start-Sleep -Milliseconds 100
        } catch {
            return
        }
    }
    throw "Phase 2 acceptance failed: previous server did not stop"
}

function Wait-Healthy {
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
            if ($health.data.status -eq "ok") { return }
        } catch {
            Start-Sleep -Milliseconds 100
        }
    }
    throw "Phase 2 acceptance failed: server did not become healthy"
}

try {
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
    $env:DB_DRIVER = "json"
    $env:TARROWYN_STATE_PATH = $statePath
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_WORLD_SECONDS_PER_TICK = "1"
    $env:TARROWYN_CROP_STAGE_SECONDS = "1"

    $server = Start-Phase2Server
    Wait-Healthy

    $sessions = @()
    foreach ($key in @("phase2-farmer", "phase2-trader", "phase2-tavern")) {
        $sessions += Invoke-RestMethod -Method Post `
            -Uri "http://127.0.0.1:8787/v1/session/guest" `
            -ContentType "application/json" `
            -Body (@{ client_key = $key; reset = $true } | ConvertTo-Json -Compress)
    }
    $headers = @($sessions | ForEach-Object { @{ Authorization = "Bearer $($_.data.account_token)" } })

    $state = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $headers[0]
    Assert-True ($state.data.world.plots.Count -eq 3) "shared farm plots were not in the state projection"
    Assert-True ($state.data.feed.notices.Count -ge 1) "the tavern notice feed was empty"

    $steps = @(@{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 0; dy = 1 }, @{ dx = 0; dy = 1 })
    for ($index = 0; $index -lt $steps.Count; $index++) {
        $body = @{ request_id = "phase2-step-$index"; dx = $steps[$index].dx; dy = $steps[$index].dy } | ConvertTo-Json -Compress
        $move = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/movement" -Headers $headers[0] -ContentType "application/json" -Body $body
        Assert-True $move.data.accepted "farmer could not reach the shared fields"
    }

    $plantBody = @{ request_id = "phase2-plant"; action = "plant"; position = @{ x = 10; y = 8 } } | ConvertTo-Json -Compress
    $plant = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/farming/actions" -Headers $headers[0] -ContentType "application/json" -Body $plantBody
    $plantRetry = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/farming/actions" -Headers $headers[0] -ContentType "application/json" -Body $plantBody
    Assert-True $plant.data.accepted "plant action was rejected"
    Assert-True ($plantRetry.data.player.inventory.seeds -eq $plant.data.player.inventory.seeds) "plant retry consumed a second seed"

    Start-Sleep -Milliseconds 300
    $harvestBody = @{ request_id = "phase2-harvest"; action = "harvest"; position = @{ x = 10; y = 8 } } | ConvertTo-Json -Compress
    $harvest = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/farming/actions" -Headers $headers[0] -ContentType "application/json" -Body $harvestBody
    Assert-True $harvest.data.accepted "shared clock did not mature the crop"

    $tradeBody = @{ request_id = "phase2-offer"; action = "create"; recipient_account_id = $sessions[1].data.account_id; offer = @{ seeds = 1; wheat = 0; turnips = 0; moonberries = 0; gold = 0 }; request = @{ seeds = 0; wheat = 0; turnips = 0; moonberries = 0; gold = 2 } } | ConvertTo-Json -Compress
    $trade = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/trades" -Headers $headers[0] -ContentType "application/json" -Body $tradeBody
    Assert-True $trade.data.accepted "trade offer was rejected"
    $tradeId = $trade.data.trade.trade_id
    $acceptBody = @{ request_id = "phase2-accept"; action = "accept"; trade_id = $tradeId } | ConvertTo-Json -Compress
    $accepted = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/trades" -Headers $headers[1] -ContentType "application/json" -Body $acceptBody
    $acceptedRetry = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/trades" -Headers $headers[1] -ContentType "application/json" -Body $acceptBody
    Assert-True ($accepted.data.trade.status -eq "accepted") "trade was not completed"
    Assert-True ($acceptedRetry.data.trade.status -eq "accepted") "trade retry did not return the completed exchange"

    $chatBody = @{ request_id = "phase2-rumour"; channel = "tavern"; text = "The fields are growing." } | ConvertTo-Json -Compress
    $null = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/chat" -Headers $headers[2] -ContentType "application/json" -Body $chatBody
    $feed = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/tavern/feed" -Headers $headers[0]
    Assert-True (@($feed.data.chat | Where-Object { $_.text -eq "The fields are growing." }).Count -eq 1) "tavern chat did not cross players"

    $completedTrade = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/trades" -Headers $headers[1]
    Assert-True (@($completedTrade.data.trades | Where-Object { $_.trade_id -eq $tradeId -and $_.status -eq "accepted" }).Count -eq 1) "completed trade was not visible"
    $beforeRestartTick = $state.meta.server_tick

    Stop-Phase2Server $server
    $server = Start-Phase2Server
    Wait-Healthy
    $resumed = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/session/guest" -ContentType "application/json" -Body (@{ client_key = "phase2-trader"; reset = $false } | ConvertTo-Json -Compress)
    Assert-True ($resumed.data.character_id -eq $sessions[1].data.character_id) "character identity did not survive restart"
    $resumedHeaders = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $afterRestart = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $resumedHeaders
    Assert-True ($afterRestart.meta.server_tick -ge $beforeRestartTick) "world clock did not survive restart"
    Write-Host "Phase 2 acceptance passed: persistent farming, idempotent exchange, tavern feed, and restart recovery." -ForegroundColor Green
} finally {
    if ($null -ne $server -and -not $server.HasExited) { Stop-Phase2Server $server }
    foreach ($name in $environmentNames) {
        $value = $previousEnvironment[$name]
        if ($null -eq $value) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            Set-Item -Path "Env:$name" -Value $value
        }
    }
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
}
