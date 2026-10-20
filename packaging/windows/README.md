# Windows installer

`build-installer.ps1` stages everything the program finds next to `openfader.exe` and compiles `openfader.nsi` with
[NSIS](https://nsis.sourceforge.io) into `dist\windows\OpenFadeR-<version>-windows-x86_64-setup.exe`.

## What goes in

| In the payload | Where it comes from |
|---|---|
| `openfader.exe`, its DLLs, the Visual C++ runtime | `target\release` (`cargo build --release`) and `%WINDIR%\System32` |
| `tools\ffmpeg.exe`, `tools\ffprobe.exe` | `-FfmpegBin`: video, mp3 export and the yt-dlp plugin. **Use a light build**: a video seek starts ffmpeg again, and the full build of gyan.dev (400 MB) takes 1.2 s to start against the 40 ms of the essentials build (https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip). The script measures it and warns. |
| `python\` | `-PythonDir`: an unpacked `python-<version>-embed-amd64.zip`, the interpreter of the `.py` plugins |
| `extension\plugin\<plugin>\` | every `plugins-repo` folder with a `plugin.json`, without sources or build folders |
| `extension\plugin\yt-dlp\bin\yt-dlp.exe` | `plugins-repo\yt-dlp\bin`: the yt-dlp program, so the plugin works without downloading anything |
| `extension\plugin\deezer\bin\deezer-engine.exe` | built from `plugins-repo\deezer`: the Deezer engine, a native program |
| `extension\skin\*.json`, `mappings\*.xml` | `themes-repo` and `maps-repo` |
| `LICENSE`, `README.md` | the repository root |

That is the layout `src/paths.rs` searches next to the executable, plus `tools\` and `python\`, which go first in the
PATH of the program and of the processes it starts (`src/main.rs`). The installer therefore does not touch the system
PATH, and ffmpeg is found inside the yt-dlp plugin too.

## The add-ons that travel inside

**yt-dlp.** The program is in `extension\plugin\yt-dlp\bin`. The copy under `Program Files` is read-only, and
`yt-dlp -U` could not update it, so the plugin copies it the first time it runs into
`%LOCALAPPDATA%\openfader\plugins\yt-dlp\bin` (writable) and uses that copy from then on: the daily update works.
`-WithoutYtdlp` leaves it out (the plugin then downloads it on first use).

**Deezer.** The engine is a native program (2.4 MB) that speaks the plugin protocol: it searches the catalogue and
downloads the tracks (with the user's own ARL), decrypting the audio on the way to the file. It needs neither Node,
nor deemix, nor Python: the installer has no `deemix` dependency at all. The script builds it with cargo
(`plugins-repo\deezer`) and puts it in the plugin's `bin` folder, where the manifest looks for it.


## Building it

Once: a release build of the program, NSIS 3 (`makensis`, the zip from <https://nsis.sourceforge.io> is enough), ffmpeg
and an embeddable Python.

```powershell
cargo build --release
.\packaging\windows\build-installer.ps1 `
    -PythonDir C:\Tools\python-3.14.8-embed-amd64 `
    -Makensis  C:\Tools\nsis-3.13\makensis.exe
```

The installer installs for all users (it asks for administrator) in `%ProgramFiles%\OpenFadeR`, with Start menu and
desktop shortcuts and an entry in Add/Remove Programs. Uninstalling asks before deleting the settings, library and
downloaded add-ons in the user's folder.

## Trying it without administrator

`-PerUser` compiles the same wizard against `%LOCALAPPDATA%\Programs\OpenFadeR`, `HKCU` and no UAC, which is how the
script is checked on a machine that cannot elevate:

```powershell
.\packaging\windows\build-installer.ps1 -PerUser -SkipBuild -PythonDir ... -Makensis ...
Start-Process .\dist\windows\OpenFadeR-0.1.0-windows-x86_64-setup-peruser.exe -ArgumentList /S -Wait
# ... start the app from %LOCALAPPDATA%\Programs\OpenFadeR and use it ...
Start-Process "$env:LOCALAPPDATA\Programs\OpenFadeR\uninstall.exe" -ArgumentList /S -Wait
```

The release workflow (`.github/workflows/release.yml`) still publishes the archives, not this installer.
