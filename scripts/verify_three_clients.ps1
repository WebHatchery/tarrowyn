$ErrorActionPreference = "Stop"

$gameDir = Split-Path $PSScriptRoot -Parent
$server = $null
$oldDbDriver = $env:DB_DRIVER

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) {
        throw "Three-client check failed: $message"
    }
}

try {
    $env:DB_DRIVER = "json"
    $server = Start-Process -FilePath "cargo.exe" `
        -ArgumentList @("run", "-p", "tarrowyn-server", "--quiet") `
        -WorkingDirectory $gameDir `
        -WindowStyle Hidden `
        -PassThru
    $health = $null
    for ($attempt = 0; $attempt -lt 40 -and $null -eq $health; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/health"
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    Assert-True ($null -ne $health -and $health.data.status -eq "ok") "server did not become healthy"

    $sessions = @()
    foreach ($key in @("fixture-one", "fixture-two", "fixture-three")) {
        $sessions += Invoke-RestMethod -Method Post `
            -Uri "http://127.0.0.1:8787/v1/session/guest" `
            -ContentType "application/json" `
            -Body (@{ client_key = $key; reset = $true } | ConvertTo-Json)
    }
    $characters = @($sessions | ForEach-Object { $_.data.character_id })
    Assert-True (@($characters | Select-Object -Unique).Count -eq 3) "guest characters were not distinct"

    $headers = @()
    foreach ($session in $sessions) {
        $headers += @{ Authorization = "Bearer $($session.data.account_token)" }
    }
    $worlds = @()
    foreach ($header in $headers) {
        $worlds += Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/world" -Headers $header
    }
    Assert-True (($worlds | ForEach-Object { $_.data.players.Count } | Measure-Object -Minimum).Minimum -eq 3) "clients did not share one presence projection"

    $moveBody = @{ request_id = "three-client-valid"; dx = 0; dy = 1 } | ConvertTo-Json
    $move = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/movement" -Headers $headers[0] -ContentType "application/json" -Body $moveBody
    Assert-True $move.data.accepted "valid movement was rejected"

    $invalidBody = @{ request_id = "three-client-invalid"; dx = 8; dy = 0 } | ConvertTo-Json
    $invalid = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/movement" -Headers $headers[0] -ContentType "application/json" -Body $invalidBody
    Assert-True (-not $invalid.data.accepted) "invalid movement was accepted"
    Assert-True ($invalid.data.position.x -eq $move.data.position.x -and $invalid.data.position.y -eq $move.data.position.y) "rejected movement did not return the authoritative position"

    $messages = @("one sees the road", "two sees the road", "three sees the road")
    for ($index = 0; $index -lt $messages.Count; $index++) {
        $chatBody = @{ request_id = "three-client-chat-$index"; channel = "settlement"; text = $messages[$index] } | ConvertTo-Json
        $null = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8787/v1/chat" -Headers $headers[$index] -ContentType "application/json" -Body $chatBody
    }
    $events = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/events?since=0" -Headers $headers[1]
    $chatTexts = @($events.data.events | Where-Object { $_.event.kind -eq "Chat" } | ForEach-Object { $_.event.value.text })
    Assert-True ($chatTexts.Count -ge 3) "chat events were not visible to the second client"
    Assert-True (($chatTexts[-3..-1] -join "|") -eq ($messages -join "|")) "chat event order was not preserved"

    $beforeTick = $worlds[0].meta.server_tick
    Start-Sleep -Milliseconds 700
    $afterWorld = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/world" -Headers $headers[2]
    Assert-True ($afterWorld.meta.server_tick -gt $beforeTick) "the server tick did not advance"
    Write-Host "Three-client Phase 1 acceptance passed: identities, shared presence, authoritative movement, ordered chat, and one world clock." -ForegroundColor Green
} finally {
    if ($null -ne $server -and -not $server.HasExited) {
        Stop-Process -Id $server.Id -Force
    }
    if ($null -eq $oldDbDriver) { Remove-Item Env:DB_DRIVER -ErrorAction SilentlyContinue } else { $env:DB_DRIVER = $oldDbDriver }
}
