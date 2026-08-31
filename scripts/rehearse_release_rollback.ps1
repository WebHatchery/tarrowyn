<#
.SYNOPSIS
    Rehearses patch deployment, rollback, and patch restoration locally.

.DESCRIPTION
    Uses a commit-addressed preserved candidate as the previous release and
    the current manifest as the patch. The exact Windows, WebGL, and
    authoritative server archives are copied through an isolated target
    directory and their manifest/checksum identity is verified at every
    switch. The server package's embedded build identity is also checked. No
    live deployment or world state is changed.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$PreservedDir,
    [string]$CurrentManifestPath = 'dist\tarrowyn_release_manifest.json'
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

function Resolve-ProjectPath {
    param([string]$ProjectDir, [string]$Path)

    if ([IO.Path]::IsPathRooted($Path)) { return [IO.Path]::GetFullPath($Path) }
    return [IO.Path]::GetFullPath((Join-Path $ProjectDir $Path))
}

function Get-ArchiveRecords($Manifest) {
    $records = @($Manifest.archives)
    if ($records.Count -ne 3) { throw 'Release manifest must identify exactly three archives.' }
    $targets = @($records | ForEach-Object { [string]$_.target } | Sort-Object)
    if (($targets -join ',') -ne 'server,webgl,windows') {
        throw 'Release manifest must identify one Windows, WebGL, and server archive.'
    }
    return $records
}

function Read-Candidate {
    param([string]$ManifestPath, [string]$BaseDir)

    if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
        throw "Rollback manifest is missing: $ManifestPath"
    }
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    if ($manifest.schema_version -ne 1 -or $manifest.working_tree_dirty -ne $false) {
        throw "Rollback candidates require schema 1 and a clean build: $ManifestPath"
    }
    $commit = [string]$manifest.git_commit
    if ($commit -notmatch '^[0-9a-f]{40}$') {
        throw "Rollback manifest has an invalid Git commit: $ManifestPath"
    }

    $artifacts = @()
    $seenNames = @{}
    foreach ($record in (Get-ArchiveRecords $manifest)) {
        $filename = [string]$record.filename
        if ([string]::IsNullOrWhiteSpace($filename) -or [IO.Path]::GetFileName($filename) -ne $filename) {
            throw "Rollback manifest contains an unsafe archive filename: $filename"
        }
        if ($seenNames.ContainsKey($filename)) {
            throw "Rollback manifest repeats an archive filename: $filename"
        }
        $seenNames[$filename] = $true

        $archive = Join-Path $BaseDir $filename
        $sidecar = "$archive.sha256"
        foreach ($required in @($archive, $sidecar)) {
            if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
                throw "Rollback candidate file is missing: $required"
            }
        }
        $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($hash -ne ([string]$record.sha256).ToLowerInvariant()) {
            throw "Rollback archive does not match its manifest: $archive"
        }
        if ((Get-Item -LiteralPath $archive).Length -ne [long]$record.bytes) {
            throw "Rollback archive size does not match its manifest: $archive"
        }
        $sidecarParts = (Get-Content -LiteralPath $sidecar -Raw).Trim() -split '\s+'
        if ($sidecarParts.Count -lt 2 -or
            $sidecarParts[0].ToLowerInvariant() -ne $hash -or
            $sidecarParts[1] -ne $filename) {
            throw "Rollback checksum sidecar does not match its archive: $sidecar"
        }
        $artifacts += [pscustomobject]@{
            Target = [string]$record.target
            FileName = $filename
            ArchivePath = $archive
            Hash = $hash
            Bytes = [long]$record.bytes
        }
    }

    return [pscustomobject]@{
        ManifestPath = $ManifestPath
        Commit = $commit
        BuildId = [string]$manifest.build_id
        Artifacts = @($artifacts)
    }
}

