[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputDir,
    [string]$FFmpeg = "ffmpeg"
)
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
$Cases = @(
    @{ Name = "h264.mp4"; Codec = @("-c:v", "libx264", "-preset", "ultrafast"); Decoder = "h264" },
    @{ Name = "h264.mov"; Codec = @("-c:v", "libx264", "-preset", "ultrafast"); Decoder = "h264" },
    @{ Name = "h264.m4v"; Codec = @("-c:v", "libx264", "-preset", "ultrafast", "-f", "mp4"); Decoder = "h264" },
    @{ Name = "hevc.mp4"; Codec = @("-c:v", "libx265", "-preset", "ultrafast", "-x265-params", "pools=2:frame-threads=2:log-level=error"); Decoder = "hevc" },
    @{ Name = "vp8.webm"; Codec = @("-c:v", "libvpx", "-deadline", "realtime", "-cpu-used", "8"); Decoder = "vp8" },
    @{ Name = "vp9.webm"; Codec = @("-c:v", "libvpx-vp9", "-deadline", "realtime", "-cpu-used", "8"); Decoder = "vp9" },
    @{ Name = "av1.mkv"; Codec = @("-c:v", "libaom-av1", "-cpu-used", "8", "-row-mt", "1"); Decoder = "av1" },
    @{ Name = "mpeg4.avi"; Codec = @("-c:v", "mpeg4", "-q:v", "4"); Decoder = "mpeg4" },
    @{ Name = "wmv2.wmv"; Codec = @("-c:v", "wmv2", "-b:v", "1M"); Decoder = "wmv2" },
    @{ Name = "mjpeg.avi"; Codec = @("-c:v", "mjpeg", "-q:v", "5"); Decoder = "mjpeg" },
    @{ Name = "mpeg2.ts"; Codec = @("-c:v", "mpeg2video", "-f", "mpegts"); Decoder = "mpeg2video" },
    @{ Name = "flv.flv"; Codec = @("-c:v", "flv"); Decoder = "flv1" },
    @{ Name = "gif.gif"; Codec = @("-c:v", "gif"); Decoder = "gif" },
    @{ Name = "raw.y4m"; Codec = @("-c:v", "wrapped_avframe", "-f", "yuv4mpegpipe"); Decoder = "rawvideo" }
)
$Manifest = @()
foreach ($Case in $Cases) {
    $Path = Join-Path $OutputDir $Case.Name
    $Arguments = @("-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=320x180:rate=60:duration=2", "-an", "-threads", "2") + $Case.Codec + @($Path)
    & $FFmpeg @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Fixture creation failed: $($Case.Name)" }
    $Manifest += [ordered]@{ file = $Case.Name; codec = $Case.Decoder; sha256 = (Get-FileHash -LiteralPath $Path).Hash; arguments = $Arguments }
}
$DesktopPath = Join-Path $OutputDir "desktop-h264.mp4"
& $FFmpeg -hide_banner -loglevel error -y -f lavfi -i "testsrc2=size=1920x1080:rate=60:duration=3" -an -c:v libx264 -preset ultrafast -threads 2 -pix_fmt yuv420p $DesktopPath
if ($LASTEXITCODE -ne 0) { throw "Desktop fixture creation failed" }
[System.IO.File]::WriteAllText((Join-Path $OutputDir "unsupported.mp4"), "Wallscape invalid-media validation fixture")
$Manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $OutputDir "manifest.json") -Encoding UTF8
& $FFmpeg -version | Set-Content -LiteralPath (Join-Path $OutputDir "ffmpeg-version.txt") -Encoding UTF8
Write-Host "Created $($Cases.Count) synthetic codec/container fixtures and desktop sample in $OutputDir"
