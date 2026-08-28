param(
    [string]$PreviewPath = ".env.preview",
    [string]$ServerAddress = "127.0.0.1:8799"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$resolvedPreviewPath = if ([System.IO.Path]::IsPathRooted($PreviewPath)) {
    $PreviewPath
} else {
    Join-Path $projectRoot $PreviewPath
}
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-mysql-" + [guid]::NewGuid().ToString("N"))
$server = $null
$environmentNames = @(
    "DB_DRIVER", "DB_HOST", "DB_PORT", "DB_DATABASE", "DB_USERNAME", "DB_PASSWORD",
    "TARROWYN_SERVER_ADDR", "TARROWYN_BACKUP_PATH", "TARROWYN_BACKUP_INTERVAL_TICKS",
    "TARROWYN_TICK_MS", "TARROWYN_SESSION_TTL_SECONDS", "TARROWYN_MOVEMENT_COOLDOWN_TICKS"
)
$oldEnvironment = @{}

function Assert-True([bool]$condition, [string]$message) {
    if (-not $condition) { throw "MySQL acceptance failed: $message" }
}

function Import-PreviewEnvironment {
    if (-not (Test-Path -LiteralPath $resolvedPreviewPath)) {
        throw "Preview configuration not found: $resolvedPreviewPath"
    }
    foreach ($line in Get-Content -LiteralPath $resolvedPreviewPath) {
        if ($line -notmatch '^\s*([A-Z][A-Z0-9_]*)\s*=\s*(.*?)\s*$') { continue }
        $name = $Matches[1]
        $value = $Matches[2]
        if ($value.Length -ge 2 -and (($value.StartsWith('"') -and $value.EndsWith('"')) -or ($value.StartsWith("'") -and $value.EndsWith("'")))) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
    Assert-True ($env:DB_DRIVER -eq "mysql") "preview DB_DRIVER must be mysql"
    Assert-True (-not [string]::IsNullOrWhiteSpace($env:DB_DATABASE)) "preview DB_DATABASE must be non-empty"
}

function Post-Json([string]$path, [hashtable]$body, [hashtable]$headers) {
    Invoke-RestMethod -Method Post -Uri "http://$ServerAddress$path" `
        -Headers $headers -ContentType "application/json" -Body ($body | ConvertTo-Json -Compress)
}

function Get-DescendantProcessIds([int]$parentId) {
    $children = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId")
    foreach ($child in $children) {
        Get-DescendantProcessIds $child.ProcessId
        $child.ProcessId
    }
}

function Stop-PreviewServer([System.Diagnostics.Process]$process) {
    if ($null -eq $process) { return }
    $processIds = @(Get-DescendantProcessIds $process.Id) + $process.Id
    foreach ($processId in $processIds) {
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        try {
            $null = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/health"
            Start-Sleep -Milliseconds 100
        } catch { return }
    }
    throw "MySQL acceptance failed: preview server did not stop"
}

function Start-PreviewServer {
    Start-Process -FilePath "cargo.exe" `
        -ArgumentList @("run", "-p", "tarrowyn-server", "--quiet") `
        -WorkingDirectory $projectRoot -WindowStyle Hidden -PassThru
}

function Wait-Ready {
    for ($attempt = 0; $attempt -lt 80; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/ops/health"
            if ($health.data.ready -and $health.data.integrity_ok) { return $health }
        } catch { Start-Sleep -Milliseconds 250 }
    }
    throw "MySQL acceptance failed: MySQL-backed server did not become ready"
}

try {
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    foreach ($name in $environmentNames) {
        $item = Get-Item "Env:$name" -ErrorAction SilentlyContinue
        $oldEnvironment[$name] = if ($null -eq $item) { $null } else { $item.Value }
    }
    Import-PreviewEnvironment
    $backupPath = Join-Path $temporaryRoot "mysql-backup.json"
    $env:TARROWYN_SERVER_ADDR = $ServerAddress
    $env:TARROWYN_BACKUP_PATH = $backupPath
    $env:TARROWYN_BACKUP_INTERVAL_TICKS = "1"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_SESSION_TTL_SECONDS = "120"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"

    $server = Start-PreviewServer
    $health = Wait-Ready
    Assert-True ($health.data.storage_version -ge 14) "the migrated world is older than storage version 14"

    $nonce = [guid]::NewGuid().ToString("N")
    $clientKey = "mysql-acceptance-$PID-$nonce"
    $session = Invoke-RestMethod -Method Post -Uri "http://$ServerAddress/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = $clientKey; reset = $true } | ConvertTo-Json -Compress)
    $headers = @{ Authorization = "Bearer $($session.data.account_token)" }
    Assert-True (-not [string]::IsNullOrWhiteSpace($session.data.character_id)) "the MySQL-backed session had no character"

    $state = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/state" -Headers $headers
    Assert-True ($state.data.world.animals.Count -ge 1) "the persisted world projection lost its authored animal"
    Assert-True ($state.data.player.animal_condition -ge 0) "the persisted animal condition was invalid"

    $chatText = "MySQL bridge acceptance $nonce"
    $chatBody = @{ request_id = "mysql-chat-$nonce"; channel = "settlement"; text = $chatText }
    $chat = Post-Json "/v1/chat" $chatBody $headers
    $replayedChat = Post-Json "/v1/chat" $chatBody $headers
    Assert-True ($chat.data.accepted -and $replayedChat.data.accepted) "the MySQL-backed chat mutation was rejected"
    Assert-True ($chat.data.message.message_id -eq $replayedChat.data.message.message_id) "duplicate request replay produced a second MySQL-backed chat result"

    for ($attempt = 0; $attempt -lt 80 -and -not (Test-Path -LiteralPath $backupPath); $attempt++) {
        Start-Sleep -Milliseconds 250
    }
    Assert-True (Test-Path -LiteralPath $backupPath) "the MySQL-backed server did not write its configured backup"
    $backup = Get-Content -Raw -LiteralPath $backupPath | ConvertFrom-Json
    Assert-True ($backup.storage_version -ge 14) "the backup storage version was not current"
    Assert-True ($backup.phase4.animals.Count -ge 1) "the backup omitted the authored animal state"
    $characterId = $session.data.character_id

    Stop-PreviewServer $server
    $server = $null
    $server = Start-PreviewServer
    $health = Wait-Ready
    Assert-True ($health.data.storage_version -ge 14) "the restarted MySQL server reported an old storage version"
    $resumed = Invoke-RestMethod -Method Post -Uri "http://$ServerAddress/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = $clientKey; reset = $false } | ConvertTo-Json -Compress)
    Assert-True ($resumed.data.character_id -eq $characterId) "the MySQL-backed identity did not survive restart"
    $resumedHeaders = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $resumedState = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/state" -Headers $resumedHeaders
    Assert-True ($resumedState.data.world.animals.Count -ge 1) "the restarted MySQL world lost its animal projection"

    Write-Host "MySQL acceptance passed: migration/readiness, authoritative state, duplicate-request replay, backup, and restart persistence." -ForegroundColor Green
} finally {
    if ($null -ne $server -and -not $server.HasExited) { Stop-PreviewServer $server }
    foreach ($name in $environmentNames) {
        $value = $oldEnvironment[$name]
        if ($null -eq $value) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable($name, $value, "Process")
        }
    }
    if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
}