function Assert-DeployedCandidate {
    param([string]$Directory, $Candidate)

    foreach ($artifact in $Candidate.Artifacts) {
        $archivePath = Join-Path $Directory $artifact.FileName
        $hash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($hash -ne $artifact.Hash) {
            throw "Rehearsal archive hash mismatch: $archivePath"
        }
        if ((Get-Item -LiteralPath $archivePath).Length -ne $artifact.Bytes) {
            throw "Rehearsal archive size mismatch: $archivePath"
        }
        if ($artifact.Target -ne 'server') { continue }

        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($archivePath)
        try {
            $entry = $zip.GetEntry('BUILD_INFO.json')
            if ($null -eq $entry) {
                throw "Server rehearsal archive lacks BUILD_INFO.json: $archivePath"
            }
            $reader = [IO.StreamReader]::new($entry.Open())
            try { $buildInfo = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
        } finally {
            $zip.Dispose()
        }
        if ([string]$buildInfo.git_commit -ne $Candidate.Commit -or
            [string]$buildInfo.build_id -ne $Candidate.BuildId) {
            throw "Embedded server build identity does not match its manifest: $archivePath"
        }
    }
}

$projectDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$distDir = [IO.Path]::GetFullPath((Join-Path $projectDir 'dist'))
$historyRoot = Join-Path $distDir 'history'
$preserved = Resolve-ProjectPath $projectDir $PreservedDir
Assert-ChildPath $historyRoot $preserved
$previousManifest = Join-Path $preserved 'tarrowyn_release_manifest.json'
$currentManifest = Resolve-ProjectPath $projectDir $CurrentManifestPath
Assert-ChildPath $distDir $currentManifest
$previous = Read-Candidate $previousManifest $preserved
$current = Read-Candidate $currentManifest $distDir
if ($previous.Commit -eq $current.Commit) {
    throw 'Previous and patch candidates must identify different commits.'
}

$targetRoot = [IO.Path]::GetFullPath((Join-Path $projectDir 'target'))
$rehearsalDir = Join-Path $targetRoot ('tarrowyn-rollback-' + [Guid]::NewGuid().ToString('N'))
$evidencePath = Join-Path $targetRoot 'tarrowyn-release-rollback-rehearsal.json'
Assert-ChildPath $targetRoot $rehearsalDir

try {
    New-Item -ItemType Directory -Path $rehearsalDir -Force | Out-Null
    foreach ($artifact in $current.Artifacts) {
        Copy-Item -LiteralPath $artifact.ArchivePath -Destination (Join-Path $rehearsalDir $artifact.FileName)
    }
    Assert-DeployedCandidate $rehearsalDir $current
    foreach ($artifact in $previous.Artifacts) {
        Copy-Item -LiteralPath $artifact.ArchivePath -Destination (Join-Path $rehearsalDir $artifact.FileName) -Force
    }
    Assert-DeployedCandidate $rehearsalDir $previous
    foreach ($artifact in $current.Artifacts) {
        Copy-Item -LiteralPath $artifact.ArchivePath -Destination (Join-Path $rehearsalDir $artifact.FileName) -Force
    }
    Assert-DeployedCandidate $rehearsalDir $current

    $previousByTarget = @{}
    foreach ($artifact in $previous.Artifacts) { $previousByTarget[$artifact.Target] = $artifact.Hash }
    $currentByTarget = @{}
    foreach ($artifact in $current.Artifacts) { $currentByTarget[$artifact.Target] = $artifact.Hash }
    $evidence = [ordered]@{
        schema_version = 1
        previous_commit = $previous.Commit
        previous_build_id = $previous.BuildId
        previous_sha256 = $previousByTarget
        patch_commit = $current.Commit
        patch_build_id = $current.BuildId
        patch_sha256 = $currentByTarget
        sequence = @('patch', 'rollback', 'patch_restored')
        final_sha256 = $currentByTarget
    }
    $utf8NoBom = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText($evidencePath, ($evidence | ConvertTo-Json -Depth 5) + [Environment]::NewLine, $utf8NoBom)

    Write-Host 'Local release rollback rehearsal passed:' -ForegroundColor Green
    Write-Host "  Patch:    $($current.BuildId)"
    Write-Host "  Rollback: $($previous.BuildId)"
    Write-Host '  Final state: client and server archives restored and verified'
    Write-Host "  Evidence: $evidencePath"
} finally {
    if (Test-Path -LiteralPath $rehearsalDir) {
        Assert-ChildPath $targetRoot $rehearsalDir
        Remove-Item -LiteralPath $rehearsalDir -Recurse -Force
    }
}
