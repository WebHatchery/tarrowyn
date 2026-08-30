$ErrorActionPreference = "Stop"

$gameDir = Split-Path $PSScriptRoot -Parent
$statePath = Join-Path ([System.IO.Path]::GetTempPath()) "tarrowyn-phase3-$PID.json"
$server = $null
$environmentNames = @(
    "DB_DRIVER",
    "TARROWYN_STATE_PATH",
    "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
    "TARROWYN_TICK_MS",
    "TARROWYN_CLAIM_RECLAIM_TICKS",
    "TARROWYN_SESSION_TTL_SECONDS"
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) { throw "Phase 3 acceptance failed: $message" }
}

function Start-Phase3Server {
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

function Stop-Phase3Server([System.Diagnostics.Process]$process) {
    if ($null -eq $process) { return }
    $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
            Start-Sleep -Milliseconds 100
        } catch { return }
    }
    throw "Phase 3 acceptance failed: previous server did not stop"
}

function Wait-Healthy {
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
            if ($health.data.status -eq "ok") { return }
        } catch { Start-Sleep -Milliseconds 100 }
    }
    throw "Phase 3 acceptance failed: server did not become healthy"
}

function Post-Json([string]$path, [hashtable]$body, [hashtable]$headers) {
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787$path" `
        -Headers $headers -ContentType "application/json" -Body ($body | ConvertTo-Json -Compress)
}

