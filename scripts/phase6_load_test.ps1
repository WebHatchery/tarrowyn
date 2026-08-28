param(
    [int]$ClientCount = 24,
    [int]$Rounds = 3,
    [string]$ServerAddress = "127.0.0.1:8799",
    [string[]]$AllowedAlertFlags = @()
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$runId = [guid]::NewGuid().ToString("N")
$statePath = Join-Path ([System.IO.Path]::GetTempPath()) "tarrowyn-phase6-load-$runId.json"
$backupPath = Join-Path ([System.IO.Path]::GetTempPath()) "tarrowyn-phase6-load-$runId.backup.json"
$server = $null
$environmentNames = @(
    "TARROWYN_SERVER_ADDR",
    "TARROWYN_STATE_PATH",
    "TARROWYN_BACKUP_PATH",
    "TARROWYN_BACKUP_INTERVAL_TICKS",
    "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
    "TARROWYN_TICK_MS",
    "TARROWYN_SESSION_TTL_SECONDS",
    "TARROWYN_SUPPORT_OPERATOR_ACCOUNTS"
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) { throw "Phase 6 load test failed: $message" }
}

function Start-Phase6Server {
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

function Stop-Phase6Server([System.Diagnostics.Process]$process) {
    if ($null -eq $process) { return }
    $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/ops/health"
            Start-Sleep -Milliseconds 100
        } catch { return }
    }
    throw "Phase 6 load test failed: previous server did not stop"
}

function Wait-Healthy {
    for ($attempt = 0; $attempt -lt 80; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/ops/health"
            if ($health.data.ready -and $health.data.integrity_ok) { return $health }
        } catch { }
        Start-Sleep -Milliseconds 250
    }
    throw "Phase 6 load test failed: server did not become ready"
}

function New-GuestSession([string]$clientKey, [bool]$reset) {
    Invoke-RestMethod -Method Post -Uri "http://$ServerAddress/v1/session/guest" `
        -ContentType "application/json" `
        -Body (@{ client_key = $clientKey; reset = $reset } | ConvertTo-Json -Compress)
}

