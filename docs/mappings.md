# Controller mappings

A mapping is one XML file (`<OpenFadeRMapping>`) that tells the app which MIDI message of a controller is which control, and which
message lights which light. The files live in the [OpenFadeR-Map](https://github.com/OpenRsG/OpenFadeR-Map) repository. The app reads
them from, in order, `$OPENFADER_MAPPINGS` (`:`-separated folders), `~/.config/openfader/mappings/` (`$XDG_CONFIG_HOME/openfader/mappings`)
and the bundled `mappings/` next to the executable; a file in an earlier folder replaces one of the same name in a later one. When a
controller connects, the file whose `<port>` is the longest one found inside a MIDI input port name (case-insensitive) is loaded.

## Install from a repository

PREFERENCES > CONTROLLER > **GET MAPPINGS** lists the mapping files of your GitHub repositories, with the same repository list as GET
PLUGINS and GET THEMES (`~/.config/openfader/repositories.json`; `OpenRsG/OpenFadeR-Map` is in it on first run): type `owner/repo` and
**ADD**, **x** removes one, **REFRESH** downloads them again. A row shows the controller, its port name and the repository, with
**INSTALL**, **UPDATE** (the file of that name that the app reads differs from the repository's) or **INSTALLED**; the search box filters
by controller, port or file name. Installing copies the file into `~/.config/openfader/mappings/` after the mapping parser accepted it,
and if a matching controller is connected the session is opened again with it, as after a plug.

A repository's mapping files are the `*.xml` files at its top or at most two folders deep whose root element is `<OpenFadeRMapping`;
files the parser refuses are skipped and counted as "unreadable" on the repository's line.

**No search needed**: when a MIDI input port is connected that no mapping file matches, the app looks in the downloaded repositories (the
first time it downloads them) for a file whose `<port>` fits the port's name by the same rule, and shows
`Mapping for <controller> available: INSTALL` at the top right of the window. One click installs it and loads it.

## The mapping file

One file per controller: a port-name match, a list of input controls (MIDI status + number → one of the [actions](#actions) with its
deck and constant) and a list of outputs (one of the [lights](#lights) → MIDI status + number):

```xml
<OpenFadeRMapping schemaVersion="1">
  <info>
    <name>Example Controller</name>
    <port>Example</port>           <!-- picked when an input port name contains it (longest wins) -->
    <outInterval>2</outInterval>   <!-- ms between two messages to the controller (optional) -->
    <ringInterval>12</ringInterval> <!-- jog ring refresh in ms (optional) -->
    <stateRequest>F0 7E ... F7</stateRequest> <!-- sysex that asks the controller to report its state (optional) -->
  </info>
  <controls>
    <control status="0x90" midino="0x0B" action="play" deck="0"/>
    <control status="0xB4" midino="0x02" action="fx-level-1" deck="0" hires="true"/> <!-- 14-bit: LSB on midino + 0x20 -->
    <control status="0x97" midino="0x00" action="hw-pad" deck="0" arg="0"/>
  </controls>
  <outputs>
    <output status="0x90" midino="0x58" light="Sync" deck="0" blink="600"/>
    <output status="0xB4" midino="0x64" light="FxBeat(0)"/>
  </outputs>
</OpenFadeRMapping>
```

`schemaVersion` is `1` (a file without it is read as 1; another value is reported and read as 1). One binding per message: when a file
binds the same status and number twice, the first one wins and the lines after it are reported. `deck` is 0..3 (deck A..D; the FX unit
or MERGE side for those actions), `arg` the action's constant (default 0), `blink` the period in ms of the light's blinking state (a
light without it only ever shows on / off), and `light` one of the lights written as in Rust ([`#lights`](#lights)).

The session's log shows the diagnostics: unknown actions and lights, malformed attributes, elements or attributes the parser does not
know, and a `deck` outside 0..3. Controls and rows that survive stay in.

## Actions

Mapping files and MIDI LEARN share one action table, all of it listed below. "What it moves" is the MIDI message shape the action
wants: a mapping file binds them the way the shape says, and a relative encoder that is `two_comp` sends 127 for −1 (the
manufacturers' usual) while one without is centred on 64. Actions that need a per-row constant (pad slots, pad modes and the like)
bind only in mapping files (the row's `arg`); MIDI LEARN offers every other action — not just the ones the shipped files use, so an
action no shipped mapping binds still works from any button the user binds.

<!-- The ACTIONS table: generated from src/app/midi/actions (test `the_mappings_doc_lists_every_action_and_light` keeps it in sync). -->

| Action | What it moves |
|---|---|
| `play` | press |
| `cue` | press |
| `load` | press |
| `sync` | press |
| `key-lock` | press |
| `quantize` | press |
| `tempo-range` | press |
| `zoom-in` | press |
| `zoom-out` | press |
| `loop-in` | press |
| `loop-out` | press |
| `reloop` | press |
| `loop-halve` | press |
| `loop-double` | press |
| `loop-1` | press |
| `loop-2` | press |
| `loop-4` | press |
| `loop-8` | press |
| `loop-16` | press |
| `jump-back-4` | press |
| `jump-fwd-4` | press |
| `jump-back-1` | press |
| `jump-fwd-1` | press |
| `key-up` | press |
| `key-down` | press |
| `key-reset` | press |
| `key-sync` | press |
| `slip` | press |
| `reverse` | press |
| `brake` | press |
| `grid-edit` | press |
| `grid-beat1` | press |
| `grid-double` | press |
| `grid-halve` | press |
| `grid-tap` | press |
| `grid-reset` | press |
| `memory-cue` | press |
| `pad-mode` | press |
| `pad1` | press |
| `pad2` | press |
| `pad3` | press |
| `pad4` | press |
| `pad5` | press |
| `pad6` | press |
| `pad7` | press |
| `pad8` | press |
| `cue-monitor` | press |
| `split` | press |
| `rec` | press |
| `tag` | press |
| `stem-vocal` | press |
| `stem-inst` | press |
| `stem-bass` | press |
| `stem-drums` | press |
| `stem-mode` | press |
| `iso-toggle` | press |
| `iso-vocal` | knob (absolute CC) |
| `iso-inst` | knob (absolute CC) |
| `iso-drums` | knob (absolute CC) |
| `drum-gain` | knob (absolute CC) |
| `fx-toggle` | press |
| `fx-beat-up` | press |
| `fx-beat-down` | press |
| `merge-abort` | press |
| `sampler-open` | press |
| `help` | press |
| `tempo` | knob (absolute CC) |
| `tempo-hw` | knob (absolute CC) |
| `fader` | knob (absolute CC) |
| `crossfader` | knob (absolute CC) |
| `trim` | knob (absolute CC) |
| `eq-hi` | knob (absolute CC) |
| `eq-mid` | knob (absolute CC) |
| `eq-low` | knob (absolute CC) |
| `filter` | knob (absolute CC) |
| `master` | knob (absolute CC) |
| `hp-mix` | knob (absolute CC) |
| `hp-level` | knob (absolute CC) |
| `fx-level` | knob (absolute CC) |
| `fx-level-1` | knob (absolute CC) |
| `fx-level-2` | knob (absolute CC) |
| `fx-level-3` | knob (absolute CC) |
| `fx-beat` | knob (absolute CC) |
| `fx-beat-rel` | relative encoder |
| `merge-knob` | knob (absolute CC) |
| `merge-volume` | knob (absolute CC) |
| `jog` | jog (scratch while touched) |
| `jog-2c` | jog (scratch while touched) |
| `jog-bend` | jog (temporary pitch bend) |
| `jog-search` | jog (fast search) |
| `jog-cut` | jog (JOG CUTTER gate) |
| `jog-touch` | platter touch |
| `vinyl` | vinyl mode on / off |
| `jog-cutter` | JOG CUTTER mode on / off |
| `browse` | relative encoder |
| `browse-back` | press |
| `browse-view` | press |
| `browse-area` | press |
| `browse-push` | press |
| `zoom-rot` | relative encoder |
| `hw-deck-select` | mapping files only: deck select |
| `hw-pad` | mapping files only: straight onto the deck |
| `hw-pad-delete` | mapping files only: straight onto the deck |
| `hw-pad-stop` | mapping files only: straight onto the deck |
| `hw-pad-unload` | mapping files only: straight onto the deck |
| `hw-dual` | mapping files only: dual deck |
| `hw-pad-mode` | mapping files only: straight onto the deck |
| `hw-cue-call` | mapping files only: straight onto the deck |
| `hw-master` | mapping files only: straight onto the deck |
| `hw-track-start` | mapping files only: straight onto the deck |
| `hw-sampler` | mapping files only: press with the row's constant |
| `hw-fx-assign` | mapping files only: press with the row's constant |
| `hw-fx-target` | mapping files only: press with the row's constant |
| `hw-fx-on` | mapping files only: press with the row's constant |
| `hw-fx-hold` | mapping files only: press with the row's constant |
| `hw-fx-select` | mapping files only: press with the row's constant |
| `hw-fx-pick` | mapping files only: press with the row's constant |
| `hw-fx-pick-on` | mapping files only: press with the row's constant |
| `hw-fx-slot-beat` | mapping files only: press with the row's constant |
| `hw-fx-ch-hold` | mapping files only: press with the row's constant |
| `hw-fx-auto` | mapping files only: press with the row's constant |
| `hw-fx-tap` | mapping files only: press with the row's constant |
| `hw-fx-bpm` | mapping files only: press with the row's constant |
| `hw-fx-panel` | mapping files only: press with the row's constant |
| `hw-fx-release` | mapping files only: press with the row's constant |
| `hw-stem-double` | mapping files only: press with the row's constant |
| `hw-fx-part` | mapping files only: press with the row's constant |
| `hw-sampler-stop` | mapping files only: press with the row's constant |
| `hw-related` | mapping files only: press with the row's constant |
| `hw-memory-delete` | mapping files only: straight onto the deck |
| `hw-pad-page` | mapping files only: straight onto the deck |
| `hw-sort` | mapping files only: press with the row's constant |
| `hw-start-play` | mapping files only: straight onto the deck |
| `hw-start-cue` | mapping files only: straight onto the deck |
| `merge-on` | mapping files only: press with the row's constant |
| `merge-preset` | mapping files only: press with the row's constant |
| `hw-drum-swap` | mapping files only: press with the row's constant |
| `hw-drum-bank` | mapping files only: press with the row's constant |
| `hw-drum-mode` | mapping files only: press with the row's constant |
| `hw-drum-capture` | mapping files only: press with the row's constant |
| `hw-drum-range` | mapping files only: press with the row's constant |
| `hw-drum-release` | mapping files only: press with the row's constant |
| `hw-drum-roll` | mapping files only: press with the row's constant |
| `hw-drum-trans` | mapping files only: press with the row's constant |
| `hw-drum-fx` | mapping files only: press with the row's constant |

## Lights

A mapping file's `<output ... light="..."/>` names one of the lights below, written as in Rust (`Fx(0,2)` the unit 0's slot 2), and
states it with its MIDI message. `deck` picks which deck's state the light follows (the FX unit or MERGE side for those lights); a
light's deck shows nothing while the layout hides that deck. The `blink` attribute gives the period in ms of a blinking state.

| Light | Shows |
|---|---|
| `Play`, `Cue`, `Sync` | the play, cue and sync state |
| `Master` | the master button |
| `Loop` | a loop is active |
| `Vinyl` | vinyl mode (platter scratches while touched) |
| `Cutter` | JOG CUTTER mode |
| `HpCue` | cue monitor (headphones) |
| `Loaded` | the deck has a track |
| `Mode(n)` | pad mode button `n` (0 hot cue, 1 beat jump, 2 keyboard, 3 beat loop, 4 pad FX, 5 key shift) |
| `Pad(m,slot)` | the pad slot of pad mode `m` |
| `PadSlot(slot)` | a pad of whatever mode the deck shows (controllers whose pads send the same note in every mode) |
| `Sampler` | the sampler page |
| `Fx(unit,slot)` | a Beat FX slot is on |
| `FxEffect(unit,slot,fx,on)` | a Beat FX slot has effect `fx` selected (selector index), and with `true` also on |
| `FxTapped(unit,tap)` | the Beat FX unit's tempo comes from TAP (`true`) or AUTO (`false`) |
| `FxAssign(unit,ch)` | the Beat FX unit is assigned (0..3 CH1..CH4, 4 MASTER) |
| `FxChOn(unit,ch,slot)` | a Beat FX slot is on, with its unit on channel `ch`; the message goes to the channel the unit is assigned |
| `FxRelease(unit)` | the Beat FX unit's RELEASE FX is running |
| `FxBeat(unit)` | the Beat FX unit's beat display: the message's velocity is the beat's code, not on / off |
| `SamplerPad(slot)` | a sampler pad of the visible bank holds a sample |
| `TempoRange`, `TrackStart`, `Tap`, `Loop4` | the shift layer: tempo range not ±6 %, a track is loaded, GRID edit is on, the active loop is 4 beats |
| `Level` | the channel VU meter: the message's velocity is the level 0..127, not on / off |
| `Ring` | jog ring segment `1..=72` of the playhead's turn (0 = no track) |
| `Iso` | the PART ISOLATOR mode (STEMS) |
| `Stem(part)` | a STEM's ACTIVE PART light (part 0..3: VOCAL, INST, BASS, DRUMS) |
| `FxPart(part)` | a Beat FX unit's FX PART SELECT light of part 1..4 |
| `MergePreset(side,n)` | a MERGE FX side's preset `n` (0..4) is selected, or an effect is held on |
| `MergeOn(side)` | a MERGE FX side is active |
| `Drum(pad)` | a BEAT LAB DRUM SWAP pad (0..3 of the bank in view) playing or waiting |
| `DrumBank(bank)` | the BEAT LAB bank in view (0 A, 1 B) |
| `DrumMode(multi)` | the BEAT LAB mode (true: MULTI, false: SINGLE) |
| `DrumCapture` | DRUM CAPTURE is armed |

## audio-cards.json

Besides its mappings, a repository may hold a `audio-cards.json` described in `src/audio/bus.rs`:

```json
{ "headphones_3_4": ["ddj-1000", "xone"] }
```

The list holds lower-case name fragments of the audio devices whose outputs 3/4 carry the headphones (a DJ controller: master on its
outputs 1/2, headphones on 3/4 — without the entry the app would send the cue feed to SPLIT only). The first mapping folder that has
one wins. GET MAPPINGS installs it beside the mappings when you install from the repository, keeping an `audio-cards.json` you already
have (yours wins). PREFERENCES > AUDIO can always name the headphone pair by hand first.
