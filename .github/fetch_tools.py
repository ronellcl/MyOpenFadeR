#!/usr/bin/env python3
"""Fetches the static ffmpeg and ffprobe the archives ship in `tools/`.

The app starts them by name (`src/main.rs` puts `tools/` first in PATH), so a fresh
machine needs nothing else for video, mp3 export or merging the yt-dlp video. The
binaries come from the pinned ffmpeg-static build b6.1.1: Linux and Windows are
GPLv3 (they carry libx264), macOS is LGPL; the matching licence travels with them.

Usage: fetch_tools.py [folder]     (default: tools)
       OPENFADER_FFMPEG_TARGET=linux-arm64 fetch_tools.py tools   (a cross build)
"""
import os
import pathlib
import platform
import shutil
import sys
import urllib.request

VERSION = "b6.1.1"
BASE = f"https://github.com/eugeneware/ffmpeg-static/releases/download/{VERSION}"
TIMEOUT = 300


def target():
    """The asset suffix of this OS and architecture (the release matrix: Linux x86_64, Windows x86_64, macOS arm64)."""
    system, machine = platform.system(), platform.machine().lower()
    if system == "Linux":
        return "linux-arm64" if machine in ("aarch64", "arm64") else "linux-x64"
    if system == "Darwin":
        return "darwin-arm64" if machine in ("aarch64", "arm64") else "darwin-x64"
    if system == "Windows":
        return "win32-x64"
    raise SystemExit(f"no static ffmpeg for {system} {machine}")


def get(url, dest):
    print(f"fetching {url}")
    with urllib.request.urlopen(url, timeout=TIMEOUT) as src, open(dest, "wb") as out:
        shutil.copyfileobj(src, out)


def main():
    dest = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tools")
    dest.mkdir(parents=True, exist_ok=True)
    # A cross build (Dockerfile.pi) runs on x86_64 but packages ARM: its target is not the host's.
    key = os.environ.get("OPENFADER_FFMPEG_TARGET") or target()
    for name in ("ffmpeg", "ffprobe"):
        path = dest / (name + (".exe" if os.name == "nt" else ""))
        get(f"{BASE}/{name}-{key}", path)
        path.chmod(0o755)  # ignored on Windows; the execute bit matters on POSIX
    licence = dest / "LICENSE.ffmpeg.txt"
    try:
        get(f"{BASE}/{key}.LICENSE", licence)
    except OSError as e:
        print(f"warning: no ffmpeg licence downloaded: {e}", file=sys.stderr)


if __name__ == "__main__":
    main()
