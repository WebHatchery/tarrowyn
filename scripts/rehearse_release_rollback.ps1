<#
.SYNOPSIS
    Rehearses patch deployment, rollback, and patch restoration locally.

.DESCRIPTION
    Uses a commit-addressed preserved candidate as the previous release and
    the current manifest as the patch. The exact Windows archive is copied
    through an isolated target directory and its manifest/checksum identity is
    verified at every switch. No live deployment or world state is changed.
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

function Get-WindowsRecord($Manifest) {
    $records = @($Manifest.archives | Where-Object { [string]$_.target -eq 'windows' })
    if ($records.Count -ne 1) { throw 'Release manifest must identify exactly one Windows archive.' }
    return $records[0]
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
    if ($commit -notmatch '^[0-9a-f]{40}$') { throw "Rollback manifest has an invalid Git commit: $ManifestPath" }

    $record = Get-WindowsRecord $manifest
    $filename = [string]$record.filename
    if ([string]::IsNullOrWhiteSpace($filename) -or [IO.Path]::GetFileName($filename) -ne $filename) {
        throw "Rollback manifest contains an unsafe Windows archive filename: $filename"
    }
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

    return [pscustomobject]@{
        ManifestPath = $ManifestPath
        ArchivePath = $archive
        Commit = $commit
        BuildId = [string]$manifest.build_id
        Hash = $hash
        Bytes = [long]$record.bytes
    }
}

function Assert-DeployedCandidate {
    param([string]$ArchivePath, $Candidate)

    $hash = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne $Candidate.Hash) { throw "Rehearsal archive hash mismatch: $ArchivePath" }
    if ((Get-Item -LiteralPath $ArchivePath).Length -ne $Candidate.Bytes) {
        throw "Rehearsal archive size mismatch: $ArchivePath"
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
$deployedArchive = Join-Path $rehearsalDir 'years_of_tarrowyn_windows.zip'
$evidencePath = Join-Path $targetRoot 'tarrowyn-release-rollback-rehearsal.json'
Assert-ChildPath $targetRoot $rehearsalDir

try {
    New-Item -ItemType Directory -Path $rehearsalDir -Force | Out-Null
    Copy-Item -LiteralPath $current.ArchivePath -Destination $deployedArchive
    Assert-DeployedCandidate $deployedArchive $current
    Copy-Item -LiteralPath $previous.ArchivePath -Destination $deployedArchive -Force
    Assert-DeployedCandidate $deployedArchive $previous
    Copy-Item -LiteralPath $current.ArchivePath -Destination $deployedArchive -Force
    Assert-DeployedCandidate $deployedArchive $current

    $evidence = [ordered]@{
        schema_version = 1
        previous_commit = $previous.Commit
        previous_build_id = $previous.BuildId
        previous_sha256 = $previous.Hash
        patch_commit = $current.Commit
        patch_build_id = $current.BuildId
        patch_sha256 = $current.Hash
        sequence = @('patch', 'rollback', 'patch_restored')
        final_sha256 = $current.Hash
    }
    $utf8NoBom = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText($evidencePath, ($evidence | ConvertTo-Json -Depth 4) + [Environment]::NewLine, $utf8NoBom)

    Write-Host 'Local release rollback rehearsal passed:' -ForegroundColor Green
    Write-Host "  Patch:    $($current.BuildId) $($current.Hash)"
    Write-Host "  Rollback: $($previous.BuildId) $($previous.Hash)"
    Write-Host "  Final state: patch restored and hash verified"
    Write-Host "  Evidence: $evidencePath"
} finally {
    if (Test-Path -LiteralPath $rehearsalDir) {
        Assert-ChildPath $targetRoot $rehearsalDir
        Remove-Item -LiteralPath $rehearsalDir -Recurse -Force
    }
}
