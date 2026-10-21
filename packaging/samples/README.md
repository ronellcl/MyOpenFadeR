# The factory sample pack (BEAT LAB)

BEAT LAB's "factory" drum presets are the **playlists of a sampler pack** (a `.spp` file): the `sample-pack` plugin
opens the pack and the first eight samples of each of its lists become a Drum Preset. The upstream never published
one, so the presets of a fresh installation are empty until a pack is set.

PREFERENCES > EXTENSIONS > **FACTORY SAMPLES PACK (sample pack)** takes the path of a `.spp` file. The pack is
unpacked under the cache folder (`%TEMP%\openfader\sample-pack\` on Windows) and the plugin
[`sample-pack`](../../plugins-repo/sample-pack) must be switched on.

## This pack

`make_pack.py` builds one from scratch: 56 drum loops (one bar of 120 BPM) in seven styles — Rock, House, Techno,
Trap, Funk, Breakbeat and Disco — synthesized with the standard library (no sample library, so there is no licensing
question at all). The sounds are a 909-style kit: a kick with a pitch sweep, a snare of tone plus noise, filtered
noise hats, a clap of four bursts, a tom and a ride.

```sh
python make_pack.py OpenFadeR-Factory-Loops.spp
```

The output is a zip with the audio (one folder per style) and the `sampler_pack.xml` the plugin reads: `<TRACKS>` of
`<TRACK id location mode bpm/>` and `<LISTS>` of `<LIST id name attr seq>` holding `<TRACK key=id/>`. The names of
the lists are the names of the presets.

The format is open, so a pack of recorded loops works the same way: a zip with the audio and that XML. The plugin only
reads packs; it never writes them.

## Using it

Point the setting at the file and pick a preset in BEAT LAB (the PRESET dropdown of each deck). DRUM CAPTURE, which
takes the drums of the track you are playing, works with or without a pack.
