<#
.SYNOPSIS
    Writes immutable identity and checksum records for the release archives.

.DESCRIPTION
    Validates the publisher's Windows and WebGL ZIPs plus the host-targeted
    server ZIP, records every archive entry, and writes an external manifest
    plus SHA-256 sidecars. The manifest is release evidence, not a public-
    launch approval.
#>
param(
    [string]$WindowsArchivePath = "dist\years_of_tarrowyn_windows.zip",
    [string]$WebGLArchivePath = "dist\tarrowyn_webgl.zip",
    [string]$ServerArchivePath = "dist\tarrowyn_server.zip",
    [string]$OutputPath = "dist\tarrowyn_release_manifest.json",
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

function Assert-ChildPath {
    param([string]$Parent, [string]$Child)

    $parentFull = [IO.Path]::GetFullPath($Parent).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    $childFull = [IO.Path]::GetFullPath($Child)
    if (-not $childFull.StartsWith($parentFull, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes the expected directory: $childFull"
    }
}

function Resolve-DistPath {
    param([string]$ProjectDir, [string]$DistDir, [string]$Path)

    $resolved = if ([IO.Path]::IsPathRooted($Path)) {
        [IO.Path]::GetFullPath($Path)
    } else {
        [IO.Path]::GetFullPath((Join-Path $ProjectDir $Path))
    }
    Assert-ChildPath $DistDir $resolved
    return $resolved
}

function Get-StreamSha256 {
    param([IO.Stream]$Stream)

    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha256.ComputeHash($Stream))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha256.Dispose()
    }
}

function Read-ArchiveRecord {
    param(
        [string]$Path,
        [string]$Target,
        [string[]]$RequiredFiles
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing $Target release archive: $Path. Run .\publish.ps1 first."
    }
    if ([IO.Path]::GetExtension($Path) -ine '.zip') {
        throw "Release archive must be a ZIP file: $Path"
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $files = @()
        $seen = @{}
        foreach ($entry in $archive.Entries) {
            $normalized = $entry.FullName.Replace('\', '/')
            if ($normalized.EndsWith('/')) { continue }
            if ([string]::IsNullOrWhiteSpace($normalized) -or
                $normalized.StartsWith('/') -or
                $normalized -match '(^|/)\.\.(/|$)') {
                throw "Unsafe path in $Target archive: $normalized"
            }
            if ($seen.ContainsKey($normalized)) {
                throw "Duplicate path in $Target archive: $normalized"
            }
            $seen[$normalized] = $true

            $stream = $entry.Open()
            try {
                $files += [ordered]@{
                    path = $normalized
                    bytes = [long]$entry.Length
                    sha256 = Get-StreamSha256 $stream
                }
            } finally {
                $stream.Dispose()
            }
        }
    } finally {
        $archive.Dispose()
    }

    foreach ($required in $RequiredFiles) {
        if (-not $seen.ContainsKey($required)) {
            throw "$Target archive is missing required file: $required"
        }
    }

    if ($Target -in @('windows', 'server')) {
        $executables = @($files | Where-Object {
            [IO.Path]::GetExtension([string]$_.path) -ieq '.exe' -and
            -not ([string]$_.path).Contains('/')
        })
        if ($executables.Count -ne 1) {
            throw "Windows release must contain exactly one root executable; found $($executables.Count)."
        }
    } elseif ($Target -eq 'webgl') {
        $wasmFiles = @($files | Where-Object { [IO.Path]::GetExtension([string]$_.path) -ieq '.wasm' })
        if ($wasmFiles.Count -ne 1) {
            throw "WebGL release must contain exactly one WASM file; found $($wasmFiles.Count)."
        }
    }

    $archiveInfo = Get-Item -LiteralPath $Path
    return [ordered]@{
        target = $Target
        filename = $archiveInfo.Name
        bytes = [long]$archiveInfo.Length
        sha256 = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
        files = @($files | Sort-Object path)
    }
}

$projectDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$distDir = [IO.Path]::GetFullPath((Join-Path $projectDir 'dist'))
$windowsArchive = Resolve-DistPath $projectDir $distDir $WindowsArchivePath
$webglArchive = Resolve-DistPath $projectDir $distDir $WebGLArchivePath
$serverArchive = Resolve-DistPath $projectDir $distDir $ServerArchivePath
$outputPath = Resolve-DistPath $projectDir $distDir $OutputPath

$commit = (& git -C $projectDir rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $commit -notmatch '^[0-9a-f]{40}$') {
    throw 'Could not resolve the Tarrowyn Git commit.'
}
$dirtyLines = @(& git -C $projectDir status --porcelain)
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the Tarrowyn working tree.' }
$isDirty = $dirtyLines.Count -gt 0
if ($isDirty -and -not $AllowDirty) {
    throw 'The working tree is dirty. Commit the release candidate or pass -AllowDirty for internal testing.'
}

Push-Location $projectDir
try {
    $metadata = (& cargo metadata --manifest-path Cargo.toml --locked --no-deps --format-version 1) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Could not read locked Cargo metadata.' }
    $packages = @($metadata.packages | Where-Object { $_.name -eq 'years_of_tarrowyn' })
    if ($packages.Count -ne 1) { throw 'Expected exactly one years_of_tarrowyn package in Cargo metadata.' }
    $version = [string]$packages[0].version
} finally {
    Pop-Location
}

$windowsRecord = Read-ArchiveRecord $windowsArchive 'windows' @('years_of_tarrowyn.exe')
$webglRecord = Read-ArchiveRecord $webglArchive 'webgl' @('index.html')
$serverRecord = Read-ArchiveRecord $serverArchive 'server' @(
    'BUILD_INFO.json',
    'server/migrations/0001_initial_world.sql',
    'docs/SERVER_DEPLOYMENT.md'
)
$shortCommit = $commit.Substring(0, 12)
$dirtySuffix = if ($isDirty) { '-dirty' } else { '' }
$manifest = [ordered]@{
    schema_version = 1
    package_status = 'internal_preview_not_publicly_approved'
    game = 'years_of_tarrowyn'
    version = $version
    build_id = "$version+g$shortCommit$dirtySuffix"
    git_commit = $commit
    working_tree_dirty = $isDirty
    built_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    archives = @($windowsRecord, $webglRecord, $serverRecord)
    excluded_runtime_state = @(
        'dist/tarrowyn-server-state.json',
        'dist/tarrowyn-server-state.json.backup'
    )
}

$utf8NoBom = [Text.UTF8Encoding]::new($false)
$outputDirectory = Split-Path -Parent $outputPath
if (-not (Test-Path -LiteralPath $outputDirectory)) {
    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
}
[IO.File]::WriteAllText($outputPath, ($manifest | ConvertTo-Json -Depth 8) + [Environment]::NewLine, $utf8NoBom)

foreach ($record in @($windowsRecord, $webglRecord, $serverRecord)) {
    $archivePath = Join-Path $distDir $record.filename
    $sidecarPath = "$archivePath.sha256"
    $sidecar = "$($record.sha256)  $($record.filename)$([Environment]::NewLine)"
    [IO.File]::WriteAllText($sidecarPath, $sidecar, $utf8NoBom)
}

Write-Host "Release manifest written for $($manifest.build_id):" -ForegroundColor Green
Write-Host "  Manifest: $outputPath"
Write-Host "  Windows:  $($windowsRecord.sha256)"
Write-Host "  WebGL:    $($webglRecord.sha256)"
Write-Host "  Server:   $($serverRecord.sha256)"
Write-Host "  Runtime state excluded: yes"
