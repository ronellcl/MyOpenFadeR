<p align="center">
  <img src="assets/openfader.png" alt="MyOpenFadeR logo" width="160">
</p>

<h1 align="center">MyOpenFadeR</h1>

<p align="center">
  <img src="docs/media/hero.png" alt="MyOpenFadeR with two decks playing, synced, with waveforms, mixer and track browser" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: AGPL-3.0" src="https://img.shields.io/github/license/ronellcl/MyOpenFadeR"></a>
  <a href="https://github.com/ronellcl/MyOpenFadeR/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/ronellcl/MyOpenFadeR?style=flat"></a>
</p>

<p align="center">
  <a href="README.md"><b>English</b></a> · <a href="README.es.md"><b>Español</b></a>
</p>

MyOpenFadeR is a fork of [OpenFadeR](https://github.com/OpenRsG/OpenFadeR): an open-source DJ application for two or four decks, written in Rust with [Slint](https://slint.dev). It analyses your tracks, separates stems, mixes with effects and a sampler, and reads MIDI controllers from a growing set of mapping files. Themes, plugins and controller mappings install from GitHub repositories with one click.

<p align="center">
  <img src="docs/media/demo.webp" alt="Loading two tracks, syncing them, moving the crossfader and switching themes" width="100%">
</p>

<p align="center"><a href="docs/media/demo.mp4">Watch the demo as MP4</a></p>

Every screenshot on this page comes from the running app, with a library of synthetic tracks generated for these pictures. There is no copyrighted music in them.

## What's new in 0.4.2

0.4.2 makes timing and SYNC trustworthy under fast hands and adds the last safety and workflow details:

- **Loops and cues keep the beat.** Loop ends are stored to 1/256 ms, LOOP OUT goes on at the phase it would have had, quantized jumps catch up through loop wraps, and hot cues land on the beat the audio callback actually plays, including under latency, key lock and tempo changes.
- **SYNC knows when to let go.** It stops correcting while either deck brakes, spins up or the master scratches or reverses, leaves a master loop alone when it cannot preserve the beat phase, and gives the tempo fader back when the master cannot be followed.
- **Beat FX follow their source.** Each unit chooses CH, MST or TAP. Stem buttons keep the EQ processing and no longer switch the strip to STEM ISO.
- **Instant doubles and edits stay in phase.** Instant doubles, slicers, beat jumps and loop moves keep the beat instead of jumping up to a buffer late.
- **MIDI soft takeover.** An absolute knob or fader waits at the on-screen value after a layer, STEM or mode change, with a hint telling you which way to turn it; PREFERENCES > CONTROLLER > OTHERS turns it off.
- **Reload Tag (discard edits).** One undoable step reloads the file's text, BPM, key and analysis fields over the Collection row; fields the file lacks stay.
- **Store safety.** A `store.db` from another build or program is refused instead of migrated, and missing local tables fail closed instead of being recreated.

### 0.4.0

0.4.0 gathered what this branch added since 0.3.1:

- **SYNC locks at once.** The tempo and phase locks are taken together and the correction stays within a millisecond, so syncing two decks no longer pulls the mix.
- **Each track lives once, in 16 bits.** Decks, the browser preview and batch analysis decode straight to 16-bit samples (10.6 MB per minute of stereo at 44.1 kHz) instead of keeping an f32 copy of the whole track, and the fingerprint is computed while decoding.
- **Analysis reads through `audio::Frames`.** Grid, waveform, key and voice analysis accept the deck's 16-bit track without copying it; a track with no cache is analysed on the spot and stored as before.
- **The memory goes back to the system.** On glibc the allocator returns the buffers of an unloaded track instead of keeping them in the heap (274 MB to 203 MB with two six-minute tracks in the upstream measurement).
- **A Raspberry Pi build.** `Dockerfile.pi` cross-compiles a 64-bit archive for the Pi 3, 4 and CM4 (Debian 13), with STEMS on the CPU. See [docs/raspberry-pi.md](docs/raspberry-pi.md).
- **MIDI clock out.** PREFERENCES > CONTROLLER > OTHERS > MIDI CLOCK OUT sends a 24 PPQN clock at the lead deck's tempo and beat, with START on its next bar line and STOP when nothing plays.
- **Memory cues.** Memory points are drawn as wide bars with an M flag, next to the hot cues.
- **Edit track details.** A dialog edits title, artist, album, genre, comment, rating and colour without leaving the browser.
- **Stems are reused.** A track that was already separated keeps its stems when it is loaded again, and a new attach path fixes the first load.
- **The browser filters by origin.** The origin filter, the pinned folders and the duplicate notice are wired to the foldable tree; the tree labels speak one language (English, or Spanish in the preferences), and the PREVIEW column shows a still frame instead of the waveform.

<details>
<summary>Earlier releases (0.3.1 and before)</summary>

This release carries the changes of this branch on top of the upstream 0.2.1: the
interface in Spanish, a Windows installer that needs nothing else on the machine,
no console windows, a video engine that keeps up with the music, YouTube with
picture, the native Deezer engine, cloud tracks in the Collection, the VDJ stage
and a window that opens full screen.

- **The interface in Spanish.** Every menu, preference page and plugin label is translated, and PREFERENCES > VIEW > LANGUAGE chooses System, English or Spanish. The preferences the upstream added last (the bold beat grid, the large hot cue markers) are translated too.
- **A Windows installer that carries everything.** `MyOpenFadeR-<version>-windows-x86_64-setup.exe` installs the program for all users (it asks for administrator) with the plugins, the themes, the controller mappings, a light ffmpeg, a Python for the Python plugins and the yt-dlp program: a fresh machine needs nothing else. See [packaging/windows](packaging/windows).
- **No console windows.** The plugins run the interpreter that travels with the program, not the `py` launcher, and every helper (ffmpeg, ffprobe, the file pickers) starts without a console: no Windows Terminal window pops up over the app any more.
- **Video that keeps up.** A seek goes black and comes back with the right frame instead of freezing on the last one, and the ffmpeg that starts for it is the light build (80 ms against two seconds): a jump takes half a second instead of three.
- **YouTube with picture.** The yt-dlp plugin downloads H.264 + AAC (the formats every player shows), the picture lands on the deck's own screen and on the master screen, and the plugin keeps itself up to date from the copy it holds in your user folder.
- **Deezer, native.** The deezer plugin is a Rust program: it searches the catalogue and downloads the tracks (FLAC or MP3 320) with your own ARL, with no Node, no deemix and no Python in the middle. It travels with the installer.
- **Cloud tracks in the Collection.** A track from a plugin source (YouTube, Deezer) can live in a playlist or a palette like any other: the audio downloads when a deck loads it, and the browser marks with a ✓ the ones already downloaded.
- **The VDJ stage.** A theme can ask for the Virtual DJ layout: the FX panel and the master screen share a strip under the decks, every deck has a screen of its own, and the channel faders keep their length.
- **The window opens full screen.** PREFERENCES > VIEW > "Full screen at start-up" (on by default). The View menu's "Pantalla completa" turns it on and off, and the choice is remembered.
- **A browser that shows what is there.** The tree lists every mounted volume (a USB stick without a DJ library too, and a USB hard disk: Windows calls it a fixed disk) with its top-level folders, each open as a source of its own, and a "Downloaded" section with what the cloud plugins downloaded: real files, loadable, importable and searchable.
- **The local disk, too.** The volume Windows runs from (C:) is listed with the others, read-only, so its folders can be browsed like the ones of a stick.
- **The folders you pin.** The FAV button pins the folder the tree has chosen; the pinned folders wait for you in the "Favorites" section, and a pin survives the stick changing letter.
- **One search for everything local.** The search box looks in the Collection, in the index of the pinned folders and in the sources that are read, and each result says where it comes from. The online searches of YouTube and Deezer stay in their own box: the filter never asks a plugin.
- **Audio or video, said out loud.** A chip in the filter bar keeps everything, only the audio or only the video, and the track table has a FORMATO column with a note for the music and a film for the video.
- **Bitrate.** The track table gains a BITRATE column to the right of FORMATO, showing "320 kbps", "256 kbps", … (empty when unknown).
- **Foldable sections.** A click on History, Tags, Devices, Downloaded, Música Local or Música en Línea folds/unfolds their children (the icon toggles between ▸ and ▾); the keyboard only moves the selection.
- **History right-click.** Right-click a session row to rename it (keeping its date as the key) or delete it.
- **Related tracks auto-anchor.** If no track is chosen when Related turns on, it anchors on the first visible track instead of showing nothing.

</details>

## Download

Get the archive for your system from the [releases page](https://github.com/ronellcl/MyOpenFadeR/releases): Linux (x86_64), Windows (x86_64) or macOS (Apple silicon). Each holds the app with its themes, controller mappings, plugins and its own static `ffmpeg`/`ffprobe` (in `tools/`, first in its `PATH`): video, mp3 export and the yt-dlp video merge need nothing installed. Unpack it and run `openfader` (macOS: right-click `MyOpenFadeR.app` > Open the first time, the app is not signed). This is a pre-release: please report what does not work.

On Windows there is also an installer, `MyOpenFadeR-<version>-windows-x86_64-setup.exe`: it installs the program for all users (it asks for administrator) with the plugins, themes, controller mappings, ffmpeg and a Python for the Python plugins, adds Start menu and desktop shortcuts and appears in Add/Remove Programs. Uninstalling leaves your settings, library and downloaded add-ons in your user folder unless you say otherwise. Neither it nor the app is signed, so Windows SmartScreen asks the first time. See [packaging/windows](packaging/windows) to build it.

## Build from source

You need `git` and a [Rust toolchain](https://rustup.rs).

```sh
git clone https://github.com/ronellcl/MyOpenFadeR.git
cd MyOpenFadeR
./start.sh ~/Music
```

The first run clones the plugins, themes and mappings next to the code, builds everything and connects the folder you gave to the Collection. You should see your tracks in the browser, with BPM, key and waveform previews filling in as the analysis finishes. Double-click a track to load it on deck 1, press `Space` to play.

`start.sh` without a path opens the Collection as you left it; every connected library and folder syncs at start and whenever it changes, and `$XDG_MUSIC_DIR` (else `~/Music/OpenFadeR`) is always connected. To build only the core: `cargo build --release`.

The release archives carry their own `ffmpeg`/`ffprobe`; a source build can fetch the same static pair into `tools/` next to the binary (or use an `ffmpeg` from `PATH`):

```sh
python3 .github/fetch_tools.py target/release/tools
```

On Windows a seek starts `ffmpeg` again: a light build starts in 80 ms, while the full build of [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) (400 MB) takes two seconds and every jump waits for it. The static pair the release ships is the light one:

```powershell
Expand-Archive ffmpeg-release-essentials.zip -DestinationPath C:\Tools\ffmpeg
New-Item -ItemType Directory -Force target\release\tools
Copy-Item C:\Tools\ffmpeg\ffmpeg-*-essentials\bin\ffmpeg.exe, C:\Tools\ffmpeg\ffmpeg-*-essentials\bin\ffprobe.exe target\release\tools
```

To install it for your user on Linux instead (the `openfader` command and an application menu entry, with the plugins, themes and mappings bundled): `./install.sh`. It also sets the default plugin, theme and mapping repositories of PREFERENCES (GET PLUGINS / GET THEMES / GET MAPPINGS), which the app uses anyway until you edit the list. Run it again to update; `./install.sh --uninstall` removes it.

### Raspberry Pi

For a Raspberry Pi 3, 4 or CM4 (64-bit Raspberry Pi OS, Debian 13), `docker buildx build -f Dockerfile.pi -o dist/pi .` cross-compiles an optimised archive on any x86_64 machine. See [docs/raspberry-pi.md](docs/raspberry-pi.md) for memory use (about 200 MB with two tracks playing, software renderer) and settings.
## A tour

### Analysis that is ready when you load

BPM, beat grid, downbeat, key, a three-band waveform, phrases (intro, up, chorus, down) and automatic cues are computed in the background and cached. The pictures above show them: coloured phrase bars over the enlarged waveforms, hot cues A to H under each deck, and a key colour in the track list. Decks sync tempo and beats, key sync follows the key, and key lock keeps the pitch while you push the tempo.

For clearer waveforms, open PREFERENCES > VIEW > WAVEFORM. Set BEAT GRID to BOLD for outlined lines drawn over the audio, with wider downbeats that do not rely on colour alone. Set HOT CUE MARKERS to LARGE for double-size flags and letters on the decks and browser previews (when preview cue markers are enabled). Both settings apply immediately and are saved; NORMAL keeps the original sizes.

### Two decks or four

<p align="center"><img src="docs/media/four-decks.png" alt="Four decks playing in sync with four stacked waveform lanes" width="100%"></p>

Press `D` to switch layouts. All four decks get their own pads, loops, slip, sync and stem buttons. The deck layout (horizontal or vertical) is a VIEW setting, and a theme can ask for it.

### Effects

<p align="center"><img src="docs/media/fx.png" alt="Beat FX units with echo, reverb and flanger above the decks, and the color FX row" width="100%"></p>

Beat FX has several units with three slots each, tempo taken from the deck or tapped. COLOR FX (filter, space, dub echo, sweep, noise, crush, cross-pass) sits on every channel and MERGE FX works on the whole mix. More effects arrive through effect plugins.

### Sampler

<p align="center"><img src="docs/media/sampler.png" alt="Sampler strip with sixteen slots, banks and a sequencer" width="100%"></p>

Sixteen slots in two banks, loaded from a deck or from a file. Slots play as a shot or a gate, can follow the tempo, and a sequencer records and replays a pattern.

### Stems

<p align="center"><img src="docs/media/stems.png" alt="Stem buttons on two decks with vocal and drums muted on deck A" width="100%"></p>

Enable STEMS in PREFERENCES > EXTENSIONS and each loaded track is separated into vocal, instrument, bass and drums (a 3-stem mode exists too) by an ONNX model that runs on the CPU or the GPU. Switching STEMS on downloads that model once (316 MB), with a progress card at the bottom right. A deck's stem button is filled while its stem plays at full level; a click turns that stem off, another brings it back. The pads and a controller can mute or solo, and the mixer's knobs can act on stems. The waveform follows what you hear.

### BEAT LAB

<p align="center"><img src="docs/media/beatlab.png" alt="BEAT LAB strip with drum loops captured from two decks" width="100%"></p>

BEAT LAB captures a few beats of a deck's drum stem into a slot. Fill the slots of a deck, swap between them, and drop drums from one track under the melody of another. It needs STEMS and is switched on in the same preferences page.

### A browser that suggests

<p align="center"><img src="docs/media/suggest.png" alt="Track browser showing suggestions for the loaded track, ranked by match" width="100%"></p>

The browser reads folders (subfolders become playlists) or library plugins, and has search, sortable columns, a track filter (BPM range, key match, rating), related tracks, a Tag List, smart playlists, history, a duplicate finder and a missing-file manager. The SUGGEST tab ranks the rest of your collection against the loaded track and shows a score for each, with filters for BPM and key. Your own edits (cues, loops, grid, rating, comment, colour) live in `store.db`; a library source is never written to.

### Edit mode

<p align="center"><img src="docs/media/edit.png" alt="EDIT mode with a bar-snapped selection of 16 bars on a full-track waveform" width="100%"></p>

EDIT shows the whole track with its beat grid. Select with snap to beat or bar, then copy, cut, paste, insert, delete, mute, duplicate or add silence, undo and redo, and render the result to a WAV file or load it on a deck.

### Lighting mode (beta)

<p align="center"><img src="docs/media/lighting.png" alt="LIGHTING mode with fixtures lit by the phrase of the playing track" width="100%"></p>

LIGHTING patches fixtures (RGB pars, RGBW pars, moving heads, strobes) and drives them from the phrase that is playing, so a chorus looks different from a breakdown. Pick a palette and a scene, or let AUTO choose. DMX goes out over Art-Net or sACN, or nowhere: the stage view works without any hardware. It is disabled by default: turn it on in PREFERENCES > EXTENSIONS to add LIGHTING to the mode menu.

### Export and video

EXPORT mode shows cues and memory points for the two decks next to a DEVICES panel. USB sticks and SD cards are written by device plugins, several formats side by side on one stick sharing one copy of the audio, so the same stick plays on different players and apps. A track that is both in the collection and on a stick keeps the same cues, grid and phrases on both copies: plugging the stick in fills the copy that has none, and loading a track makes the copy you loaded win.

<p align="center"><img src="docs/media/export.png" alt="EXPORT mode with cue editing for two decks and a devices panel" width="100%"></p>

VIDEO (beta) plays video tracks on an output window and crossfades between the two decks' pictures. It is disabled by default: like LIGHTING, it shows in the mode menu once turned on in PREFERENCES > EXTENSIONS.

### Your library is yours

The Collection is OpenFadeR's own database. Other apps' libraries connect to it through library plugins (PREFERENCES > LIBRARIES) and are synced in; a format that holds less than the Collection (fewer hot cues, no loops, a smaller colour palette) is written from a fitted copy, and the status line says what did not fit. File > Export Collection also writes an open JSON file, a CSV table and an M3U8 playlist, described with the database tables in [docs/library-format.md](docs/library-format.md).

## Themes

<p align="center"><img src="docs/media/themes-grid.jpg" alt="The same view in Catppuccin Mocha, Latte, Rek, So Pro, Dracula, Nord, Tokyo Night and Gruvbox Dark" width="100%"></p>

The app ships with Catppuccin Mocha (the default) and Latte. The [MyOpenFadeR-Themes](https://github.com/ronellcl/MyOpenFadeR-Themes) repository adds eleven more, so there are 13 in total. A theme can change colours, fonts, sizes and even the layout, and it applies at once. PREFERENCES > THEMES has an editor for every token with live preview, SAVE AS, and import.

<p align="center"><img src="docs/media/themes-gallery.png" alt="PREFERENCES, THEMES page with the gallery and GET THEMES" width="100%"></p>

## Plugins, themes and mappings

Three companion repositories feed the app, and PREFERENCES lists them from the first run:

| Repository | Holds | Where in the app |
| --- | --- | --- |
| [MyOpenFadeR-Plugins](https://github.com/ronellcl/MyOpenFadeR-Plugins) | library, device, effect, action and source plugins (17 at the time of writing) | PREFERENCES > EXTENSIONS > GET PLUGINS |
| [MyOpenFadeR-Themes](https://github.com/ronellcl/MyOpenFadeR-Themes) | themes | PREFERENCES > THEMES > GET THEMES |
| [MyOpenFadeR-Map](https://github.com/ronellcl/MyOpenFadeR-Map) | MIDI controller mappings (more than 170 files) | PREFERENCES > CONTROLLER > GET MAPPINGS |

INSTALL installs one. Add any repository by its GitHub path (`user/name`) and the app finds the plugins, themes or mappings in it too, so sharing a theme is publishing a GitHub repository.

Plugins start switched off. At the first start a welcome window offers the suggested ones (the plugins that read and write other DJ software's libraries, devices and effects) and STEMS; ACCEPT switches them on, SKIP leaves them off. Help > Welcome opens it again.

At each start the app asks GitHub for the newest release; when it is newer than the running version a window offers to open its download page. Offline, nothing is shown.

Plug in a controller that has no mapping yet and the app offers the matching file from the mapping repository: one click on INSTALL and the controller works. Mapping files cover the default layer of each controller; MIDI learn and LED feedback are built in, and hot-plug detection picks controllers up while the app runs. The format is described in [docs/mappings.md](docs/mappings.md).

For external gear, PREFERENCES > CONTROLLER > OTHERS > MIDI CLOCK OUT takes part of an output port's name and sends a 24 PPQN MIDI clock at the lead deck's tempo and beat: START on that deck's next bar line and STOP when nothing plays any more, so a drum machine or a sequencer follows the mix (and picks the clock up again after an unplug and replug).

## Keyboard shortcuts

One table drives dispatch and the `?` help overlay (Shift = deck B, Ctrl = deck C, Ctrl+Shift = deck D):

| Key | Action | Key | Action |
| --- | --- | --- | --- |
| Space | play/pause | K | key sync |
| Z / X | CUE A / CUE B | `-` / `=` / 0 | key down / up / reset |
| 1–8 | pads (per pad mode) | M | next pad mode |
| L | 4-beat loop | V / E / B | slip / reverse / brake |
| I / O / R | loop in / out / reloop | T | add to Tag List |
| `[` / `]` | beat jump −4 / +4 | ← / → | FX beat fraction |
| Enter | LOAD | F | FX on/off |
| S | SYNC | Ctrl+F | focus search |
| Q | MASTER | W / Shift+W | VIEW / Area |
| Up / Down | browser selection | ? | help overlay |
| G / Y / N | grid row / tap / beat 1 here | `,` / `.` | grid shift ∓1 ms |
| P | sampler strip | A / Shift+A | AUTOMIX on/off / skip |
| D | 2 / 4-deck layout | Ctrl+Z / Ctrl+Shift+Z (Ctrl+Y) | undo / redo a library edit |

The table is editable in PREFERENCES > KEYBOARD.

## Where things are found

Each kind is looked up in an environment-variable folder list (`:`-separated), a folder in the config directory, and the bundled folder next to the executable (or in a folder above it).

| Kind | Environment variable | Config folder | Bundled folder |
| --- | --- | --- | --- |
| Plugins | `OPENFADER_PLUGINS` | `~/.config/openfader/extension/plugin/` | `extension/plugin/` |
| Themes | `OPENFADER_SKINS` | `~/.config/openfader/extension/skin/` | `extension/skin/` |
| Controller mappings | `OPENFADER_MAPPINGS` | `~/.config/openfader/mappings/` | `mappings/` |

`start.sh` clones the three repositories into those bundled folders (git ignores them here) and the native plugins build with `extension/plugin/build.sh` against this checkout's `sdk/`. Settings live in `$XDG_CONFIG_HOME/openfader/` (default `~/.config/openfader/`), the Collection (`store.db`) and our analysis in `$XDG_DATA_HOME/openfader/` (default `~/.local/share/openfader/`).

### Linux, Windows and macOS

The docs write the Linux folders; the others follow their system's convention (`$XDG_*_HOME` wins on every OS when set):

| | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Settings, your plugins, themes, mappings (`~/.config/openfader/` in these docs) | `~/.config/openfader/` | `~/Library/Application Support/openfader/` | `%APPDATA%\openfader\` |
| Store, analysis, downloaded plugin files | `~/.local/share/openfader/` | `~/Library/Application Support/openfader/` | `%LOCALAPPDATA%\openfader\` |
| Caches (stems, models) | `~/.cache/openfader/` | `~/Library/Caches/openfader/` | `%LOCALAPPDATA%\openfader\cache\` |

The release archives hold the bundled folders next to the executable (`MyOpenFadeR.app/Contents/Resources/` on macOS). Removable devices are found as USB sticks mounted below `/run/media` or `/media` (Linux), removable drive letters (Windows) and volumes below `/Volumes` (macOS); ejecting uses udisks, the volume's own eject request, or `diskutil`. Linux alone has the per-card volume sliders on the audio page (ALSA mixer, the sound server) and mounts a plugged stick itself (udisks): on Windows and macOS the volume is the system's and the OS mounts the stick. File choosers are zenity or kdialog on Linux, PowerShell's dialogs on Windows and `osascript` on macOS.

Audio, MIDI and folder behaviour can be set with `OPENFADER_OUTPUT_DEVICE`, `OPENFADER_HP_CHANNELS`, `OPENFADER_CUE_DEVICE`, `OPENFADER_MIDI_PORT`, `OPENFADER_MIDI_LOG`, `OPENFADER_JOG_TICKS`, `OPENFADER_TEMPO_INVERT`, `OPENFADER_REC_DIR`, `OPENFADER_EXPORT_DIR`, `OPENFADER_PATH_MAP` (audio path remap, `from=>to;…`) and `OPENFADER_DEVICE_ROOTS` (extra folders treated as devices).

## Documentation

- [docs/plugins.md](docs/plugins.md): plugin format and the library / device ABI, with the SDK in [`sdk/`](sdk/)
- [docs/skins.md](docs/skins.md): theme files, every token, layout hints
- [docs/mappings.md](docs/mappings.md): controller mapping files
- [docs/library-format.md](docs/library-format.md): the open JSON / CSV export and the library database tables
- [hallazgos_solucionados.md](hallazgos_solucionados.md): what was found and what was repaired, with the commit of each fix (in Spanish)

The code follows [docs/code-structure.md](docs/code-structure.md): one folder per section (`library/`, `analysis/`, `audio/`, `plugin/`, `app/`, …), a base `mod.rs` and one child per variant. Only `app/` and the `ui/` crate use Slint.

## Contributing

Bug reports, mapping files, themes and plugins are all welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first; the [code of conduct](CODE_OF_CONDUCT.md) applies, and security reports go through [SECURITY.md](SECURITY.md). `cargo test --release` runs the tests, and the companion repositories' tests run too when they are checked out.

## License

AGPL-3.0, see [LICENSE](LICENSE). Original [OpenFadeR](https://github.com/OpenRsG/OpenFadeR) by [OpenRsG](https://github.com/OpenRsG); MyOpenFadeR is maintained by [ronellcl](https://github.com/ronellcl).