function Invoke-PostJson([string]$path, [hashtable]$body, [hashtable]$headers) {
    Invoke-RestMethod -Method Post -Uri "http://$ServerAddress$path" `
        -Headers $headers -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Compress)
}

function Invoke-MixedClientLoad {
    param(
        [string]$Token,
        [int]$ClientIndex,
        [int]$ExpectedClients,
        [int]$RoundCount,
        [string]$Address
    )

    $headers = @{ Authorization = "Bearer $Token" }
    $requestCount = 0
    $acceptedCommands = 0
    $rejectedCommands = 0
    try {
        for ($round = 0; $round -lt $RoundCount; $round++) {
            $state = Invoke-RestMethod -Method Get -Uri "http://$Address/v1/state" -Headers $headers
            $requestCount++
            if ($state.data.world.players.Count -lt $ExpectedClients) {
                throw "only $($state.data.world.players.Count) of $ExpectedClients players were visible"
            }

            $events = Invoke-RestMethod -Method Get -Uri "http://$Address/v1/events?since=0" -Headers $headers
            $regionalEvents = Invoke-RestMethod -Method Get -Uri "http://$Address/v1/events/region?since=0" -Headers $headers
            $region = Invoke-RestMethod -Method Get -Uri "http://$Address/v1/region" -Headers $headers
            $market = Invoke-RestMethod -Method Get -Uri "http://$Address/v1/market/orders" -Headers $headers
            $requestCount += 4
            if ($null -eq $events.data.cursor -or $null -eq $regionalEvents.data.cursor `
                -or $null -eq $region.data.cursor -or $null -eq $market.data.cursor) {
                throw "a cursorable regional projection was incomplete"
            }

            $move = Invoke-PostJson "/v1/movement" `
                @{ request_id = "phase6-load-$ClientIndex-move-$round"; dx = 1; dy = 0 } $headers $Address
            $chat = Invoke-PostJson "/v1/chat" `
                @{ request_id = "phase6-load-$ClientIndex-chat-$round"; channel = "settlement"; text = "load $ClientIndex/$round" } $headers $Address
            $requestCount += 2
            if ($null -eq $move.data.accepted -or $null -eq $chat.data.accepted) {
                throw "movement or chat returned no authoritative outcome"
            }
            if ($move.data.accepted) { $acceptedCommands++ } else { $rejectedCommands++ }
            if ($chat.data.accepted) { $acceptedCommands++ } else { $rejectedCommands++ }

            $invalidMove = Invoke-PostJson "/v1/movement" `
                @{ request_id = "phase6-load-$ClientIndex-invalid-move-$round"; dx = 0; dy = 0 } $headers $Address
            $requestCount++
            if ($null -eq $invalidMove.data.accepted -or $invalidMove.data.accepted) {
                throw "the invalid movement probe did not return a rejected authoritative outcome"
            }
            $rejectedCommands++

            if ($round -eq 0) {
                $order = Invoke-PostJson "/v1/market/orders" `
                    @{ request_id = "phase6-load-$ClientIndex-order"; action = "create"; destination_location_id = "saltmere"; commodity = "seeds"; quantity = 1 } $headers $Address
                $travel = Invoke-PostJson "/v1/travel" `
                    @{ request_id = "phase6-load-$ClientIndex-travel"; action = "start"; route_id = "saltmere-ferry" } $headers $Address
                $requestCount += 2
                if ($null -eq $order.data.accepted -or $null -eq $travel.data.accepted) {
                    throw "market or travel returned no authoritative outcome"
                }
                if ($order.data.accepted) { $acceptedCommands++ } else { $rejectedCommands++ }
                if ($travel.data.accepted) { $acceptedCommands++ } else { $rejectedCommands++ }
            }
        }
        [pscustomobject]@{
            passed = $true
            client = $ClientIndex
            requests = $requestCount
            accepted = $acceptedCommands
            rejected = $rejectedCommands
        }
    } catch {
        [pscustomobject]@{
            passed = $false
            client = $ClientIndex
            requests = $requestCount
            accepted = $acceptedCommands
            rejected = $rejectedCommands
            error = $_.Exception.Message
        }
    }
}

function Invoke-ConcurrentMixedLoad([object[]]$sessions) {
    $jobScript = ${function:Invoke-MixedClientLoad}.ToString()
    $jobHelper = @'
function Invoke-PostJson([string]$path, [hashtable]$body, [hashtable]$headers, [string]$address) {
    Invoke-RestMethod -Method Post -Uri "http://$address$path" `
        -Headers $headers -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Compress)
}
'@
    $jobFunction = $jobHelper + "`nfunction Invoke-MixedClientLoad {`n$jobScript`n}"
    $jobs = @()
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        for ($index = 0; $index -lt $sessions.Count; $index++) {
            $jobs += Start-Job -ScriptBlock ([scriptblock]::Create($jobFunction + "`nInvoke-MixedClientLoad @args")) -ArgumentList @(
                $sessions[$index].data.account_token,
                $index,
                $sessions.Count,
                $Rounds,
                $ServerAddress
            )
        }
        $completed = Wait-Job -Job $jobs -Timeout 120
        Assert-True (@($completed).Count -eq $sessions.Count) "mixed load did not finish within 120 seconds"
        $results = @($jobs | Receive-Job)
        Assert-True ($results.Count -eq $sessions.Count) "mixed load returned an incomplete result set"
        $failures = @($results | Where-Object { -not $_.passed })
        Assert-True ($failures.Count -eq 0) ("mixed client failed: " + (($failures | ForEach-Object { $_.error }) -join "; "))
        $timer.Stop()
        [pscustomobject]@{
            elapsed_ms = [math]::Round($timer.Elapsed.TotalMilliseconds, 2)
            requests = ($results | Measure-Object -Property requests -Sum).Sum
            accepted = ($results | Measure-Object -Property accepted -Sum).Sum
            rejected = ($results | Measure-Object -Property rejected -Sum).Sum
        }
    } finally {
        foreach ($job in $jobs) {
            Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        }
    }
}