function Invoke-ConcurrentSoak([int]$clientCount, [int]$rounds) {
    $soakSessions = @()
    for ($index = 0; $index -lt $clientCount; $index++) {
        $soakSessions += Invoke-RestMethod -Method Post `
            -Uri "http://127.0.0.1:8787/v1/session/guest" `
            -ContentType "application/json" `
            -Body (@{ client_key = "phase3-soak-$index"; reset = $true } | ConvertTo-Json -Compress)
    }

    $jobScript = {
        param([string]$token, [int]$clientIndex, [int]$expectedClients, [int]$roundCount)
        $headers = @{ Authorization = "Bearer $token" }
        try {
            for ($round = 0; $round -lt $roundCount; $round++) {
                $state = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $headers
                if ($state.data.world.players.Count -lt $expectedClients) {
                    throw "only $($state.data.world.players.Count) of $expectedClients players were visible"
                }

                $events = Invoke-RestMethod -Method Get `
                    -Uri "http://127.0.0.1:8787/v1/events?since=0" -Headers $headers
                $move = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/movement" `
                    -Headers $headers -ContentType "application/json" `
                    -Body (@{ request_id = "phase3-soak-$clientIndex-move-$round"; dx = 1; dy = 0 } | ConvertTo-Json -Compress)
                $chat = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/chat" `
                    -Headers $headers -ContentType "application/json" `
                    -Body (@{ request_id = "phase3-soak-$clientIndex-chat-$round"; channel = "settlement"; text = "soak $clientIndex/$round" } | ConvertTo-Json -Compress)

                if ($null -eq $events.data.cursor -or $null -eq $move.data.accepted -or $null -eq $chat.data.accepted) {
                    throw "one or more shared-road responses were incomplete"
                }
            }
            [pscustomobject]@{ passed = $true; client = $clientIndex }
        } catch {
            [pscustomobject]@{ passed = $false; client = $clientIndex; error = $_.Exception.Message }
        }
    }

    $jobs = @()
    try {
        for ($index = 0; $index -lt $clientCount; $index++) {
            $jobs += Start-Job -ScriptBlock $jobScript -ArgumentList @(
                $soakSessions[$index].data.account_token,
                $index,
                $clientCount,
                $rounds
            )
        }
        $completed = Wait-Job -Job $jobs -Timeout 60
        Assert-True (@($completed).Count -eq $clientCount) "concurrent soak did not finish within 60 seconds"
        $results = @($jobs | Receive-Job)
        Assert-True ($results.Count -eq $clientCount) "concurrent soak returned an incomplete result set"
        $failures = @($results | Where-Object { -not $_.passed })
        Assert-True ($failures.Count -eq 0) ("concurrent soak client failed: " + (($failures | ForEach-Object { $_.error }) -join "; "))

        Start-Sleep -Milliseconds 150
        $health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
        Assert-True ($health.meta.server_tick -gt 0) "server clock stopped during concurrent soak"
    } finally {
        foreach ($job in $jobs) {
            Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        }
    }
}

try {
    Remove-Item -LiteralPath $statePath -Force -ErrorAction SilentlyContinue
    $env:DB_DRIVER = "json"
    $env:TARROWYN_STATE_PATH = $statePath
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_CLAIM_RECLAIM_TICKS = "20"
    $env:TARROWYN_SESSION_TTL_SECONDS = "120"

    $server = Start-Phase3Server
    Wait-Healthy
    $sessions = @()
    foreach ($key in @("phase3-scout", "phase3-farmer", "phase3-builder")) {
        $sessions += Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/session/guest" `
            -ContentType "application/json" -Body (@{ client_key = $key; reset = $true } | ConvertTo-Json -Compress)
    }
    $headers = @($sessions | ForEach-Object { @{ Authorization = "Bearer $($_.data.account_token)" } })

    $state = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $headers[0]
    Assert-True ($state.data.world.wilderness.threat_active) "the Brambleback threat was not in the world projection"
    $contracts = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/contracts" -Headers $headers[0]
    Assert-True ($contracts.data.contracts.Count -eq 1) "the repeatable tavern contract was not posted"

    $accept = Post-Json "/v1/contracts/brambleback-watch" @{ request_id = "phase3-contract-accept"; action = "accept"; contract_id = "brambleback-watch" } $headers[0]
    Assert-True $accept.data.accepted "the Brambleback contract was rejected"
    $steps = @(@{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 1; dy = 0 }, @{ dx = 0; dy = -1 }, @{ dx = 0; dy = -1 })
    for ($index = 0; $index -lt $steps.Count; $index++) {
        $move = Post-Json "/v1/movement" @{ request_id = "phase3-step-$index"; dx = $steps[$index].dx; dy = $steps[$index].dy } $headers[0]
        Assert-True $move.data.accepted "the scout could not reach Whisperwood Edge"
    }
    for ($index = 0; $index -lt 3; $index++) {
        $progress = Post-Json "/v1/contracts/brambleback-watch" @{ request_id = "phase3-contract-progress-$index"; action = "progress"; contract_id = "brambleback-watch" } $headers[0]
        Assert-True $progress.data.accepted "the contract did not progress"
    }
    $report = Post-Json "/v1/contracts/brambleback-watch" @{ request_id = "phase3-contract-report"; action = "report"; contract_id = "brambleback-watch" } $headers[0]
    Assert-True $report.data.accepted "the tavern did not report the completed contract"

    $knockout = Post-Json "/v1/combat/actions" @{ request_id = "phase3-improvised-strike"; action = "strike"; weapon = "improvised_club" } $headers[0]
    Assert-True ($knockout.data.outcome -eq "knocked_out" -and $knockout.data.player.knocked_out) "the inferior weapon did not produce an authoritative knockout"
    $recovery = Post-Json "/v1/recovery" @{ request_id = "phase3-rescue"; choice = "ask_rescuer" } $headers[0]
    Assert-True ($recovery.data.accepted -and -not $recovery.data.player.knocked_out) "the recovery prompt did not restore control"

    $chronicle = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/settlement/chronicle?since=0" -Headers $headers[1]
    Assert-True (@($chronicle.data.entries | Where-Object { $_.kind -eq "knockout" }).Count -eq 1) "another player could not read the knockout chronicle"
    $opportunities = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/settlement/opportunities" -Headers $headers[2]
    Assert-True ($opportunities.data.opportunities.Count -ge 1) "the household opportunity signal was missing"

    $claim = Post-Json "/v1/claims" @{ request_id = "phase3-claim"; action = "request" } $headers[0]
    Assert-True $claim.data.accepted "the homestead lease was not recognised"
    $announce = Post-Json "/v1/expeditions" @{ request_id = "phase3-announce"; action = "announce"; role = "scout"; outpost_name = "Lantern Rest"; food = 0; tools = 0; materials = 0; safety = 0 } $headers[0]
    Assert-True $announce.data.accepted "the pioneer expedition was not announced"
    $joinFarmer = Post-Json "/v1/expeditions" @{ request_id = "phase3-join-farmer"; action = "join"; expedition_id = "pioneer-1"; role = "farmer"; food = 0; tools = 0; materials = 0; safety = 0 } $headers[1]
    $joinBuilder = Post-Json "/v1/expeditions" @{ request_id = "phase3-join-builder"; action = "join"; expedition_id = "pioneer-1"; role = "builder"; food = 0; tools = 0; materials = 0; safety = 0 } $headers[2]
    Assert-True ($joinFarmer.data.accepted -and $joinBuilder.data.accepted) "complementary expedition roles could not join"
    $supply = Post-Json "/v1/expeditions" @{ request_id = "phase3-supply"; action = "supply"; expedition_id = "pioneer-1"; food = 6; tools = 3; materials = 8; safety = 3 } $headers[0]
    Assert-True $supply.data.accepted "expedition supplies were rejected"
    $launch = Post-Json "/v1/expeditions" @{ request_id = "phase3-launch"; action = "launch"; expedition_id = "pioneer-1"; food = 0; tools = 0; materials = 0; safety = 0 } $headers[0]
    Assert-True $launch.data.accepted "the prepared expedition could not launch"
    $resolved = Post-Json "/v1/expeditions" @{ request_id = "phase3-resolve"; action = "resolve"; expedition_id = "pioneer-1"; food = 0; tools = 0; materials = 0; safety = 0 } $headers[0]
    Assert-True ($resolved.data.accepted -and $resolved.data.expedition.status -eq "succeeded") "the pioneer expedition did not resolve into an outpost"

    $eventCursor = (Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/events?since=0" -Headers $headers[1]).data.cursor
    $gap = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/events?since=$eventCursor" -Headers $headers[1]
    $replayedEvents = @($gap.data.events)
    $duplicateEvents = @($replayedEvents | Where-Object { [uint64]$_.cursor -le [uint64]$eventCursor })
    Assert-True ($duplicateEvents.Count -eq 0) "event cursor replay returned an event at or before the accepted cursor"
    Assert-True ([uint64]$gap.data.cursor -ge [uint64]$eventCursor) "event cursor replay moved the accepted cursor backwards"

    $characterBeforeRestart = $sessions[0].data.character_id
    Stop-Phase3Server $server
    $server = Start-Phase3Server
    Wait-Healthy
    $resumed = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = "phase3-scout"; reset = $false } | ConvertTo-Json -Compress)
    Assert-True ($resumed.data.character_id -eq $characterBeforeRestart) "the scout identity did not survive restart"
    $resumedHeader = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $resumedState = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/state" -Headers $resumedHeader
    Assert-True ($null -ne $resumedState.data.world.outpost) "the outpost was lost during restart"
    $resumedChronicle = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/settlement/chronicle?since=0" -Headers $resumedHeader
    Assert-True (@($resumedChronicle.data.entries | Where-Object { $_.kind -eq "outpost founded" }).Count -eq 1) "the outpost chronicle was lost during restart"

    Invoke-ConcurrentSoak 20 3
    Write-Host "Phase 3 acceptance passed: threat ripple, contract, knockout recovery, household signal, chronicle, renewable claim, expedition outpost, cursor catch-up, restartable state, and concurrent 20-client polling." -ForegroundColor Green
} finally {
    try {
        if ($null -ne $server -and -not $server.HasExited) { Stop-Phase3Server $server }
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
