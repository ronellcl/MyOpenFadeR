"""Builds a factory sample pack (.spp) for BEAT LAB: drum loops synthesized from scratch, so there is no licensing
question at all. The format is the one the sample-pack plugin reads: a zip with the audio and a `sampler_pack.xml`
with <TRACKS> (id, location, mode, bpm) and <LISTS> (the lists, whose tracks are the ids in order).

    python make_pack.py <salida.spp>

Each list is a style and its eight tracks are the eight variations of one bar of that style, so BEAT LAB's eight
slots become that style's variations. Two bars? One bar of 120 BPM (2 s) per loop, played in loop by the deck.
"""
import math
import os
import struct
import sys
import wave
import zipfile

RATE = 44100
BPM = 120.0
BEAT = 60.0 / BPM          # seconds of a beat
BAR = BEAT * 4             # a bar of 4/4
STEPS = 16                # a bar of sixteenths


def env(i, n, decay=6.0):
    """A fast exponential decay, 1 at the start and ~0 at the end."""
    t = i / RATE
    return math.exp(-decay * t)


def silence(n):
    return [0.0] * n


def kick(length=0.32):
    n = int(RATE * length)
    out = []
    phase = 0.0
    for i in range(n):
        t = i / RATE
        f = 45.0 + 95.0 * math.exp(-28.0 * t)      # the pitch sweep, 140 Hz down to 45 Hz
        phase += 2 * math.pi * f / RATE
        click = 0.5 * math.exp(-260.0 * t) * math.sin(2 * math.pi * 1800 * t)
        out.append((math.sin(phase) * math.exp(-9.0 * t) + click) * 0.95)
    return out


def snare(length=0.24):
    n = int(RATE * length)
    out = []
    phase = 0.0
    lp = 0.0
    for i in range(n):
        t = i / RATE
        phase += 2 * math.pi * 185.0 / RATE
        noise = (math.sin(i * 12.9898) * 43758.5453) % 1.0 * 2 - 1   # a cheap noise, no RNG needed
        lp = lp * 0.55 + noise * 0.45                                 # a one-pole low-pass for the body
        body = math.sin(phase) * 0.35
        crack = (noise - lp) * 0.85
        out.append((body + crack) * math.exp(-16.0 * t) * 0.9)
    return out


def hat(length=0.06, bright=0.85):
    n = int(RATE * length)
    out = []
    lp = 0.0
    for i in range(n):
        t = i / RATE
        noise = (math.sin(i * 78.233) * 12345.678) % 1.0 * 2 - 1
        lp = lp * 0.35 + noise * 0.65
        out.append(lp * math.exp(-38.0 * t) * bright)
    return out


def clap(length=0.22):
    n = int(RATE * length)
    out = [0.0] * n
    for offset in (0.0, 0.011, 0.021, 0.030):
        start = int(offset * RATE)
        for i in range(int(RATE * 0.12)):
            if start + i >= n:
                break
            t = i / RATE
            noise = (math.sin((start + i) * 45.164) * 23421.631) % 1.0 * 2 - 1
            out[start + i] += noise * math.exp(-34.0 * t) * 0.55
    return out


def tom(length=0.3):
    n = int(RATE * length)
    out = []
    phase = 0.0
    for i in range(n):
        t = i / RATE
        f = 210.0 - 90.0 * (t / length)
        phase += 2 * math.pi * f / RATE
        out.append(math.sin(phase) * math.exp(-11.0 * t) * 0.8)
    return out


def ride(length=0.5):
    n = int(RATE * length)
    out = []
    for i in range(n):
        t = i / RATE
        noise = (math.sin(i * 91.7) * 5671.23) % 1.0 * 2 - 1
        bell = math.sin(2 * math.pi * 2400 * t) * 0.25 + math.sin(2 * math.pi * 3300 * t) * 0.15
        out.append((noise * 0.35 + bell) * math.exp(-7.0 * t) * 0.55)
    return out


SOUNDS = {
    "kick": kick(),
    "snare": snare(),
    "clap": clap(),
    "hat": hat(),
    "ohat": hat(0.38, 0.7),
    "tom": tom(),
    "ride": ride(),
}

