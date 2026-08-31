<#
.SYNOPSIS
    Launches the packaged authoritative server and checks local readiness.

.DESCRIPTION
    Expands a server ZIP into an isolated target directory, starts its
    executable with disposable JSON state and backup paths, checks both health
    endpoints, and removes the temporary run directory. It never uses the
    development state path or a configured database.
#>
param(
    [string]$ArchivePath = 'dist\tarrowyn_server.zip',
    [int]$TimeoutSeconds = 20
)

$ErrorActionPreference = 'Stop'

function Assert-ChildPath {
    param([string]$Parent, [string]$Child)

    $parentFull = [IO.Path]::GetFullPath($Parent).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    $childFull = [IO.Path]::GetFullPath($Child)
    if (-not $childFull.StartsWith($parentFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes the expected directory: $childFull"
    }
}

function Get-FreePort {
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    try {
        $listener.Start()
        return ([Net.IPEndPoint]$listener.LocalEndpoint).Port
    } finally {
        $listener.Stop()
    }
}

function Assert-SafeArchive {
    param([string]$Path)

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $seen = @{}
        foreach ($entry in $archive.Entries) {
            $normalized = $entry.FullName.Replace('\', '/')
            if ([string]::IsNullOrWhiteSpace($normalized) -or
                $normalized.StartsWith('/') -or
                $normalized -match '^[A-Za-z]:/' -or
                $normalized -match '(^|/)\.\.(/|$)') {
                throw "Unsafe path in server release archive: $normalized"
            }
            if ($seen.ContainsKey($normalized)) {
                throw "Duplicate path in server release archive: $normalized"
            }
            $seen[$normalized] = $true
        }
    } finally {
        $archive.Dispose()
    }
}

function Get-JsonEndpoint {
    param([string]$Url)

    Invoke-RestMethod -Uri $Url -Method Get -TimeoutSec 2
}

function Read-BuildInfo {
    param([string]$Path)

    try {
        return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
    } catch {
        throw "Server package BUILD_INFO.json is invalid: $Path"
    }
}

if ($TimeoutSeconds -lt 1 -or $TimeoutSeconds -gt 120) {
    throw 'TimeoutSeconds must be between 1 and 120.'
}

$projectDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$targetDir = [IO.Path]::GetFullPath((Join-Path $projectDir 'target'))
$archive = if ([IO.Path]::IsPathRooted($ArchivePath)) {
    [IO.Path]::GetFullPath($ArchivePath)
} else {
    [IO.Path]::GetFullPath((Join-Path $projectDir $ArchivePath))
}
if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
    throw "Server release archive is missing: $archive"
}
if ([IO.Path]::GetExtension($archive) -ine '.zip') {
    throw "Server release archive must be a ZIP file: $archive"
}
Assert-SafeArchive $archive

$runDir = Join-Path $targetDir ('.tarrowyn-server-release-' + [Guid]::NewGuid().ToString('N'))
$stdoutPath = Join-Path $runDir 'server.stdout.log'
$stderrPath = Join-Path $runDir 'server.stderr.log'
$statePath = Join-Path $runDir 'state.json'
$backupPath = Join-Path $runDir 'state.json.backup'
$process = $null
$environment = @{}
$environmentNames = @(
    'TARROWYN_SERVER_ADDR',
    'TARROWYN_STATE_PATH',
    'TARROWYN_BACKUP_PATH',
    'DB_DRIVER',
    'DB_HOST',
    'DB_PORT',
    'DB_DATABASE',
    'DB_USERNAME',
    'DB_PASSWORD'
)

