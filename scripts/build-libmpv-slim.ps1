[CmdletBinding()]
param(
    # Isolated workspace. Nothing outside this directory is modified.
    [string]$Root = (Join-Path ([System.IO.Path]::GetTempPath()) "wallscape-libmpv-slim"),
    [ValidateSet("all", "dependencies", "ffmpeg", "mpv", "prepare")]
    [string]$Stage = "all",
    [int]$Jobs = 8,
    [switch]$SkipToolchain,
    # Build even when the installed UCRT64 packages differ from the lock. The
    # result is then a new candidate that needs the full validation again.
    [switch]$AllowToolchainDrift
)

$ErrorActionPreference = "Stop"

# Pinned inputs. The mpv and FFmpeg revisions match upstream package
# mpv-dev-x86_64-20261005-git-c152964208.7z, so a candidate differs from that
# runtime only in its feature configuration.
# The self-extracting base archive needs no tar or xz. Windows tar.exe hands
# .tar.xz to whatever xz.exe is on PATH and can deadlock on the pipe.
$MSYS2Archive = "msys2-base-x86_64-20260927.sfx.exe"
$MSYS2Url = "https://github.com/msys2/msys2-installer/releases/download/2026-09-27/$MSYS2Archive"
$MSYS2Sha256 = "AD336CCCFDA47758B5E15CDA993FBBA421115CB0B126697DAEF1EE4DFE37209F"
# The UCRT64 compiler and libraries determine the DLL, so they are installed at
# the exact versions in this lock: gcc, binutils, nasm, meson, ninja, cmake,
# pkgconf, python, zlib, dav1d, libass, shaderc, SPIRV-Cross/Tools, glslang, and
# their dependencies. mpv's D3D11 output needs shaderc and SPIRV-Cross.
$ToolchainLock = Join-Path $PSScriptRoot "libmpv-slim-toolchain.lock"
$ToolchainMirror = "https://repo.msys2.org/mingw/ucrt64"
# MSYS utilities used by the build scripts. They do not affect the compiled DLL.
$MsysUtilities = @("git", "make", "patch", "zip")
$Sources = [ordered]@{
    mpv = @{ Url = "https://github.com/mpv-player/mpv.git"; Ref = "c1529642089bfebfc928a1c1664638a7a5d219ba" }
    ffmpeg = @{ Url = "https://github.com/FFmpeg/FFmpeg.git"; Ref = "a35c8799920a93b99781eab6e70a5795ee7d182b" }
    libplacebo = @{ Url = "https://github.com/haasn/libplacebo.git"; Ref = "92b5ac6db79f4d680eb656692f7bf51e9606f42a" }
}

$Root = [System.IO.Path]::GetFullPath($Root)
$Msys = Join-Path $Root "msys64"
$Bash = Join-Path $Msys "usr\bin\bash.exe"

function Invoke-MsysBash {
    param([string]$Command)
    $env:MSYSTEM = "UCRT64"
    $env:CHERE_INVOKING = "1"
    & $Bash -lc $Command
    if ($LASTEXITCODE -ne 0) {
        throw "MSYS2 command failed ($LASTEXITCODE): $Command"
    }
}

function Install-Toolchain {
    New-Item -ItemType Directory -Force -Path $Root | Out-Null
    if (-not (Test-Path -LiteralPath $Bash)) {
        $Archive = Join-Path $Root $MSYS2Archive
        if (-not (Test-Path -LiteralPath $Archive)) {
            Write-Host "Downloading $MSYS2Url"
            Invoke-WebRequest -Uri $MSYS2Url -OutFile $Archive
        }
        $ArchiveSha256 = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash
        if ($ArchiveSha256 -ne $MSYS2Sha256) {
            throw "Hash mismatch for $Archive. Expected $MSYS2Sha256 but found $ArchiveSha256."
        }
        Write-Host "Extracting $MSYS2Archive"
        # 7-Zip self-extractor: -y answers prompts, -o sets the parent of msys64.
        & $Archive -y "-o$Root" | Out-Null
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $Bash)) { throw "Could not extract $Archive" }
    }

    Invoke-MsysBash "/usr/bin/bash -lc 'true'"
    $MissingUtilities = @(Invoke-MsysBash ("for p in " + ($MsysUtilities -join " ") + "; do pacman -Q `$p > /dev/null 2>&1 || echo `$p; done"))
    if ($MissingUtilities.Count -gt 0) {
        Write-Host "Installing MSYS build utilities: $($MissingUtilities -join ', ')"
        Invoke-WithRetry { Invoke-MsysBash ("pacman -Sy --needed --noconfirm " + ($MissingUtilities -join " ")) }
    }

    # Install exact versions from the mirror archive rather than whatever is
    # current, so a new workspace builds with the toolchain that was validated.
    if ((Get-ToolchainDrift).Count -gt 0) {
        Write-Host "Installing the locked UCRT64 toolchain"
        $Urls = Get-LockedToolchain | ForEach-Object {
            $Name, $Version = $_ -split " ", 2
            "$ToolchainMirror/$Name-$Version-any.pkg.tar.zst"
        }
        Invoke-WithRetry { Invoke-MsysBash ("pacman -U --needed --noconfirm " + ($Urls -join " ")) }
    }
}

