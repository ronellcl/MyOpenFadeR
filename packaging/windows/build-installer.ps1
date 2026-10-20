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

.PARAMETER WithYtdlp
Ships plugins-repo\yt-dlp\bin\yt-dlp.exe too. Off by default: the plugin downloads it into the user's data folder on
first use, where it can update itself (a copy under Program Files cannot).

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
    [switch]$WithYtdlp,
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

# --- ffmpeg and ffprobe ---------------------------------------------------------------------------------------------

$tools = Join-Path $stage "tools"
New-Item -ItemType Directory -Force $tools | Out-Null
foreach ($tool in "ffmpeg.exe", "ffprobe.exe") {
    Copy-Item (Join-Path $FfmpegBin $tool) $tools
}
$ffmpegReadme = Join-Path (Split-Path $FfmpegBin -Parent) "README.txt"
if (Test-Path $ffmpegReadme) { Copy-Item $ffmpegReadme (Join-Path $tools "ffmpeg-README.txt") }

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
if (-not $WithYtdlp) { $skipDirs += "bin" }
$count = 0
Get-ChildItem (Join-Path $root "plugins-repo") -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "plugin.json") } |
    ForEach-Object {
        Copy-Folder $_.FullName (Join-Path $plugins $_.Name) $skipDirs $skipFiles
        $count++
    }

$size = (Get-ChildItem $stage -Recurse -File | Measure-Object -Property Length -Sum).Sum
Write-Host ("payload: {0} plugins, {1:N1} MB in {2}" -f $count, ($size / 1MB), $stage)

# --- the installer --------------------------------------------------------------------------------------------------

$name = "OpenFadeR-$Version-windows-x86_64-setup"
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