try {
    Assert-ChildPath $targetDir $runDir
    New-Item -ItemType Directory -Path $runDir -Force | Out-Null
    Expand-Archive -LiteralPath $archive -DestinationPath $runDir -Force

    $buildInfoPath = Join-Path $runDir 'BUILD_INFO.json'
    if (-not (Test-Path -LiteralPath $buildInfoPath -PathType Leaf)) {
        throw 'Server release archive is missing BUILD_INFO.json.'
    }
    $buildInfo = Read-BuildInfo $buildInfoPath
    if ($buildInfo.schema_version -ne 1 -or
        [string]$buildInfo.game -ne 'years_of_tarrowyn' -or
        [string]$buildInfo.package -ne 'tarrowyn-server') {
        throw 'Packaged server identity is not a Tarrowyn server package.'
    }
    $target = [string]$buildInfo.target
    if ($target -notmatch '^[A-Za-z0-9][A-Za-z0-9_.-]*$') {
        throw "Packaged server target is invalid: $target"
    }
    $executable = [string]$buildInfo.executable
    if ($executable -notmatch '^tarrowyn-server(?:\.exe)?$') {
        throw "Packaged server executable is invalid: $executable"
    }
    if ($target -match 'windows' -and $executable -ne 'tarrowyn-server.exe') {
        throw 'Windows server targets must package tarrowyn-server.exe.'
    }
    if ($target -notmatch 'windows' -and $executable -eq 'tarrowyn-server.exe') {
        throw 'Non-Windows server targets must package tarrowyn-server without the Windows extension.'
    }
    $binary = Join-Path $runDir $executable
    Assert-ChildPath $runDir $binary
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Packaged server executable is missing: $executable"
    }

    $forbidden = @('tarrowyn-server-state.json', 'tarrowyn-server-state.json.backup', '.env')
    $forbiddenFiles = @(Get-ChildItem -LiteralPath $runDir -Recurse -File | Where-Object { $forbidden -contains $_.Name })
    if ($forbiddenFiles.Count -gt 0) {
        throw "Server package contains runtime state or environment files: $($forbiddenFiles[0].FullName)"
    }

    foreach ($name in $environmentNames) {
        $environment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    }
    $port = Get-FreePort
    $env:TARROWYN_SERVER_ADDR = "127.0.0.1:$port"
    $env:TARROWYN_STATE_PATH = $statePath
    $env:TARROWYN_BACKUP_PATH = $backupPath
    $env:DB_DRIVER = 'json'
    Remove-Item Env:DB_HOST, Env:DB_PORT, Env:DB_DATABASE, Env:DB_USERNAME, Env:DB_PASSWORD -ErrorAction SilentlyContinue

    $startArguments = @{
        FilePath = $binary
        WorkingDirectory = $runDir
        PassThru = $true
        RedirectStandardOutput = $stdoutPath
        RedirectStandardError = $stderrPath
    }
    if ($env:OS -eq 'Windows_NT' -or $IsWindows) {
        $startArguments.WindowStyle = 'Hidden'
    }
    $process = Start-Process @startArguments
    $baseUrl = "http://127.0.0.1:$port"
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $ready = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($process.HasExited) {
            $stderr = if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw } else { '' }
            throw "Packaged server exited before readiness: $stderr"
        }
        try {
            $health = Get-JsonEndpoint "$baseUrl/health"
            $ready = Get-JsonEndpoint "$baseUrl/v1/ops/health"
            if ($health.data.status -eq 'ok' -and
                $ready.data.ready -eq $true -and
                $ready.data.integrity_ok -eq $true) {
                break
            }
        } catch {
            # The process may still be binding its listener; retry until the deadline.
        }
        Start-Sleep -Milliseconds 250
    }
    if ($null -eq $ready -or $ready.data.ready -ne $true -or $ready.data.integrity_ok -ne $true) {
        $stderr = if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw } else { '' }
        throw "Packaged server did not report ready within $TimeoutSeconds seconds. $stderr"
    }

    Write-Host "Packaged server launch check passed for $($buildInfo.build_id):" -ForegroundColor Green
    Write-Host "  Target:  $($buildInfo.target)"
    Write-Host '  Storage: isolated JSON state and backup'
    Write-Host '  Health:  /health and /v1/ops/health ready'
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        $null = $process.WaitForExit(5000)
    }
    foreach ($name in $environmentNames) {
        $value = $environment[$name]
        if ($null -eq $value) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable($name, $value, 'Process')
        }
    }
    if (Test-Path -LiteralPath $runDir) {
        Assert-ChildPath $targetDir $runDir
        Remove-Item -LiteralPath $runDir -Recurse -Force
    }
}
