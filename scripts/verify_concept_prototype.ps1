$ErrorActionPreference = "Stop"

$projectRoot = Split-Path $PSScriptRoot -Parent
$statePath = Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-concept-prototype-" + [guid]::NewGuid().ToString("N") + ".json")
$server = $null
$environmentNames = @(
    "DB_DRIVER",
    "TARROWYN_STATE_PATH",
    "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
    "TARROWYN_TICK_MS",
    "TARROWYN_HTTP_REQUEST_WORKERS",
    "TARROWYN_HTTP_QUEUE_CAPACITY",
    "TARROWYN_WORLD_SECONDS_PER_TICK",
    "TARROWYN_CROP_STAGE_SECONDS"
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) {
        throw "Concept prototype check failed: $message"
    }
}

function Start-ConceptServer {
    Start-Process -FilePath "cargo.exe" `
        -ArgumentList @("run", "-p", "tarrowyn-server", "--quiet") `
        -WorkingDirectory $projectRoot `
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

function Stop-ConceptServer([System.Diagnostics.Process]$process) {
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
    throw "Concept prototype check failed: previous server did not stop"
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
    throw "Concept prototype check failed: server did not become healthy"
}

function Post-Json([string]$path, [hashtable]$body, [hashtable]$headers) {
    $request = @{
        Method = "Post"
        Uri = "http://127.0.0.1:8787$path"
        ContentType = "application/json"
        Body = ($body | ConvertTo-Json -Compress)
    }
    if ($null -ne $headers) { $request.Headers = $headers }
    Invoke-RestMethod @request
}

function Get-Json([string]$path, [hashtable]$headers) {
    Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787$path" -Headers $headers
}

function Wait-TravelArrival([string]$token) {
    $headers = @{ Authorization = "Bearer $token" }
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        $region = Get-Json "/v1/region" $headers
        if ($region.data.player_location_id -eq "saltmere" -and $region.data.travel.status -eq "arrived") {
            return $region
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Concept prototype check failed: wayfarer did not complete the shared journey"
}

try {
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
    $env:DB_DRIVER = "json"
    $env:TARROWYN_STATE_PATH = $statePath
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_HTTP_REQUEST_WORKERS = "0"
    $env:TARROWYN_HTTP_QUEUE_CAPACITY = "128"
    $env:TARROWYN_WORLD_SECONDS_PER_TICK = "1"
    $env:TARROWYN_CROP_STAGE_SECONDS = "1"

    $server = Start-ConceptServer
    Wait-Healthy

    $sessions = @()
    foreach ($key in @("concept-farmer", "concept-adventurer", "concept-wayfarer")) {
        $sessions += Post-Json "/v1/session/guest" `
            @{ client_key = $key; reset = $true } $null
    }
    $headers = @($sessions | ForEach-Object { @{ Authorization = "Bearer $($_.data.account_token)" } })
    $sharedState = Get-Json "/v1/state" $headers[0]
    Assert-True ($sharedState.data.world.players.Count -eq 3) "three role players did not share one presence projection"
    Assert-True ($sharedState.data.world.plots.Count -eq 3) "the shared farming area was not visible to the roles"

    $farmerSteps = @(@{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 0; dy = 1 }, @{ dx = 0; dy = 1 })
    for ($index = 0; $index -lt $farmerSteps.Count; $index++) {
        $move = Post-Json "/v1/movement" `
            @{ request_id = "concept-farmer-step-$index"; dx = $farmerSteps[$index].dx; dy = $farmerSteps[$index].dy } $headers[0]
        Assert-True $move.data.accepted "the farmer could not reach the shared fields"
    }
    $plant = Post-Json "/v1/farming/actions" `
        @{ request_id = "concept-farmer-plant"; action = "plant"; position = @{ x = 10; y = 8 } } $headers[0]
    Assert-True $plant.data.accepted "the farmer could not plant the shared crop"
    Start-Sleep -Milliseconds 300
    $harvest = Post-Json "/v1/farming/actions" `
        @{ request_id = "concept-farmer-harvest"; action = "harvest"; position = @{ x = 10; y = 8 } } $headers[0]
    Assert-True $harvest.data.accepted "the shared clock did not mature the farmer's crop"

    $trade = Post-Json "/v1/trades" `
        @{ request_id = "concept-farmer-trade"; action = "create"; recipient_account_id = $sessions[1].data.account_id; offer = @{ seeds = 1; wheat = 0; turnips = 0; moonberries = 0; gold = 0 }; request = @{ seeds = 0; wheat = 0; turnips = 0; moonberries = 0; gold = 2 } } $headers[0]
    Assert-True $trade.data.accepted "the farmer could not offer useful harvest supplies"
    $tradeId = $trade.data.trade.trade_id
    $acceptedTrade = Post-Json "/v1/trades" `
        @{ request_id = "concept-adventurer-trade"; action = "accept"; trade_id = $tradeId } $headers[1]
    Assert-True ($acceptedTrade.data.trade.status -eq "accepted") "the adventurer could not complete the farmer's exchange"

    $chat = Post-Json "/v1/chat" `
        @{ request_id = "concept-wayfarer-tavern"; channel = "tavern"; text = "Meet at the Hearth before the road." } $headers[2]
    Assert-True $chat.data.accepted "the wayfarer could not share a tavern plan"
    $feed = Get-Json "/v1/tavern/feed" $headers[0]
    Assert-True (@($feed.data.chat | Where-Object { $_.text -eq "Meet at the Hearth before the road." }).Count -eq 1) "the role players did not share the tavern conversation"

    $contracts = Get-Json "/v1/contracts" $headers[1]
    Assert-True ($contracts.data.contracts.Count -eq 1) "the adventurer could not see the repeatable contract"
    $acceptedContract = Post-Json "/v1/contracts/brambleback-watch" `
        @{ request_id = "concept-adventurer-contract"; action = "accept"; contract_id = "brambleback-watch" } $headers[1]
    Assert-True $acceptedContract.data.accepted "the adventurer could not accept the shared contract"
    $adventurerSteps = @(@{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 0; dy = -1 }, @{ dx = 0; dy = -1 })
    for ($index = 0; $index -lt $adventurerSteps.Count; $index++) {
        $move = Post-Json "/v1/movement" `
            @{ request_id = "concept-adventurer-step-$index"; dx = $adventurerSteps[$index].dx; dy = $adventurerSteps[$index].dy } $headers[1]
        Assert-True $move.data.accepted "the adventurer could not reach the contract site"
    }
    for ($index = 0; $index -lt 3; $index++) {
        $progress = Post-Json "/v1/contracts/brambleback-watch" `
            @{ request_id = "concept-adventurer-progress-$index"; action = "progress"; contract_id = "brambleback-watch" } $headers[1]
        Assert-True $progress.data.accepted "the adventurer's contract did not progress"
    }
    $reported = Post-Json "/v1/contracts/brambleback-watch" `
        @{ request_id = "concept-adventurer-report"; action = "report"; contract_id = "brambleback-watch" } $headers[1]
    Assert-True $reported.data.accepted "the adventurer could not report the community contract"

    $journey = Post-Json "/v1/travel" `
        @{ request_id = "concept-wayfarer-journey"; action = "start"; route_id = "saltmere-ferry" } $headers[2]
    Assert-True $journey.data.accepted "the wayfarer could not begin the shared regional journey"
    $arrival = Wait-TravelArrival $sessions[2].data.account_token
    Assert-True ($arrival.data.travel.status -eq "arrived") "the shared regional journey did not arrive"

    Write-Host "Concept prototype acceptance passed: farmer, adventurer, and wayfarer shared farming, exchange, tavern coordination, repeatable contract, and regional travel." -ForegroundColor Green
} finally {
    try {
        if ($null -ne $server -and -not $server.HasExited) { Stop-ConceptServer $server }
    } finally {
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
}
