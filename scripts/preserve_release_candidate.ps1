<#
.SYNOPSIS
    Preserves an exact clean release candidate for a future rollback.

.DESCRIPTION
    Verifies the release manifest, both archive checksums, and the sidecars,
    then copies them into a commit-addressed directory under dist/history.
    Existing evidence is never replaced by a different byte sequence.
#>
param(
    [string]$ManifestPath = 'dist\tarrowyn_release_manifest.json'
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

function Copy-WithoutReplacingDifferentFile {
    param([string]$Source, [string]$Destination)

    if (Test-Path -LiteralPath $Destination -PathType Leaf) {
        $sourceHash = (Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash
        $destinationHash = (Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash
        if ($sourceHash -ne $destinationHash) {
            throw "Refusing to replace different preserved evidence: $Destination"
        }
        return
    }
    Copy-Item -LiteralPath $Source -Destination $Destination
}

function Resolve-DistFile {
    param([string]$ProjectDir, [string]$DistDir, [string]$Path)

    $resolved = if ([IO.Path]::IsPathRooted($Path)) {
        [IO.Path]::GetFullPath($Path)
    } else {
        [IO.Path]::GetFullPath((Join-Path $ProjectDir $Path))
    }
    Assert-ChildPath $DistDir $resolved
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "Required release evidence is missing: $resolved"
    }
    return $resolved
}

$projectDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$distDir = [IO.Path]::GetFullPath((Join-Path $projectDir 'dist'))
$manifestFile = Resolve-DistFile $projectDir $distDir $ManifestPath
$manifest = Get-Content -LiteralPath $manifestFile -Raw | ConvertFrom-Json

if ($manifest.schema_version -ne 1) { throw 'Unsupported release manifest schema.' }
if ($manifest.working_tree_dirty -ne $false) {
    throw 'Only a clean-build manifest can become rollback evidence.'
}
$commit = [string]$manifest.git_commit
if ($commit -notmatch '^[0-9a-f]{40}$') { throw 'Release manifest has an invalid Git commit.' }

$archiveRecords = @($manifest.archives)
if ($archiveRecords.Count -ne 3) { throw 'Release manifest must identify exactly three archives.' }
foreach ($record in $archiveRecords) {
    $filename = [string]$record.filename
    if ([string]::IsNullOrWhiteSpace($filename) -or
        [IO.Path]::GetFileName($filename) -ne $filename) {
        throw "Release manifest contains an unsafe archive filename: $filename"
    }
    $archive = Resolve-DistFile $projectDir $distDir (Join-Path 'dist' $filename)
    $actualHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne ([string]$record.sha256).ToLowerInvariant()) {
        throw "Archive hash does not match the release manifest: $archive"
    }
    if ((Get-Item -LiteralPath $archive).Length -ne [long]$record.bytes) {
        throw "Archive size does not match the release manifest: $archive"
    }

    $sidecar = "$archive.sha256"
    if (-not (Test-Path -LiteralPath $sidecar -PathType Leaf)) {
        throw "Archive checksum sidecar is missing: $sidecar"
    }
    $sidecarParts = (Get-Content -LiteralPath $sidecar -Raw).Trim() -split '\s+'
    if ($sidecarParts.Count -lt 2 -or
        $sidecarParts[0].ToLowerInvariant() -ne $actualHash -or
        $sidecarParts[1] -ne $filename) {
        throw "Archive checksum sidecar does not match the release manifest: $sidecar"
    }
}

$historyRoot = Join-Path $distDir 'history'
$preservedDir = Join-Path $historyRoot $commit
Assert-ChildPath $distDir $historyRoot
Assert-ChildPath $historyRoot $preservedDir
New-Item -ItemType Directory -Path $preservedDir -Force | Out-Null
Copy-WithoutReplacingDifferentFile $manifestFile (Join-Path $preservedDir $manifestFile.Name)

foreach ($record in $archiveRecords) {
    $filename = [string]$record.filename
    $archive = Join-Path $distDir $filename
    Copy-WithoutReplacingDifferentFile $archive (Join-Path $preservedDir $filename)
    Copy-WithoutReplacingDifferentFile "$archive.sha256" (Join-Path $preservedDir "$filename.sha256")
}

Write-Host 'Release candidate preserved:' -ForegroundColor Green
Write-Host "  Build ID: $($manifest.build_id)"
Write-Host "  Commit:   $commit"
Write-Host "  Directory: $preservedDir"
