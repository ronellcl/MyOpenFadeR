<#
.SYNOPSIS
Builds the Windows installer: stages everything the program finds next to openfader.exe and compiles it with NSIS.

.DESCRIPTION
The payload is the release layout of .github/bundle.py plus what makes a fresh machine work:

  openfader.exe, the DLLs it needs, the Visual C++ runtime   (the program)
  tools\ffmpeg.exe, tools\ffprobe.exe                        (video, mp3 and the yt-dlp plugin)
  python\                                                    (the interpreter of the Python plugins)
  extension\plugin\<plugin>\                                 (yt-dlp and the importers/BFX of the repository)
  extension\skin\*.json, mappings\*.xml                      (themes and controller mappings)
  LICENSE, README.md

Two add-ons travel inside the installer, so a fresh machine needs nothing else:

  extension\plugin\yt-dlp\bin\yt-dlp.exe   the yt-dlp program (the plugin copies it to the user's data
                                           folder the first time it runs, where it can update itself)
  extension\plugin\deezer\deezer-engine.exe  the Deezer engine, a native program (no Node, no deemix, no Python)

`tools\` and `python\` come first in the PATH of the program (src/main.rs), so nothing is added to the system PATH.

.PARAMETER Version
Version shown by the wizard and in Add/Remove Programs. Default: the `version` of Cargo.toml.

.PARAMETER OutDir
Where the payload (`payload\`) and the setup.exe land. Default: dist\windows.

.PARAMETER FfmpegBin
Folder with ffmpeg.exe and ffprobe.exe to bundle. Default: C:\Program Files\FFmpeg\bin.

.PARAMETER PythonDir
Folder with an *embeddable* Python (python.exe + python3xx.zip + the .pyd/.dll files), as unpacked from
https://www.python.org/ftp/python/<version>/python-<version>-embed-amd64.zip . Required.

.PARAMETER Makensis
makensis.exe of NSIS 3 (https://nsis.sourceforge.io). Default: the one on the PATH.

.PARAMETER PerUser
Per-user build (%LOCALAPPDATA%, no UAC). The default is per-machine (Program Files, asks for administrator).
It is also how build-installer.ps1 self-tests: it installs that variant silently and starts the app from it.

.PARAMETER WithoutYtdlp
Leaves the yt-dlp program out. Off by default: the installer ships it, so the plugin works without downloading
anything; the plugin copies it to the user's data folder (a copy under Program Files cannot update itself).

.PARAMETER SkipBuild
Uses target\release\openfader.exe as it is instead of running cargo build --release.

.EXAMPLE
.\build-installer.ps1 -PythonDir "$env:TEMP\py-embed" -Makensis "C:\Tools\nsis\makensis.exe"
#>
param(
    [string]$Version,
    [string]$OutDir,
    [string]$FfmpegBin = "C:\Program Files\FFmpeg\bin",
    [string]$PythonDir,
    [string]$Makensis,
    [switch]$PerUser,
    [switch]$WithoutYtdlp,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $OutDir) { $OutDir = Join-Path $root "dist\windows" }
$OutDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutDir)
$release = Join-Path $root "target\release"

if (-not $Version) {
    $line = Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version = "(.+)"' | Select-Object -First 1
    if (-not $line) { throw "no version in Cargo.toml: pass -Version" }
    $Version = $line.Matches[0].Groups[1].Value
}

if (-not $SkipBuild) {
    Write-Host "cargo build --release"
    & cargo build --release --manifest-path (Join-Path $root "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$exe = Join-Path $release "openfader.exe"
if (-not (Test-Path $exe)) { throw "$exe is missing: build the program first (or drop -SkipBuild)" }
if (-not $PythonDir -or -not (Test-Path (Join-Path $PythonDir "python.exe"))) {
    throw "-PythonDir must point at an unpacked python-<version>-embed-amd64.zip (python.exe inside)"
}
if (-not (Test-Path (Join-Path $FfmpegBin "ffmpeg.exe"))) { throw "$FfmpegBin has no ffmpeg.exe" }
if (-not $Makensis) {
    $cmd = Get-Command makensis -ErrorAction SilentlyContinue
    if (-not $cmd) { throw "makensis.exe of NSIS 3 is needed: put it on the PATH or pass -Makensis" }
    $Makensis = $cmd.Source
}

$stage = Join-Path $OutDir "payload"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force $stage | Out-Null

function Copy-Folder([string]$From, [string]$To, [string[]]$SkipDirs, [string[]]$SkipFiles) {
    New-Item -ItemType Directory -Force $To | Out-Null
    $robocopy = @($From, $To, "/E", "/NFL", "/NDL", "/NJH", "/NJS", "/NP", "/R:1", "/W:1")
    if ($SkipDirs.Count) { $robocopy += "/XD"; $robocopy += $SkipDirs }
    if ($SkipFiles.Count) { $robocopy += "/XF"; $robocopy += $SkipFiles }
    & robocopy @robocopy | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "robocopy $From -> $To failed ($LASTEXITCODE)" }
    $global:LASTEXITCODE = 0
}

# --- the program ---------------------------------------------------------------------------------------------------

Copy-Item $exe $stage
Get-ChildItem (Join-Path $release "*.dll") -ErrorAction SilentlyContinue | Copy-Item -Destination $stage
# The Visual C++ runtime next to the program: it starts on a machine that never installed the redistributable.
foreach ($dll in "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll") {
    $from = Join-Path $env:WINDIR "System32\$dll"
    if (-not (Test-Path $from)) { throw "$from is missing" }
    Copy-Item $from $stage
}
Copy-Item (Join-Path $root "LICENSE") $stage
Copy-Item (Join-Path $root "README.md") $stage

# --- the add-ons that travel inside the installer ---------------------------------------------------------------

# The Deezer engine: a native program (no Node, no deemix, no Python). The plugin runs it from its own `bin` folder.
$crate = Join-Path $root "plugins-repo\deezer"
Write-Host "cargo build --release (deezer-engine)"
& cargo build --release --manifest-path (Join-Path $crate "Cargo.toml") --target-dir (Join-Path $root "plugins-repo\target\deezer")
if ($LASTEXITCODE -ne 0) { throw "the deezer engine did not build" }
$built = Join-Path $root "plugins-repo\target\deezer\release\deezer-engine.exe"
if (-not (Test-Path $built)) { throw "$built is missing" }
# Windows runs a program with its extension: the manifest asks for `bin/deezer-engine`, the file is the .exe.
New-Item -ItemType Directory -Force (Join-Path $crate "bin") | Out-Null
try {
    Copy-Item $built (Join-Path $crate "bin\deezer-engine.exe") -Force
} catch {
    # The app may be running the engine from that folder (a persistent plugin holds its program open): the payload
    # gets the fresh build all the same.
    Write-Warning "the deezer engine of plugins-repo is in use: it was not replaced there (the installer ships the new one)"
}

# --- ffmpeg and ffprobe ---------------------------------------------------------------------------------------------

$tools = Join-Path $stage "tools"
New-Item -ItemType Directory -Force $tools | Out-Null
foreach ($tool in "ffmpeg.exe", "ffprobe.exe") {
    Copy-Item (Join-Path $FfmpegBin $tool) $tools
}
$ffmpegReadme = Join-Path (Split-Path $FfmpegBin -Parent) "README.txt"
if (Test-Path $ffmpegReadme) { Copy-Item $ffmpegReadme (Join-Path $tools "ffmpeg-README.txt") }

# A video seek starts ffmpeg again, so a build that takes a second to start makes every seek wait that long: the
# full build of gyan.dev (400 MB) needs 1.2 s, the essentials build (100 MB) 40 ms. Warn before shipping the slow one.
$started = Measure-Command { & (Join-Path $FfmpegBin "ffmpeg.exe") -version 2>&1 | Out-Null }
$startMs = [math]::Round($started.TotalMilliseconds)
if ($startMs -gt 400) {
    Write-Warning ("the ffmpeg of {0} takes {1} ms to start: every video seek will wait that long. " +
        "The essentials build (https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip) starts in 40 ms." -f $FfmpegBin, $startMs)
} else {
    Write-Host ("ffmpeg starts in {0} ms: video seeks will be quick" -f $startMs)
}

# --- Python for the .py plugins -------------------------------------------------------------------------------------

Copy-Folder $PythonDir (Join-Path $stage "python") @() @()

# --- themes, mappings, plugins --------------------------------------------------------------------------------------

$skins = Join-Path $stage "extension\skin"
New-Item -ItemType Directory -Force $skins | Out-Null
Get-ChildItem (Join-Path $root "themes-repo\*.json") | Copy-Item -Destination $skins

$maps = Join-Path $stage "mappings"
New-Item -ItemType Directory -Force $maps | Out-Null
Get-ChildItem (Join-Path $root "maps-repo\*.xml"), (Join-Path $root "maps-repo\audio-cards.json") | Copy-Item -Destination $maps

$plugins = Join-Path $stage "extension\plugin"
$skipDirs = @("target", "src", "tests", "__pycache__", ".git")
$skipFiles = @("Cargo.toml", "Cargo.lock", "update.stamp", ".gitignore")
# The yt-dlp program travels with the installer: the plugin copies it to the user's data folder the first time it
# runs (a copy under Program Files is read-only and could not update itself).
if ($WithoutYtdlp) { $skipDirs += "bin" }
$count = 0
Get-ChildItem (Join-Path $root "plugins-repo") -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "plugin.json") } |
    ForEach-Object {
        Copy-Folder $_.FullName (Join-Path $plugins $_.Name) $skipDirs $skipFiles
        $count++
    }

# The yt-dlp program has to be there: the plugin would download it on the first use otherwise.
if (-not $WithoutYtdlp) {
    $ytdlp = Join-Path $plugins "yt-dlp\bin\yt-dlp.exe"
    if (-not (Test-Path $ytdlp)) { throw "plugins-repo\yt-dlp\bin\yt-dlp.exe is missing: download it from https://github.com/yt-dlp/yt-dlp/releases" }
}

# The Deezer plugin runs the native engine; the Python version it replaces is not needed in the payload, and the copy
# of the engine without its extension cannot run on Windows.
$deezerStaged = Join-Path $plugins "deezer"
if (Test-Path (Join-Path $deezerStaged "main.py")) { Remove-Item (Join-Path $deezerStaged "main.py") }
$engineNoExt = Join-Path $deezerStaged "bin\deezer-engine"
if (Test-Path $engineNoExt) { Remove-Item $engineNoExt -Force -ErrorAction SilentlyContinue }
# The engine that was just built goes into the payload, whatever the copy in the repository holds.
if (Test-Path $built) {
    New-Item -ItemType Directory -Force (Join-Path $deezerStaged "bin") | Out-Null
    Copy-Item $built (Join-Path $deezerStaged "bin\deezer-engine.exe") -Force
}

$size = (Get-ChildItem $stage -Recurse -File | Measure-Object -Property Length -Sum).Sum
Write-Host ("payload: {0} plugins, {1:N1} MB in {2}" -f $count, ($size / 1MB), $stage)

# --- the installer --------------------------------------------------------------------------------------------------

$name = "MyOpenFadeR-$Version-windows-x86_64-setup"
if ($PerUser) { $name += "-peruser" }
$setup = Join-Path $OutDir "$name.exe"

Push-Location $PSScriptRoot
try {
    $defines = @("/V3", "/DPAYLOAD=$stage", "/DVERSION=$Version", "/DOUTFILE=$setup")
    if ($PerUser) { $defines += "/DPERUSER" }
    & $Makensis @defines "openfader.nsi"
    if ($LASTEXITCODE -ne 0) { throw "makensis failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

$mb = [math]::Round((Get-Item $setup).Length / 1MB, 1)
Write-Host "installer: $setup ($mb MB)"
