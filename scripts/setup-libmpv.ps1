[CmdletBinding()]
param(
    [string]$Source = $env:WALLSCAPE_LIBMPV_SOURCE,
    [string]$InstallDir,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$PackageName = "mpv-dev-slim-x86_64-20260622-git-1bcc17a.zip"
$ExpectedSha256 = "EC952CBA1DE9297B75B661993455D485BE84811DB7D5EDC0E7041EBDAB9E55EC"
$GitHubRepo = "thynomex/wallscape"
$GitHubReleaseTag = "libmpv-slim-20260622"
$DefaultSource = "https://github.com/$GitHubRepo/releases/download/$GitHubReleaseTag/$PackageName"

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$VendorDir = Join-Path $ProjectRoot ".vendor"
$CacheDir = Join-Path $VendorDir "cache"
$DefaultInstallDir = Join-Path $VendorDir "mpv-dev"
$BundleDir = Join-Path $ProjectRoot "src-tauri\bundle"
$BundleDll = Join-Path $BundleDir "libmpv-2.dll"

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
    $InstallDir = $DefaultInstallDir
}

function Test-HttpSource {
    param([string]$Value)

    return $Value -match "^https?://"
}

function Save-HttpArchive {
    param(
        [string]$ArchiveSource,
        [string]$ArchivePath
    )

    $TempPath = "$ArchivePath.download"
    if (Test-Path -LiteralPath $TempPath) {
        Remove-Item -LiteralPath $TempPath -Force
    }

    $WebClient = New-Object System.Net.WebClient
    try {
        $WebClient.DownloadFile($ArchiveSource, $TempPath)
        Move-Item -LiteralPath $TempPath -Destination $ArchivePath -Force
    } finally {
        $WebClient.Dispose()
        if (Test-Path -LiteralPath $TempPath) {
            Remove-Item -LiteralPath $TempPath -Force
        }
    }
}

function Save-GitHubReleaseArchive {
    param([string]$ArchivePath)

    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        throw "Download failed and GitHub CLI was not found. Install gh and authenticate, or pass -Source with a local archive path."
    }

    Write-Host "Downloading GitHub release asset with gh"
    & gh release download $GitHubReleaseTag --repo $GitHubRepo --pattern $PackageName --dir $CacheDir --clobber
    if ($LASTEXITCODE -ne 0) {
        throw "gh release download failed for $GitHubRepo@$GitHubReleaseTag."
    }

    if (-not (Test-Path -LiteralPath $ArchivePath)) {
        throw "gh release download did not produce $ArchivePath."
    }
}

function Copy-ArchiveToCache {
    param(
        [string]$ArchiveSource,
        [string]$ArchivePath
    )

    New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null

    if (Test-HttpSource $ArchiveSource) {
        if ($Force -or -not (Test-Path -LiteralPath $ArchivePath)) {
            Write-Host "Downloading $ArchiveSource"
            try {
                Save-HttpArchive -ArchiveSource $ArchiveSource -ArchivePath $ArchivePath
            } catch {
                if ($ArchiveSource -ne $DefaultSource) {
                    throw
                }

                Save-GitHubReleaseArchive -ArchivePath $ArchivePath
            }
        }

        return
    }

    $ResolvedSource = Resolve-Path $ArchiveSource
    if ($ResolvedSource.Path -ne $ArchivePath) {
        Copy-Item -LiteralPath $ResolvedSource.Path -Destination $ArchivePath -Force
    }
}

function Assert-ArchiveHash {
    param([string]$ArchivePath)

    $ActualSha256 = Get-Sha256 -Path $ArchivePath
    if ($ActualSha256 -ne $ExpectedSha256) {
        throw "Hash mismatch for $ArchivePath. Expected $ExpectedSha256 but found $ActualSha256."
    }
}

function Get-Sha256 {
    param([string]$Path)

    $Stream = [System.IO.File]::OpenRead($Path)
    try {
        $Sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            $HashBytes = $Sha256.ComputeHash($Stream)
            return ([System.BitConverter]::ToString($HashBytes) -replace "-", "").ToUpperInvariant()
        } finally {
            $Sha256.Dispose()
        }
    } finally {
        $Stream.Dispose()
    }
}

function Find-PackageRoot {
    param([string]$ExtractDir)

    $Candidates = @((Get-Item -LiteralPath $ExtractDir))
    $Candidates += Get-ChildItem -LiteralPath $ExtractDir -Directory

    foreach ($Candidate in $Candidates) {
        $Path = $Candidate.FullName
        if (
            (Test-Path -LiteralPath (Join-Path $Path "libmpv-2.dll")) -and
            (Test-Path -LiteralPath (Join-Path $Path "mpv.lib")) -and
            (Test-Path -LiteralPath (Join-Path $Path "libmpv.dll.a")) -and
            (Test-Path -LiteralPath (Join-Path $Path "include\mpv\client.h"))
        ) {
            return $Path
        }
    }

    throw "Could not find libmpv dev files in extracted archive."
}

function Install-LibmpvPackage {
    param(
        [string]$PackageRoot,
        [string]$Destination
    )

    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Copy-Item -LiteralPath (Join-Path $PackageRoot "libmpv-2.dll") -Destination (Join-Path $Destination "libmpv-2.dll") -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot "mpv.lib") -Destination (Join-Path $Destination "mpv.lib") -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot "libmpv.dll.a") -Destination (Join-Path $Destination "libmpv.dll.a") -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot "include") -Destination $Destination -Recurse -Force
}

if ([string]::IsNullOrWhiteSpace($Source)) {
    $CachedArchive = Join-Path $CacheDir $PackageName
    if (Test-Path -LiteralPath $CachedArchive) {
        $Source = $CachedArchive
    } else {
        $Source = $DefaultSource
    }
}

$ArchivePath = Join-Path $CacheDir $PackageName
Copy-ArchiveToCache -ArchiveSource $Source -ArchivePath $ArchivePath
Assert-ArchiveHash -ArchivePath $ArchivePath

$ExtractDir = Join-Path $CacheDir "extract-$PID"
if (Test-Path -LiteralPath $ExtractDir) {
    Remove-Item -LiteralPath $ExtractDir -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null

try {
    Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractDir -Force
    $PackageRoot = Find-PackageRoot -ExtractDir $ExtractDir
    Install-LibmpvPackage -PackageRoot $PackageRoot -Destination $InstallDir

    New-Item -ItemType Directory -Force -Path $BundleDir | Out-Null
    Copy-Item -LiteralPath (Join-Path $InstallDir "libmpv-2.dll") -Destination $BundleDll -Force

    Write-Host "Installed slim libmpv to $InstallDir"
    Write-Host "Staged bundled DLL at $BundleDll"
    Write-Host "Archive SHA-256 verified: $ExpectedSha256"
} finally {
    if (Test-Path -LiteralPath $ExtractDir) {
        Remove-Item -LiteralPath $ExtractDir -Recurse -Force
    }
}
