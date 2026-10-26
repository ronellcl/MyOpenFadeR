#!/usr/bin/env python3
"""Fills a release folder with what the app finds next to its executable (src/paths.rs search_path): the plugins
(extension/plugin: each plugin folder as the app reads it, with the library built for this OS, no sources), the themes
(extension/skin) and the controller mappings (mappings). Usage: bundle.py <folder>  (run from the core checkout with the
companion repositories in place and the plugins built by extension/plugin/build.sh)."""
import pathlib
import shutil
import sys

dest = pathlib.Path(sys.argv[1])
root = pathlib.Path(".")
SKIP = {"src", "tests", "examples", "target", "dist", ".git", ".github", "Cargo.toml", "Cargo.lock"}

plugins = 0
for manifest in sorted((root / "extension/plugin").glob("*/plugin.json")):
    src = manifest.parent
    shutil.copytree(src, dest / "extension/plugin" / src.name, ignore=lambda _, names: [n for n in names if n in SKIP])
    plugins += 1

(dest / "extension/skin").mkdir(parents=True, exist_ok=True)
themes = sorted((root / "extension/skin").glob("*.json"))
for f in themes:
    shutil.copy(f, dest / "extension/skin" / f.name)

(dest / "mappings").mkdir(parents=True, exist_ok=True)
maps = [f for f in sorted((root / "mappings").iterdir()) if f.suffix in (".xml", ".json")]
for f in maps:
    shutil.copy(f, dest / "mappings" / f.name)

# `tools/` (static ffmpeg and ffprobe, fetched by .github/fetch_tools.py) goes beside the program or, in a macOS
# bundle, in Resources: src/main.rs looks in both. A source checkout that has not fetched them still packages; the
# release workflow does fetch them, so the archives carry their own decoder.
tools = 0
if (root / "tools").is_dir():
    shutil.copytree(root / "tools", dest / "tools")
    tools = len(list((dest / "tools").iterdir()))
else:
    print("warning: no tools/ (ffmpeg and ffprobe) to bundle", file=sys.stderr)

print(f"bundled {plugins} plugins, {len(themes)} themes, {len(maps)} mappings, {tools} tools into {dest}")
if not (plugins and themes and maps):
    sys.exit("a companion repository is missing or empty")
