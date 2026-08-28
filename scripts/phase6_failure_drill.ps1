param(
    [string]$StatePath = "dist/tarrowyn-server-state.json",
    [string]$BackupPath = "dist/tarrowyn-server-state.json.backup",
    [string]$ServerAddress = "127.0.0.1:8798"
)
$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
function Resolve-ProjectPath([string]$path) {
    if ([System.IO.Path]::IsPathRooted($path)) {
        return [System.IO.Path]::GetFullPath($path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $projectRoot $path))
}

$stateFullPath = Resolve-ProjectPath $StatePath
$backupFullPath = Resolve-ProjectPath $BackupPath
if (-not (Test-Path -LiteralPath $stateFullPath)) { throw "State not found: $stateFullPath" }
if (-not (Test-Path -LiteralPath $backupFullPath)) { throw "Backup not found: $backupFullPath" }
$activeStateHash = (Get-FileHash -LiteralPath $stateFullPath -Algorithm SHA256).Hash
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("tarrowyn-restore-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
$server = $null
$oldAddress = $env:TARROWYN_SERVER_ADDR
$oldState = $env:TARROWYN_STATE_PATH
$oldBackup = $env:TARROWYN_BACKUP_PATH
$oldInterval = $env:TARROWYN_BACKUP_INTERVAL_TICKS

function Stop-ServerProcessTree([int]$processId) {
    if ($processId -le 0) { return }
    $children = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
        Where-Object { $_.ParentProcessId -eq $processId }
    foreach ($child in $children) {
        Stop-ServerProcessTree ([int]$child.ProcessId)
    }
    Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
}

try {
    $restorePath = Join-Path $temporaryRoot "restored-state.json"
    $restoreBackupPath = Join-Path $temporaryRoot "restored-state.json.backup"
    Copy-Item -LiteralPath $backupFullPath -Destination $restorePath
    $restored = Get-Content -Raw -LiteralPath $restorePath | ConvertFrom-Json
    if ($restored.storage_version -lt 3) { throw "The backup predates the supported migration floor." }
    if (-not $restored.phase5 -or -not $restored.phase6) { throw "The backup has no Phase 5/6 state fields." }

    $env:TARROWYN_SERVER_ADDR = $ServerAddress
    $env:TARROWYN_STATE_PATH = $restorePath
    $env:TARROWYN_BACKUP_PATH = $restoreBackupPath
    $env:TARROWYN_BACKUP_INTERVAL_TICKS = "1"
    $server = Start-Process -FilePath "cargo.exe" `
        -ArgumentList @("run", "-p", "tarrowyn-server", "--quiet") `
        -WorkingDirectory $projectRoot `
        -WindowStyle Hidden `
        -PassThru
    $health = $null
    for ($attempt = 0; $attempt -lt 40 -and $null -eq $health; $attempt++) {
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://$ServerAddress/v1/ops/health"
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if ($null -eq $health -or -not $health.data.ready -or -not $health.data.integrity_ok) {
        throw "Temporary restore server did not become ready."
    }
    $backupWritten = $false
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if (Test-Path -LiteralPath $restoreBackupPath) {
            $backupWritten = $true
            break
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $backupWritten) {
        throw "Temporary restore server did not write a backup."
    }
    Stop-ServerProcessTree $server.Id
    $activeStateHashAfterDrill = (Get-FileHash -LiteralPath $stateFullPath -Algorithm SHA256).Hash
    if ($activeStateHashAfterDrill -ne $activeStateHash) {
        throw "The active state changed during the restore drill."
    }
    Push-Location $projectRoot
    cargo test -p tarrowyn-server phase5 -- --nocapture
    Pop-Location
    Write-Host "Restore drill passed: temporary restore became ready, wrote a backup, and passed regional tests; active state was not overwritten."
} finally {
    if ($null -ne $server) { Stop-ServerProcessTree $server.Id }
    if ($null -eq $oldAddress) { Remove-Item Env:TARROWYN_SERVER_ADDR -ErrorAction SilentlyContinue } else { $env:TARROWYN_SERVER_ADDR = $oldAddress }
    if ($null -eq $oldState) { Remove-Item Env:TARROWYN_STATE_PATH -ErrorAction SilentlyContinue } else { $env:TARROWYN_STATE_PATH = $oldState }
    if ($null -eq $oldBackup) { Remove-Item Env:TARROWYN_BACKUP_PATH -ErrorAction SilentlyContinue } else { $env:TARROWYN_BACKUP_PATH = $oldBackup }
    if ($null -eq $oldInterval) { Remove-Item Env:TARROWYN_BACKUP_INTERVAL_TICKS -ErrorAction SilentlyContinue } else { $env:TARROWYN_BACKUP_INTERVAL_TICKS = $oldInterval }
    if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
}