function Invoke-WithRetry {
    param([scriptblock]$Action)
    # MSYS2 mirrors return 429 under load; retry once before giving up.
    try {
        & $Action
    } catch {
        Write-Host "Package installation failed; retrying once"
        & $Action
    }
}

function Get-LockedToolchain {
    Get-Content -LiteralPath $ToolchainLock | ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith("#") }
}

function Get-ToolchainDrift {
    $Installed = @(Invoke-MsysBash "pacman -Q | grep '^mingw-w64-ucrt-x86_64-' || true" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    @(Compare-Object -ReferenceObject @(Get-LockedToolchain) -DifferenceObject $Installed | ForEach-Object {
        if ($_.SideIndicator -eq "<=") { "locked, not installed: $($_.InputObject)" } else { "installed, not locked: $($_.InputObject)" }
    })
}

function Sync-Sources {
    New-Item -ItemType Directory -Force -Path (Join-Path $Root "sources") | Out-Null
    $LibPlaceboDir = Join-Path $Root "native-deps\libplacebo"
    foreach ($Name in $Sources.Keys) {
        $Spec = $Sources[$Name]
        $Destination = if ($Name -eq "libplacebo") { $LibPlaceboDir } else { Join-Path $Root "sources\$Name" }
        if (-not (Test-Path -LiteralPath (Join-Path $Destination ".git"))) {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
            Write-Host "Cloning $Name at $($Spec.Ref)"
            # Windows Git converts line endings on checkout by default, which
            # marks every text file modified and leaks "-dirty" into the mpv
            # version string. Keep the working tree pristine.
            git clone --filter=blob:none --config core.autocrlf=false $Spec.Url $Destination
            if ($LASTEXITCODE -ne 0) { throw "Could not clone $Name" }
        }
        git -C $Destination config core.autocrlf false
        git -C $Destination fetch --depth 1 origin $Spec.Ref
        if ($LASTEXITCODE -ne 0) { throw "Could not fetch $($Spec.Ref) for $Name" }
        git -C $Destination checkout --detach $Spec.Ref
        if ($LASTEXITCODE -ne 0) { throw "Could not check out $($Spec.Ref) for $Name" }
        # Normalise any checkout created before the autocrlf setting existed.
        git -C $Destination checkout -- .
    }
}

if (-not $SkipToolchain) {
    Install-Toolchain
}
$Drift = @(Get-ToolchainDrift)
if ($Drift.Count -gt 0) {
    $Message = "Installed UCRT64 packages differ from $($ToolchainLock):`n  " + ($Drift -join "`n  ")
    if (-not $AllowToolchainDrift) {
        throw "$Message`nPass -AllowToolchainDrift to build anyway; the result then needs the full validation again."
    }
    Write-Warning $Message
}
Sync-Sources

if ($Stage -eq "prepare") {
    Write-Host "Workspace prepared at $Root"
    return
}

$RootUnix = ($Root -replace '\\', '/') -replace '^([A-Za-z]):', '/$1'
$ScriptUnix = ((Resolve-Path (Join-Path $PSScriptRoot "build-libmpv-slim.sh")).Path -replace '\\', '/') -replace '^([A-Za-z]):', '/$1'
# MSYS2 needs a home directory; a bare Windows profile path breaks Meson's
# CMake dependency fallback. Build the Windows form here rather than calling
# cygpath, which is not on PATH outside an MSYS2 or Git for Windows shell.
$HomeWindows = Join-Path $Root "home"
Invoke-MsysBash "mkdir -p '$RootUnix/home' && export USERPROFILE='$HomeWindows' && WALLSCAPE_BUILD_JOBS=$Jobs bash '$ScriptUnix' '$RootUnix' '$Stage'"

Write-Host "Candidate package: $Root\candidate"