# The styles: which steps of the sixteen of a bar get which hit. "hat" is every other step unless said otherwise.
STYLES = {
    "Rock":      {"kick": (0, 8), "snare": (4, 12), "hat": tuple(range(0, 16, 2))},
    "House":     {"kick": (0, 4, 8, 12), "clap": (4, 12), "hat": tuple(range(16))},
    "Techno":    {"kick": (0, 4, 8, 12), "clap": (8, 14), "hat": tuple(range(16)), "ride": (0, 4, 8, 12)},
    "Trap":      {"kick": (0, 7, 10), "snare": (8,), "hat": tuple(range(0, 16, 2))},
    "Funk":      {"kick": (0, 6, 10, 11), "snare": (4, 12), "hat": tuple(range(16))},
    "Breakbeat": {"kick": (0, 6, 10), "snare": (8, 13), "hat": tuple(range(0, 16, 2)), "ride": (4, 12)},
    "Disco":     {"kick": (0, 4, 8, 12), "clap": (4, 12), "hat": tuple(range(0, 16, 2)), "tom": (14,)},
}

# The sound of each hit and how loud.
VOICES = {
    "kick": 1.0,
    "snare": 0.85,
    "clap": 0.8,
    "hat": 0.45,
    "ohat": 0.4,
    "tom": 0.7,
    "ride": 0.35,
}


def build_loop(style, variation):
    """One bar of a style: the pattern, plus the variation (the open hat moves, the last beat gets a fill)."""
    pattern = STYLES[style]
    length = int(RATE * BAR)
    out = [0.0] * length
    step_dur = BAR / STEPS
    for name, steps in pattern.items():
        for step in steps:
            sound = SOUNDS[name]
            gain = VOICES[name]
            start = int(step * step_dur * RATE)
            for i, s in enumerate(sound):
                if start + i >= length:
                    break
                out[start + i] += s * gain
    # The open hat on the "and" of the beat, in some variations.
    if variation % 2 == 1 and style in ("House", "Disco", "Techno"):
        for step in (2, 6, 10, 14):
            sound = SOUNDS["ohat"]
            start = int(step * step_dur * RATE)
            for i, s in enumerate(sound):
                if start + i >= length:
                    break
                out[start + i] += s * 0.35
    # A fill in the last beat of some variations.
    if variation % 4 == 3:
        for k in range(4):
            step = 12 + k
            sound = SOUNDS["snare" if k % 2 else "tom"]
            start = int(step * step_dur * RATE)
            for i, s in enumerate(sound):
                if start + i >= length:
                    break
                out[start + i] += s * 0.6
    # A soft limit, so the mix never clips.
    return [math.tanh(v * 1.2) * 0.88 for v in out]


def write_wav(path, samples):
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        frames = bytearray()
        for v in samples:
            v = max(-1.0, min(1.0, v))
            frames += struct.pack("<h", int(v * 32767))
        w.writeframes(bytes(frames))


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "factory-loops.spp"
    work = os.path.join(os.path.dirname(os.path.abspath(out)), "_pack_build")
    os.makedirs(work, exist_ok=True)
    tracks = []
    lists = []
    track_id = 0
    for style in STYLES:
        keys = []
        for v in range(8):
            track_id += 1
            name = f"{style}/loop-{v + 1}.wav"
            path = os.path.join(work, name.replace("/", os.sep))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            print(f"  {name}", flush=True)
            write_wav(path, build_loop(style, v))
            tracks.append((str(track_id), name, BPM))
            keys.append(str(track_id))
        lists.append((style, keys))

    xml = ['<?xml version="1.0" encoding="UTF-8"?>', '<PACK Version="1.0.0">', f'<TRACKS num="{len(tracks)}">']
    for tid, location, bpm in tracks:
        xml.append(f'<TRACK id="{tid}" location="{location}" mode="1" bpm="{bpm:.2f}" />')
    xml.append("</TRACKS>")
    xml.append("<LISTS>")
    for i, (name, keys) in enumerate(lists):
        xml.append(f'<LIST id="{i + 1}" name="{name}" attr="3" seq="{i}">')
        for k in keys:
            xml.append(f'<TRACK key="{k}" />')
        xml.append("</LIST>")
    xml.append("</LISTS>")
    xml.append("</PACK>")
    with open(os.path.join(work, "sampler_pack.xml"), "w", encoding="utf-8") as f:
        f.write("\n".join(xml))

    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.write(os.path.join(work, "sampler_pack.xml"), "sampler_pack.xml")
        for style in STYLES:
            for v in range(8):
                z.write(os.path.join(work, style, f"loop-{v + 1}.wav"), f"{style}/loop-{v + 1}.wav")
    print(f"listo: {out} ({os.path.getsize(out) / 1e6:.1f} MB, {len(tracks)} loops, {len(lists)} estilos)")


if __name__ == "__main__":
    main()
