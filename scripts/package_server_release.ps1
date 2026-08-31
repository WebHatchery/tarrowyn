<#
.SYNOPSIS
    Builds and packages the authoritative server for a Rust target.

.DESCRIPTION
    Produces a target-specific internal server ZIP containing the release
    binary, build identity, migration record, and safe deployment notes. The
    current Rust host is the default target; an explicit installed target can
    be supplied for a deployment preflight. It never copies credentials, live
    state, or backup files into the package.
#>
param(
    [string]$OutputPath = 'dist\tarrowyn_server.zip',
    [string]$Target,
    [switch]$AllowDirty
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

function Write-Utf8NoBom {
    param([string]$Path, [string]$Content)

    [IO.File]::WriteAllText($Path, $Content, [Text.UTF8Encoding]::new($false))
}

$projectDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$distDir = [IO.Path]::GetFullPath((Join-Path $projectDir 'dist'))
$output = Resolve-DistPath $projectDir $distDir $OutputPath
$commit = (& git -C $projectDir rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $commit -notmatch '^[0-9a-f]{40}$') {
    throw 'Could not resolve the Tarrowyn Git commit.'
}
$dirtyLines = @(& git -C $projectDir status --porcelain)
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the Tarrowyn working tree.' }
$isDirty = $dirtyLines.Count -gt 0
if ($isDirty -and -not $AllowDirty) {
    throw 'The working tree is dirty. Commit the server candidate or pass -AllowDirty for internal testing.'
}

$hostTarget = $null
foreach ($line in (& rustc -vV)) {
    if ($line -match '^host:\s+(.+)$') {
        $hostTarget = $Matches[1].Trim()
        break
    }
}
$targetWasExplicit = $PSBoundParameters.ContainsKey('Target')
if ($targetWasExplicit) {
    $Target = $Target.Trim()
    if ([string]::IsNullOrWhiteSpace($Target) -or $Target -notmatch '^[A-Za-z0-9][A-Za-z0-9_.-]*$') {
        throw "Target must be a Rust target triple without path separators: $Target"
    }
}
$target = if ($targetWasExplicit) { $Target } else { $hostTarget }
if ([string]::IsNullOrWhiteSpace($target)) { throw 'Could not resolve the Rust host target.' }
$binaryName = if ($target -match 'windows') { 'tarrowyn-server.exe' } else { 'tarrowyn-server' }

Push-Location $projectDir
try {
    $metadata = (& cargo metadata --manifest-path Cargo.toml --locked --no-deps --format-version 1) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Could not read locked Cargo metadata.' }
    $packages = @($metadata.packages | Where-Object { $_.name -eq 'tarrowyn-server' })
    if ($packages.Count -ne 1) { throw 'Expected exactly one tarrowyn-server package in Cargo metadata.' }
    $version = [string]$packages[0].version
    $targetDir = [IO.Path]::GetFullPath([string]$metadata.target_directory)

    $buildArguments = @('build', '-p', 'tarrowyn-server', '--release')
    if ($targetWasExplicit) { $buildArguments += @('--target', $target) }
    & cargo @buildArguments
    if ($LASTEXITCODE -ne 0) { throw 'The authoritative server release build failed.' }
} finally {
    Pop-Location
}

$releaseDir = if ($targetWasExplicit) {
    Join-Path $targetDir (Join-Path $target 'release')
} else {
    Join-Path $targetDir 'release'
}
$binary = Join-Path $releaseDir $binaryName
$migration = Join-Path $projectDir 'server\migrations\0001_initial_world.sql'
$deploymentNotes = Join-Path $projectDir 'docs\SERVER_DEPLOYMENT.md'
foreach ($required in @($binary, $migration, $deploymentNotes)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required server package input is missing: $required"
    }
}

$shortCommit = $commit.Substring(0, 12)
$dirtySuffix = if ($isDirty) { '-dirty' } else { '' }
$stage = Join-Path $distDir ('.tarrowyn-server-' + [Guid]::NewGuid().ToString('N'))
$temporaryZip = Join-Path $distDir ('.tarrowyn-server-' + [Guid]::NewGuid().ToString('N') + '.zip')
Assert-ChildPath $distDir $stage
Assert-ChildPath $distDir $temporaryZip

try {
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $stage 'docs') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $stage 'server\migrations') -Force | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $stage $binaryName) -Force
    Copy-Item -LiteralPath $deploymentNotes -Destination (Join-Path $stage 'docs\SERVER_DEPLOYMENT.md') -Force
    Copy-Item -LiteralPath $migration -Destination (Join-Path $stage 'server\migrations\0001_initial_world.sql') -Force

    $buildInfo = [ordered]@{
        schema_version = 1
        game = 'years_of_tarrowyn'
        package = 'tarrowyn-server'
        version = $version
        build_id = "$version+g$shortCommit$dirtySuffix"
        git_commit = $commit
        working_tree_dirty = $isDirty
        target = $target
        executable = $binaryName
        built_utc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    }
    Write-Utf8NoBom (Join-Path $stage 'BUILD_INFO.json') (($buildInfo | ConvertTo-Json -Depth 4) + [Environment]::NewLine)

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    Compress-Archive -Path "$stage\*" -DestinationPath $temporaryZip -CompressionLevel Optimal -ProgressAction SilentlyContinue
    if (Test-Path -LiteralPath $output -PathType Leaf) {
        Remove-Item -LiteralPath $output -Force
    }
    Move-Item -LiteralPath $temporaryZip -Destination $output
} finally {
    if (Test-Path -LiteralPath $stage) {
        Assert-ChildPath $distDir $stage
        Remove-Item -LiteralPath $stage -Recurse -Force
    }
    if (Test-Path -LiteralPath $temporaryZip) {
        Assert-ChildPath $distDir $temporaryZip
        Remove-Item -LiteralPath $temporaryZip -Force
    }
}

$archiveInfo = Get-Item -LiteralPath $output
$archiveHash = (Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Server release package created for ${target}:" -ForegroundColor Green
Write-Host "  Archive: $output"
Write-Host "  Build:   $version+g$shortCommit$dirtySuffix"
Write-Host "  SHA-256: $archiveHash"
Write-Host "  State and credentials excluded: yes"
