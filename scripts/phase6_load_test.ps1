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
    "DB_DRIVER",
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

function Get-ServerWorkingSetBytes([System.Diagnostics.Process]$process) {
    if ($null -eq $process) { return 0 }
    $processIds = @($process.Id) + @(Get-DescendantProcessIds $process.Id) | Select-Object -Unique
    $workingSet = [int64]0
    foreach ($processId in $processIds) {
        try {
            $workingSet += [int64](Get-Process -Id $processId -ErrorAction Stop).WorkingSet64
        } catch { }
    }
    return $workingSet
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

function Assert-ForbiddenGet([string]$path, [hashtable]$headers, [string]$message) {
    $forbidden = $false
    try {
        Invoke-RestMethod -Method Get -Uri "http://$ServerAddress$path" -Headers $headers
    } catch {
        $forbidden = $_.Exception.Response.StatusCode.value__ -eq 403
    }
    Assert-True $forbidden $message
}

function Assert-CursorErrorGet([string]$path, [hashtable]$headers, [string]$expectedCode, [string]$message) {
    $status = 0
    $body = $null
    try {
        Invoke-RestMethod -Method Get -Uri "http://$ServerAddress$path" -Headers $headers
    } catch {
        $response = $_.Exception.Response
        $body = $_.ErrorDetails.Message
        if ($null -ne $response) {
            $status = [int]$response.StatusCode
            if (-not [string]::IsNullOrWhiteSpace($body)) {
                # PowerShell 7 disposes HttpResponseMessage content before catch.
            } elseif ($response -is [System.Net.Http.HttpResponseMessage]) {
                $body = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
            } else {
                $reader = New-Object System.IO.StreamReader($response.GetResponseStream())
                try { $body = $reader.ReadToEnd() } finally { $reader.Dispose() }
            }
        }
    }
    $error = $null
    if ($null -ne $body) {
        try { $error = $body | ConvertFrom-Json } catch { }
    }
    $cursorError = $status -eq 409 -and $null -ne $error `
        -and $error.error.code -eq $expectedCode
    Assert-True $cursorError $message
}

function Wait-EventCursor([hashtable]$headers, [uint64]$minimum) {
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        $state = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/state" -Headers $headers
        if ([uint64]$state.data.world.cursor -gt $minimum) { return $state }
        Start-Sleep -Milliseconds 100
    }
    throw "Phase 6 load test failed: the event cursor did not cross the retained-history boundary"
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
    $env:DB_DRIVER = "json"
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
    $support = Invoke-RestMethod -Method Get `
        -Uri "http://$ServerAddress/v1/support/account?account_id=$($sessions[0].data.account_id)" `
        -Headers $headers
    Assert-True ($support.data.account.account_id -eq $sessions[0].data.account_id) "the allowlisted support view returned the wrong account"
    Assert-True ($support.data.account.character_id -eq $sessions[0].data.character_id) "the support view returned the wrong character boundary"
    Assert-True ($support.data.event_cursor -ge 0) "the support view omitted its event cursor"
    foreach ($secretField in @("account_token", "refresh_token", "provider_subject")) {
        Assert-True ($support.data.PSObject.Properties.Name -notcontains $secretField) "the support view exposed $secretField"
        Assert-True ($support.data.account.PSObject.Properties.Name -notcontains $secretField) "the support account exposed $secretField"
    }
    $ordinaryHeaders = @{ Authorization = "Bearer $($sessions[1].data.account_token)" }
    Assert-ForbiddenGet "/v1/support/account?account_id=$($sessions[0].data.account_id)" `
        $ordinaryHeaders "an ordinary player could read the support account view"
    Assert-CursorErrorGet "/v1/events?since=999999999" $headers "cursor_ahead" `
        "the shared event endpoint did not expose its cursor_ahead boundary"
    Assert-CursorErrorGet "/v1/events/region?since=999999999" $headers "cursor_ahead" `
        "the regional event endpoint did not expose its cursor_ahead boundary"
    $seed = Invoke-PostJson "/v1/events/region" `
        @{ request_id = "phase6-load-$runId-event"; action = "seed" } $headers
    Assert-True $seed.data.accepted "the regional event seed was rejected"

    $load = Invoke-ConcurrentMixedLoad $sessions
    $arrival = Wait-TravelArrival $sessions[0].data.account_token
    Assert-True (Test-Path -LiteralPath $backupPath) "the scheduled backup was not written"
    $backup = Get-Content -Raw -LiteralPath $backupPath | ConvertFrom-Json
    Assert-True ($backup.storage_version -ge 20) "the load backup has an old storage version"
    $serverWorkingSetBytes = Get-ServerWorkingSetBytes $server
    Assert-True ($serverWorkingSetBytes -gt 0) "the load server working set could not be measured"

    $metrics = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/ops/metrics" -Headers $headers
    Assert-True ($metrics.data.server_tick -gt 0) "the operational tick metric did not advance"
    Assert-True ($metrics.data.completed_commands -gt 0) "completed command metrics did not advance"
    Assert-True ($metrics.data.rejected_commands -gt 0) "rejected command metrics did not advance"
    foreach ($metricField in @(
        "average_price_index_percent", "scarce_goods_count", "npc_fallback_households",
        "open_market_fallback_orders", "abandoned_claims", "declining_settlements",
        "newcomer_access", "alert_flags"
    )) {
        Assert-True ($null -ne $metrics.data.$metricField) "the operational metrics omitted $metricField"
    }
    Assert-ForbiddenGet "/v1/ops/metrics" $ordinaryHeaders "an ordinary player could read operational metrics"
    $alertFlags = @($metrics.data.alert_flags)
    $unexpectedAlertFlags = @($alertFlags | Where-Object { $AllowedAlertFlags -notcontains $_ })
    Assert-True ($unexpectedAlertFlags.Count -eq 0) ("the mixed load raised unexpected alerts: " + ($unexpectedAlertFlags -join ", "))

    $normalTickMs = $env:TARROWYN_TICK_MS
    Stop-Phase6Server $server
    $server = $null
    $env:TARROWYN_TICK_MS = "1"
    $server = Start-Phase6Server
    $null = Wait-Healthy
    $staleSession = New-GuestSession "phase6-load-$runId-stale" $true
    $staleHeaders = @{ Authorization = "Bearer $($staleSession.data.account_token)" }
    $null = Wait-EventCursor $staleHeaders 2048
    Assert-CursorErrorGet "/v1/events?since=0" $staleHeaders "cursor_stale" `
        "the shared event endpoint did not expose its retained-history boundary"
    Assert-CursorErrorGet "/v1/events/region?since=0" $staleHeaders "cursor_stale" `
        "the regional event endpoint did not expose its retained-history boundary"
    Stop-Phase6Server $server
    $server = $null
    $env:TARROWYN_TICK_MS = $normalTickMs
    $server = Start-Phase6Server
    $null = Wait-Healthy

    $restartTimer = [System.Diagnostics.Stopwatch]::StartNew()
    Stop-Phase6Server $server
    $server = $null
    $server = Start-Phase6Server
    $restartHealth = Wait-Healthy
    $restartTimer.Stop()
    $restartRecoveryMs = [math]::Round($restartTimer.Elapsed.TotalMilliseconds, 2)
    $resumed = New-GuestSession "phase6-load-$runId-0" $false
    $resumedHeaders = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $resumedRegion = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/region" -Headers $resumedHeaders
    Assert-True ($resumedRegion.data.player_location_id -eq "saltmere") "travel location was lost during restart"
    Assert-True ($resumedRegion.data.travel.status -eq "arrived") "travel status was lost during restart"
    Assert-True ($restartHealth.data.ready -and $restartHealth.data.persistence_error -eq $null) "restart readiness reported a persistence failure"

    $alertSummary = if ($alertFlags.Count -eq 0) { "no operational alerts" } else { "allowed alerts: $($alertFlags -join ', ')" }
    $workingSetMb = [math]::Round($serverWorkingSetBytes / 1MB, 2)
    Write-Host ("Phase 6 load test passed: {0} clients, {1} rounds, {2} requests, {3} accepted, {4} rejected, {5} ms mixed-load wall time, {6} MB server working set, {7} ms restart recovery; event, market, travel, tick, backup, metrics, support-view, and restart checks passed ({8})." -f `
        $ClientCount, $Rounds, $load.requests, $load.accepted, $load.rejected, $load.elapsed_ms, $workingSetMb, $restartRecoveryMs, $alertSummary) -ForegroundColor Green
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