function Wait-TravelArrival([string]$token) {
    $headers = @{ Authorization = "Bearer $token" }
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        $region = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/region" -Headers $headers
        if ($region.data.player_location_id -eq "saltmere" -and $region.data.travel.status -eq "arrived") {
            return $region
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Phase 6 load test failed: the first mixed-load journey did not arrive"
}

try {
    Assert-True ($ClientCount -ge 24) "the load target requires at least 24 clients"
    Assert-True ($Rounds -ge 1) "the load test requires at least one round"

    $env:TARROWYN_SERVER_ADDR = $ServerAddress
    $env:TARROWYN_STATE_PATH = $statePath
    $env:TARROWYN_BACKUP_PATH = $backupPath
    $env:TARROWYN_BACKUP_INTERVAL_TICKS = "4"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "250"
    $env:TARROWYN_SESSION_TTL_SECONDS = "120"
    $env:TARROWYN_SUPPORT_OPERATOR_ACCOUNTS = "dev-account-1"

    $server = Start-Phase6Server
    $initialHealth = Wait-Healthy
    $sessions = @()
    for ($index = 0; $index -lt $ClientCount; $index++) {
        $sessions += New-GuestSession "phase6-load-$runId-$index" $true
    }
    $headers = @{ Authorization = "Bearer $($sessions[0].data.account_token)" }
    $seed = Invoke-PostJson "/v1/events/region" `
        @{ request_id = "phase6-load-$runId-event"; action = "seed" } $headers
    Assert-True $seed.data.accepted "the regional event seed was rejected"

    $load = Invoke-ConcurrentMixedLoad $sessions
    $arrival = Wait-TravelArrival $sessions[0].data.account_token
    Assert-True (Test-Path -LiteralPath $backupPath) "the scheduled backup was not written"
    $backup = Get-Content -Raw -LiteralPath $backupPath | ConvertFrom-Json
    Assert-True ($backup.storage_version -ge 20) "the load backup has an old storage version"

    $metrics = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/ops/metrics" -Headers $headers
    Assert-True ($metrics.data.server_tick -gt 0) "the operational tick metric did not advance"
    Assert-True ($metrics.data.completed_commands -gt 0) "completed command metrics did not advance"
    Assert-True ($metrics.data.rejected_commands -gt 0) "rejected command metrics did not advance"
    $alertFlags = @($metrics.data.alert_flags)
    $unexpectedAlertFlags = @($alertFlags | Where-Object { $AllowedAlertFlags -notcontains $_ })
    Assert-True ($unexpectedAlertFlags.Count -eq 0) ("the mixed load raised unexpected alerts: " + ($unexpectedAlertFlags -join ", "))

    Stop-Phase6Server $server
    $server = $null
    $server = Start-Phase6Server
    $restartHealth = Wait-Healthy
    $resumed = New-GuestSession "phase6-load-$runId-0" $false
    $resumedHeaders = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $resumedRegion = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/region" -Headers $resumedHeaders
    Assert-True ($resumedRegion.data.player_location_id -eq "saltmere") "travel location was lost during restart"
    Assert-True ($resumedRegion.data.travel.status -eq "arrived") "travel status was lost during restart"
    Assert-True ($restartHealth.data.ready -and $restartHealth.data.persistence_error -eq $null) "restart readiness reported a persistence failure"

    $alertSummary = if ($alertFlags.Count -eq 0) { "no operational alerts" } else { "allowed alerts: $($alertFlags -join ', ')" }
    Write-Host ("Phase 6 load test passed: {0} clients, {1} rounds, {2} requests, {3} accepted, {4} rejected, {5} ms mixed-load wall time; event, market, travel, tick, backup, metrics, and restart checks passed ({6})." -f `
        $ClientCount, $Rounds, $load.requests, $load.accepted, $load.rejected, $load.elapsed_ms, $alertSummary) -ForegroundColor Green
} finally {
    if ($null -ne $server -and -not $server.HasExited) { Stop-Phase6Server $server }
    foreach ($name in $environmentNames) {
        $value = $previousEnvironment[$name]
        if ($null -eq $value) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            Set-Item "Env:$name" $value
        }
    }
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue
}
