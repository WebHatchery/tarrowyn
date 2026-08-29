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
    "TARROWYN_TICK_MS", "TARROWYN_SESSION_TTL_SECONDS", "TARROWYN_MOVEMENT_COOLDOWN_TICKS",
    "TARROWYN_MODERATION_COOLDOWN_TICKS", "MYSQL_PWD"
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

function Assert-SecondWorkerRejected {
    $secondAddress = "127.0.0.1:8800"
    $previousAddress = $env:TARROWYN_SERVER_ADDR
    $second = $null
    try {
        $env:TARROWYN_SERVER_ADDR = $secondAddress
        $second = Start-PreviewServer
        if ($null -eq $previousAddress) {
            Remove-Item "Env:TARROWYN_SERVER_ADDR" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable("TARROWYN_SERVER_ADDR", $previousAddress, "Process")
        }
        for ($attempt = 0; $attempt -lt 80 -and -not $second.HasExited; $attempt++) {
            Start-Sleep -Milliseconds 250
        }
        Assert-True $second.HasExited "a second MySQL worker was allowed to start against the same world"
    } finally {
        if ($null -ne $second -and -not $second.HasExited) {
            $originalAddress = $ServerAddress
            $ServerAddress = $secondAddress
            Stop-PreviewServer $second
            $ServerAddress = $originalAddress
        }
        if ($null -eq $previousAddress) {
            Remove-Item "Env:TARROWYN_SERVER_ADDR" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable("TARROWYN_SERVER_ADDR", $previousAddress, "Process")
        }
    }
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

function Resolve-MySqlTool([string]$name) {
    $command = Get-Command $name -ErrorAction SilentlyContinue
    if ($null -ne $command) { return $command.Source }
    $candidate = Join-Path $env:ProgramFiles ("MySQL\MySQL Server 8.0\bin\" + $name)
    if (Test-Path -LiteralPath $candidate) { return $candidate }
    throw "MySQL acceptance failed: $name was not found on PATH or in the standard MySQL Server 8.0 folder"
}

function Invoke-MySql([string]$executable, [string[]]$arguments) {
    $output = & $executable @arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        $message = ($output | ForEach-Object { $_.ToString() }) -join " "
        throw "MySQL acceptance failed: native database command failed: $message"
    }
    return @($output)
}

function Assert-MySqlPrerequisites {
    $mysql = Resolve-MySqlTool "mysql.exe"
    $null = Resolve-MySqlTool "mysqldump.exe"
    $env:MYSQL_PWD = $env:DB_PASSWORD
    $connectionArguments = @(
        "--host=$env:DB_HOST", "--port=$env:DB_PORT", "--user=$env:DB_USERNAME",
        "--batch", "--skip-column-names", "--silent", "--execute=SELECT 1"
    )
    $result = Invoke-MySql $mysql $connectionArguments
    Assert-True (($result -join "").Trim() -eq "1") "the configured MySQL connection did not answer SELECT 1"
}

function Invoke-NativeDatabaseRestore([string]$temporaryRoot, [string]$nonce) {
    $mysql = Resolve-MySqlTool "mysql.exe"
    $dump = Resolve-MySqlTool "mysqldump.exe"
    $dumpPath = Join-Path $temporaryRoot "mysql-native-backup.sql"
    $restoreDatabase = "tarrowyn_restore_${PID}_$($nonce.Substring(0, 8))"
    $connectionArguments = @(
        "--host=$env:DB_HOST", "--port=$env:DB_PORT", "--user=$env:DB_USERNAME",
        "--batch", "--skip-column-names", "--silent"
    )
    try {
        Invoke-MySql $mysql ($connectionArguments + "--execute=CREATE DATABASE IF NOT EXISTS $restoreDatabase CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci") | Out-Null
        Invoke-MySql $dump @(
            "--host=$env:DB_HOST", "--port=$env:DB_PORT", "--user=$env:DB_USERNAME",
            "--single-transaction", "--skip-lock-tables", "--no-tablespaces",
            "--result-file=$dumpPath", $env:DB_DATABASE
        ) | Out-Null
        Assert-True (Test-Path -LiteralPath $dumpPath) "mysqldump did not create the native backup file"

        $restoreInput = Get-Content -Raw -LiteralPath $dumpPath
        $restoreOutput = $restoreInput | & $mysql @($connectionArguments + "--database=$restoreDatabase") 2>&1
        if ($LASTEXITCODE -ne 0) {
            $message = ($restoreOutput | ForEach-Object { $_.ToString() }) -join " "
            throw "MySQL acceptance failed: native database restore failed: $message"
        }

        $versionOutput = Invoke-MySql $mysql ($connectionArguments + "--database=$restoreDatabase" + "--execute=SELECT storage_version FROM tarrowyn_world_state WHERE id = 1")
        $identityOutput = Invoke-MySql $mysql ($connectionArguments + "--database=$restoreDatabase" + "--execute=SELECT COUNT(*) FROM tarrowyn_identity_index")
        $version = [int](($versionOutput -join "").Trim())
        $identityCount = [int](($identityOutput -join "").Trim())
        Assert-True ($version -ge 19) "the native restore lost the current world storage version"
        Assert-True ($identityCount -ge 1) "the native restore lost the identity index"
    } finally {
        Invoke-MySql $mysql ($connectionArguments + "--execute=DROP DATABASE IF EXISTS $restoreDatabase") | Out-Null
    }
}

function Invoke-ConcurrentDuplicateChat([string]$address, [hashtable]$headers, [string]$requestId, [string]$text) {
    $jobScript = {
        param([string]$serverAddress, [string]$token, [string]$duplicateRequestId, [string]$chatText)
        try {
            $response = Invoke-RestMethod -Method Post -Uri "http://$serverAddress/v1/chat" `
                -Headers @{ Authorization = "Bearer $token" } -ContentType "application/json" `
                -Body (@{ request_id = $duplicateRequestId; channel = "settlement"; text = $chatText } | ConvertTo-Json -Compress)
            [pscustomobject]@{
                passed = $true
                accepted = [bool]$response.data.accepted
                messageId = $response.data.message.message_id
            }
        } catch {
            [pscustomobject]@{ passed = $false; error = $_.Exception.Message }
        }
    }
    $jobs = @()
    try {
        for ($index = 0; $index -lt 8; $index++) {
            $jobs += Start-Job -ScriptBlock $jobScript -ArgumentList @(
                $address, $headers.Authorization.Substring(7), $requestId, $text
            )
        }
        $completed = Wait-Job -Job $jobs -Timeout 30
        Assert-True (@($completed).Count -eq 8) "concurrent duplicate chat requests did not finish"
        $results = @($jobs | Receive-Job)
        Assert-True ($results.Count -eq 8) "concurrent duplicate chat requests returned an incomplete result set"
        $failures = @($results | Where-Object { -not $_.passed -or -not $_.accepted })
        Assert-True ($failures.Count -eq 0) "a concurrent duplicate chat request was rejected"
        $messageIds = @($results | Select-Object -ExpandProperty messageId -Unique)
        Assert-True ($messageIds.Count -eq 1) "concurrent duplicate chat requests produced multiple message IDs"
    } finally {
        foreach ($job in $jobs) {
            Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        }
    }
}

try {
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    foreach ($name in $environmentNames) {
        $item = Get-Item "Env:$name" -ErrorAction SilentlyContinue
        $oldEnvironment[$name] = if ($null -eq $item) { $null } else { $item.Value }
    }
    Import-PreviewEnvironment
    Assert-MySqlPrerequisites
    $backupPath = Join-Path $temporaryRoot "mysql-backup.json"
    $env:TARROWYN_SERVER_ADDR = $ServerAddress
    $env:TARROWYN_BACKUP_PATH = $backupPath
    $env:TARROWYN_BACKUP_INTERVAL_TICKS = "1"
    $env:TARROWYN_TICK_MS = "50"
    $env:TARROWYN_SESSION_TTL_SECONDS = "120"
    $env:TARROWYN_MOVEMENT_COOLDOWN_TICKS = "0"

    $server = Start-PreviewServer
    $health = Wait-Ready
    Assert-True ($health.data.storage_version -ge 20) "the migrated world is older than storage version 20"
    Assert-SecondWorkerRejected

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
    $movementBody = @{ request_id = "mysql-movement-$nonce"; dx = 0; dy = 1 }
    $movement = Post-Json "/v1/movement" $movementBody $headers
    Assert-True $movement.data.accepted "the MySQL-backed movement mutation was rejected"
    $concurrentChatText = "Concurrent MySQL bridge acceptance $nonce"
    Invoke-ConcurrentDuplicateChat $ServerAddress $headers "mysql-concurrent-chat-$nonce" $concurrentChatText
    $feed = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/tavern/feed" -Headers $headers
    Assert-True (@($feed.data.chat | Where-Object text -eq $concurrentChatText).Count -eq 1) "concurrent duplicate chat requests were not reduced to one visible message"

    $authClientKey = "$clientKey-auth"
    $authGuest = Invoke-RestMethod -Method Post -Uri "http://$ServerAddress/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = $authClientKey; reset = $true } | ConvertTo-Json -Compress)
    $authGuestHeaders = @{ Authorization = "Bearer $($authGuest.data.account_token)" }
    $linkBody = @{
        request_id = "mysql-link-$nonce"
        provider = "webhatchery-identity-oidc"
        subject = "mysql-subject-$nonce"
        display_name = "MySQL linked traveller"
    }
    $linked = Post-Json "/v1/auth/link" $linkBody $authGuestHeaders
    $linkedHeaders = @{ Authorization = "Bearer $($linked.data.session.account_token)" }
    $linkedRetry = Post-Json "/v1/auth/link" $linkBody $linkedHeaders
    Assert-True ($linkedRetry.data.account_id -eq $linked.data.account_id) "MySQL auth-link replay changed the account boundary"
    $account = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/account" -Headers $linkedHeaders
    Assert-True (-not $account.data.guest_fixture) "the MySQL-linked account remained a guest fixture"

    $refreshBody = @{
        request_id = "mysql-refresh-$nonce"
        refresh_token = $linked.data.session.refresh_token
    }
    $refreshed = Post-Json "/v1/auth/refresh" $refreshBody @{}
    $refreshedHeaders = @{ Authorization = "Bearer $($refreshed.data.session.account_token)" }
    $refreshedRetry = Post-Json "/v1/auth/refresh" $refreshBody @{}
    Assert-True ($refreshedRetry.data.session.account_token -eq $refreshed.data.session.account_token) "MySQL refresh replay rotated a second access session"

    $revokeBody = @{ request_id = "mysql-revoke-$nonce"; revoke_all = $true }
    $revoked = Post-Json "/v1/auth/revoke" $revokeBody $refreshedHeaders
    $revokedRetry = Post-Json "/v1/auth/revoke" $revokeBody $refreshedHeaders
    Assert-True ($revoked.data.revoked_sessions -ge 1) "MySQL auth revoke did not revoke the rotated session"
    Assert-True ($revokedRetry.data.revoked_sessions -eq $revoked.data.revoked_sessions) "MySQL auth-revoke replay changed the result"

    $moderationBody = @{
        request_id = "mysql-moderation-$nonce"
        target_account_id = $session.data.account_id
        message_id = $null
        category = "player_report"
        note = "MySQL moderation replay acceptance $nonce"
    }
    $report = Post-Json "/v1/moderation/report" $moderationBody $headers
    $reportRetry = Post-Json "/v1/moderation/report" $moderationBody $headers
    Assert-True ($report.data.report_id -eq $reportRetry.data.report_id) "MySQL moderation replay queued a second report"
    Assert-True ($report.data.status -eq "queued") "MySQL moderation report was not queued"

    for ($attempt = 0; $attempt -lt 80 -and -not (Test-Path -LiteralPath $backupPath); $attempt++) {
        Start-Sleep -Milliseconds 250
    }
    Assert-True (Test-Path -LiteralPath $backupPath) "the MySQL-backed server did not write its configured backup"
    $backup = Get-Content -Raw -LiteralPath $backupPath | ConvertFrom-Json
    Assert-True ($backup.storage_version -ge 20) "the backup storage version was not current"
    Assert-True ($backup.phase4.animals.Count -ge 1) "the backup omitted the authored animal state"
    $characterId = $session.data.character_id

    Stop-PreviewServer $server
    $server = $null
    $server = Start-PreviewServer
    $health = Wait-Ready
    Assert-True ($health.data.storage_version -ge 20) "the restarted MySQL server reported an old storage version"
    $resumed = Invoke-RestMethod -Method Post -Uri "http://$ServerAddress/v1/session/guest" `
        -ContentType "application/json" -Body (@{ client_key = $clientKey; reset = $false } | ConvertTo-Json -Compress)
    Assert-True ($resumed.data.character_id -eq $characterId) "the MySQL-backed identity did not survive restart"
    $resumedHeaders = @{ Authorization = "Bearer $($resumed.data.account_token)" }
    $resumedState = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/state" -Headers $resumedHeaders
    Assert-True ($resumedState.data.world.animals.Count -ge 1) "the restarted MySQL world lost its animal projection"
    $replayedChatAfterRestart = Post-Json "/v1/chat" $chatBody $resumedHeaders
    Assert-True ($replayedChatAfterRestart.data.message.message_id -eq $chat.data.message.message_id) "the MySQL chat replay was lost across restart"
    $replayedMovementAfterRestart = Post-Json "/v1/movement" $movementBody $resumedHeaders
    Assert-True ($replayedMovementAfterRestart.data.position.x -eq $movement.data.position.x -and $replayedMovementAfterRestart.data.position.y -eq $movement.data.position.y) "the MySQL movement replay was lost across restart"
    $revokedAfterRestart = Post-Json "/v1/auth/revoke" $revokeBody $refreshedHeaders
    Assert-True ($revokedAfterRestart.data.revoked_sessions -eq $revoked.data.revoked_sessions) "the MySQL auth-revoke replay was lost across restart"
    $reportAfterRestart = Post-Json "/v1/moderation/report" $moderationBody $resumedHeaders
    Assert-True ($reportAfterRestart.data.report_id -eq $report.data.report_id) "the MySQL moderation replay was lost across restart"

    Stop-PreviewServer $server
    $server = $null
    $env:MYSQL_PWD = $env:DB_PASSWORD
    Invoke-NativeDatabaseRestore $temporaryRoot $nonce

    Write-Host "MySQL acceptance passed: migration/readiness, single-worker authority, authoritative state, chat/movement/auth/moderation replay, backup, restart persistence, and native dump/restore." -ForegroundColor Green
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
