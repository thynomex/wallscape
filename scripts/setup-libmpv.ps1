[CmdletBinding()]
param(
    [string]$Source = $env:WALLSCAPE_LIBMPV_SOURCE,
    [string]$InstallDir,
    [switch]$SkipBundleStaging,
    [switch]$Force,
    # Install the larger upstream package instead of the slim runtime. The slim
    # package is built by scripts/build-libmpv-slim.ps1; see
    # docs/LIBMPV-SLIM-BUILD.md for the comparison and rollback notes.
    [switch]$FullRuntime
)

$ErrorActionPreference = "Stop"

$SlimPackage = @{
    Name           = "mpv-dev-slim-x86_64-c152964208.zip"
    Sha256         = "B22D705DD7CA51EC0505F6F1FB50AA069E079F5A3D390E41C95D241BFF40117B"
    GitHubRepo     = "thynomex/wallscape"
    GitHubTag      = "libmpv-slim-20261024"
    InstallDirName = "mpv-dev-slim"
}
$FullPackage = @{
    Name           = "mpv-dev-x86_64-20261005-git-c152964208.7z"
    Sha256         = "7D9DD506DA6B77E6054F56052F7423D9F7EED764F85154A6221925ABA37DBFB2"
    GitHubRepo     = "shinchiro/mpv-winbuild-cmake"
    GitHubTag      = "20261005"
    InstallDirName = "mpv-dev-filters"
}

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$VendorDir = Join-Path $ProjectRoot ".vendor"
$CacheDir = Join-Path $VendorDir "cache"

$SourceSpecified = -not [string]::IsNullOrWhiteSpace($Source)
$InstallDirSpecified = -not [string]::IsNullOrWhiteSpace($InstallDir)
$SourceName = if ($SourceSpecified) { [System.IO.Path]::GetFileName($Source) } else { "" }
# An explicit archive decides the package; a mismatched file then fails the
# checksum check instead of being silently accepted as the other runtime.
$UseSlim = if ($FullRuntime) { $false } elseif ($SourceSpecified) { $SourceName -eq $SlimPackage.Name } else { $true }

function Select-Package {
    param([hashtable]$Selected)

    $script:Package = $Selected
    $script:PackageName = $Selected.Name
    $script:ExpectedSha256 = $Selected.Sha256
    $script:GitHubRepo = $Selected.GitHubRepo
    $script:GitHubReleaseTag = $Selected.GitHubTag
    $script:DefaultSource = "https://github.com/$($Selected.GitHubRepo)/releases/download/$($Selected.GitHubTag)/$($Selected.Name)"
}

Select-Package $(if ($UseSlim) { $SlimPackage } else { $FullPackage })

$BundleDir = Join-Path $ProjectRoot "src-tauri\bundle"
$BundleDll = Join-Path $BundleDir "libmpv-2.dll"

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
            (Test-Path -LiteralPath (Join-Path $Path "libmpv.dll.a")) -and
            (Test-Path -LiteralPath (Join-Path $Path "include\mpv\client.h"))
        ) {
            return $Path
        }
    }

    throw "Could not find libmpv dev files in extracted archive."
}

function Expand-LibmpvArchive {
    param([string]$ArchivePath, [string]$Destination)

    $SevenZip = Get-Command 7z, 7za -ErrorAction SilentlyContinue | Select-Object -First 1
    $SevenZipPath = if ($SevenZip) { $SevenZip.Source } else { $null }
    if (-not $SevenZipPath) {
        foreach ($ProgramDirectory in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
            if ($ProgramDirectory) {
                $Candidate = Join-Path $ProgramDirectory "7-Zip\7z.exe"
                if (Test-Path -LiteralPath $Candidate) {
                    $SevenZipPath = $Candidate
                    break
                }
            }
        }
    }
    # The slim package is a plain zip; PowerShell can extract it without 7-Zip.
    if (-not $SevenZipPath -and $ArchivePath.EndsWith(".zip", [System.StringComparison]::OrdinalIgnoreCase)) {
        Expand-Archive -LiteralPath $ArchivePath -DestinationPath $Destination -Force
        return
    }
    if (-not $SevenZipPath) {
        throw "Extracting the pinned libmpv package requires 7-Zip or NanaZip (7z/7za on PATH). Install one, then run npm run setup:mpv again."
    }

    & $SevenZipPath x $ArchivePath "-o$Destination" -y | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Could not extract libmpv archive: $ArchivePath"
    }
}

function Install-LibmpvPackage {
    param(
        [string]$PackageRoot,
        [string]$Destination
    )

    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Copy-Item -LiteralPath (Join-Path $PackageRoot "libmpv-2.dll") -Destination (Join-Path $Destination "libmpv-2.dll") -Force
    # The upstream package ships COFF import objects in a GNU archive. MSVC
    # accepts that archive; give it the library name requested by libmpv2-sys.
    $MsvcImportLibrary = Join-Path $PackageRoot "mpv.lib"
    if (-not (Test-Path -LiteralPath $MsvcImportLibrary)) {
        $MsvcImportLibrary = Join-Path $PackageRoot "libmpv.dll.a"
    }
    Copy-Item -LiteralPath $MsvcImportLibrary -Destination (Join-Path $Destination "mpv.lib") -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot "libmpv.dll.a") -Destination (Join-Path $Destination "libmpv.dll.a") -Force
    Copy-Item -LiteralPath (Join-Path $PackageRoot "include") -Destination $Destination -Recurse -Force
}

function Resolve-DefaultSource {
    $CachedArchive = Join-Path $CacheDir $PackageName
    if (Test-Path -LiteralPath $CachedArchive) { return $CachedArchive }
    return $DefaultSource
}

if (-not $SourceSpecified) {
    $Source = Resolve-DefaultSource
}

$ArchivePath = Join-Path $CacheDir $PackageName
try {
    Copy-ArchiveToCache -ArchiveSource $Source -ArchivePath $ArchivePath
} catch {
    # Only the default slim download falls back. Explicit sources, -FullRuntime,
    # and checksum failures below still stop the setup.
    if (-not $UseSlim -or $SourceSpecified) {
        throw
    }
    Write-Warning "Could not get the slim libmpv archive ($($_.Exception.Message)). Installing the larger upstream runtime instead; the MSI will be about 55 MB rather than 22 MB."
    # Drop a partial download so the next run retries instead of failing its checksum.
    if ((Test-HttpSource $Source) -and (Test-Path -LiteralPath $ArchivePath)) {
        Remove-Item -LiteralPath $ArchivePath -Force
    }
    $UseSlim = $false
    Select-Package $FullPackage
    $Source = Resolve-DefaultSource
    $ArchivePath = Join-Path $CacheDir $PackageName
    Copy-ArchiveToCache -ArchiveSource $Source -ArchivePath $ArchivePath
}
Assert-ArchiveHash -ArchivePath $ArchivePath

if (-not $InstallDirSpecified) {
    $InstallDir = Join-Path $VendorDir $Package.InstallDirName
}

$ExtractDir = Join-Path $CacheDir "extract-$PID"
$ResolvedCacheDir = [System.IO.Path]::GetFullPath($CacheDir).TrimEnd('\') + '\'
$ResolvedExtractDir = [System.IO.Path]::GetFullPath($ExtractDir)
if (-not $ResolvedExtractDir.StartsWith($ResolvedCacheDir, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean extraction directory outside the libmpv cache: $ResolvedExtractDir"
}
if (Test-Path -LiteralPath $ExtractDir) {
    Remove-Item -LiteralPath $ExtractDir -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null

try {
    Expand-LibmpvArchive -ArchivePath $ArchivePath -Destination $ExtractDir
    $PackageRoot = Find-PackageRoot -ExtractDir $ExtractDir
    Install-LibmpvPackage -PackageRoot $PackageRoot -Destination $InstallDir

    # src-tauri/build.rs resolves .vendor\mpv-dev-slim before the upstream
    # directory and restages its DLL on every build. Remove a slim install left
    # from an earlier run so the upstream runtime installed here is the one used.
    $SlimInstallDir = Join-Path $VendorDir $SlimPackage.InstallDirName
    if (-not $UseSlim -and -not $InstallDirSpecified -and (Test-Path -LiteralPath $SlimInstallDir)) {
        $ResolvedVendorDir = [System.IO.Path]::GetFullPath($VendorDir).TrimEnd('\') + '\'
        $ResolvedSlimDir = [System.IO.Path]::GetFullPath($SlimInstallDir)
        if (-not $ResolvedSlimDir.StartsWith($ResolvedVendorDir, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove a slim runtime outside .vendor: $ResolvedSlimDir"
        }
        Remove-Item -LiteralPath $SlimInstallDir -Recurse -Force
        Write-Host "Removed $SlimInstallDir so the build uses the upstream runtime"
    }

    if (-not $SkipBundleStaging) {
        New-Item -ItemType Directory -Force -Path $BundleDir | Out-Null
        Copy-Item -LiteralPath (Join-Path $InstallDir "libmpv-2.dll") -Destination $BundleDll -Force
        Write-Host "Staged bundled DLL at $BundleDll"
    }

    Write-Host "Installed libmpv with frame filters to $InstallDir"
    Write-Host "Archive SHA-256 verified: $ExpectedSha256"
} finally {
    if (Test-Path -LiteralPath $ExtractDir) {
        Remove-Item -LiteralPath $ExtractDir -Recurse -Force
    }
}
