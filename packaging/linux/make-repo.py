#!/usr/bin/env python3
"""Build the GitHub Pages package repositories from one release's packages.

    make-repo.py --version 0.5.0 --out public --deb myopenfader_0.5.0_amd64.deb \
        --pkg myopenfader-0.5.0-1-x86_64.pkg.tar.zst

Creates:
    out/debian/pool/main/m/myopenfader/*.deb
    out/debian/dists/stable/main/binary-amd64/Packages(.gz)
    out/debian/dists/stable/Release
    out/arch/x86_64/*.pkg.tar.zst
    out/arch/x86_64/myopenfader.db.tar.gz (and the .db symlink)
    out/index.html

No dpkg or pacman tools are needed: the .deb control is read from the ar
archive and the pacman database is written by hand (the format is stable).
"""

import argparse
import datetime
import gzip
import hashlib
import io
import lzma
import os
import shutil
import subprocess
import tarfile
import time

PAGES = "https://ronellcl.github.io/MyOpenFadeR"


def ar_member(deb: str, prefix: str) -> bytes:
    members = subprocess.check_output(["ar", "t", deb], text=True).split()
    name = next(m for m in members if m.startswith(prefix))
    return subprocess.check_output(["ar", "p", deb, name]), name


def deb_control(deb: str) -> str:
    raw, name = ar_member(deb, "control.tar")
    if name.endswith(".gz"):
        raw = gzip.decompress(raw)
    elif name.endswith(".xz"):
        raw = lzma.decompress(raw)
    elif name.endswith(".zst"):
        raw = subprocess.run(
            ["zstd", "-d", "-c"], input=raw, capture_output=True, check=True
        ).stdout
    with tarfile.open(fileobj=io.BytesIO(raw)) as tf:
        for candidate in ("./control", "control"):
            try:
                f = tf.extractfile(candidate)
            except KeyError:
                continue
            if f:
                return f.read().decode()
    raise SystemExit(f"{deb}: no control member")


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def digest(path: str, algo) -> str:
    h = hashlib.new(algo)
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def pkginfo(pkg: str) -> dict:
    text = subprocess.run(
        ["tar", "--zstd", "-xOf", pkg, ".PKGINFO"],
        capture_output=True,
        check=True,
        text=True,
    ).stdout
    fields = {}
    for line in text.splitlines():
        if " = " not in line:
            continue
        key, value = line.split(" = ", 1)
        fields.setdefault(key.strip(), []).append(value.strip())
    return fields


def arch_desc(pkg: str, fields: dict) -> str:
    name = fields["pkgname"][0]
    version = fields["pkgver"][0]
    packager = fields.get("packager", ["MyOpenFadeR"])
    if packager == ["Unknown Packager"]:
        packager = ["Ronell <ronellcl@users.noreply.github.com>"]
    sections = [
        ("%FILENAME%", [os.path.basename(pkg)]),
        ("%NAME%", [name]),
        ("%BASE%", fields.get("pkgbase", [name])),
        ("%VERSION%", [version]),
        ("%DESC%", fields.get("pkgdesc", [""])),
        ("%CSIZE%", [str(os.path.getsize(pkg))]),
        ("%ISIZE%", fields.get("size", ["0"])),
        ("%MD5SUM%", [digest(pkg, "md5")]),
        ("%SHA256SUM%", [digest(pkg, "sha256")]),
        ("%URL%", fields.get("url", [""])),
        ("%LICENSE%", fields.get("license", [])),
        ("%ARCH%", fields.get("arch", ["x86_64"])),
        ("%BUILDDATE%", fields.get("builddate", [str(int(time.time()))])),
        ("%PACKAGER%", packager),
        ("%DEPENDS%", fields.get("depend", [])),
        ("%OPTDEPENDS%", fields.get("optdepend", [])),
        ("%PROVIDES%", fields.get("provides", [])),
        ("%CONFLICTS%", fields.get("conflict", [])),
        ("%REPLACES%", fields.get("replaces", [])),
    ]
    out = []
    for title, values in sections:
        if not values:
            continue
        out.append(title)
        out.extend(values)
        out.append("")
    return "\n".join(out)


def write_arch_db(out: str, pkg: str) -> None:
    fields = pkginfo(pkg)
    name = fields["pkgname"][0]
    version = fields["pkgver"][0]
    db_dir = os.path.join(out, "arch", "x86_64")
    os.makedirs(db_dir, exist_ok=True)
    data = io.BytesIO()
    with tarfile.open(fileobj=data, mode="w:gz") as tf:
        desc = arch_desc(pkg, fields).encode()
        info = tarfile.TarInfo(f"{name}-{version}/desc")
        info.size = len(desc)
        info.mtime = int(time.time())
        tf.addfile(info, io.BytesIO(desc))
    db = os.path.join(db_dir, "myopenfader.db.tar.gz")
    with open(db, "wb") as f:
        f.write(data.getvalue())
    link = os.path.join(db_dir, "myopenfader.db")
    if os.path.lexists(link):
        os.remove(link)
    os.symlink("myopenfader.db.tar.gz", link)


def write_apt(out: str, deb: str) -> None:
    pool = os.path.join(out, "debian", "pool", "main", "m", "myopenfader")
    os.makedirs(pool, exist_ok=True)
    target = os.path.join(pool, os.path.basename(deb))
    shutil.copy2(deb, target)
    control = deb_control(deb).rstrip("\n")
    stanza = (
        control
        + f"\nFilename: pool/main/m/myopenfader/{os.path.basename(deb)}"
        + f"\nSize: {os.path.getsize(target)}"
        + f"\nSHA256: {sha256(target)}\n"
    )
    index_dir = os.path.join(out, "debian", "dists", "stable", "main", "binary-amd64")
    os.makedirs(index_dir, exist_ok=True)
    packages = os.path.join(index_dir, "Packages")
    with open(packages, "w") as f:
        f.write(stanza + "\n")
    with open(packages, "rb") as src, gzip.open(packages + ".gz", "wb", compresslevel=9) as dst:
        shutil.copyfileobj(src, dst)
    now = datetime.datetime.now(datetime.timezone.utc)
    release = (
        "Origin: MyOpenFadeR\n"
        "Label: MyOpenFadeR\n"
        "Suite: stable\n"
        "Codename: stable\n"
        "Architectures: amd64\n"
        "Components: main\n"
        "Description: MyOpenFadeR apt repository\n"
        f"Date: {now.strftime('%a, %d %b %Y %H:%M:%S UTC')}\n"
        "SHA256:\n"
    )
    for name in ("Packages", "Packages.gz"):
        path = os.path.join(index_dir, name)
        release += f" {sha256(path)} {os.path.getsize(path)} main/binary-amd64/{name}\n"
    with open(os.path.join(out, "debian", "dists", "stable", "Release"), "w") as f:
        f.write(release)


def write_index(out: str, version: str) -> None:
    html = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>MyOpenFadeR packages</title>
<style>
body {{ font-family: system-ui, sans-serif; max-width: 760px; margin: 3rem auto; padding: 0 1rem; line-height: 1.5; }}
pre {{ background: #f4f4f4; padding: .8rem; overflow-x: auto; }}
code {{ background: #f4f4f4; padding: .1rem .3rem; }}
</style>
</head>
<body>
<h1>MyOpenFadeR packages</h1>
<p>APT and pacman repositories for MyOpenFadeR {version}. The packages install the app in
<code>/opt/myopenfader</code>, add <code>openfader</code> to the PATH and a desktop menu entry.</p>

<h2>Debian / Ubuntu</h2>
<pre>echo 'deb [trusted=yes] {PAGES}/debian stable main' | sudo tee /etc/apt/sources.list.d/myopenfader.list
sudo apt update
sudo apt install myopenfader</pre>

<h2>Arch Linux / CachyOS</h2>
<p>Add these lines to <code>/etc/pacman.conf</code>:</p>
<pre>[myopenfader]
SigLevel = Optional TrustAll
Server = {PAGES}/arch/x86_64</pre>
<p>Then:</p>
<pre>sudo pacman -Sy myopenfader</pre>

<h2>Portable archive</h2>
<p>The <code>.tar.gz</code> archive is published with each release at
<a href="https://github.com/ronellcl/MyOpenFadeR/releases">github.com/ronellcl/MyOpenFadeR/releases</a>.</p>
</body>
</html>
"""
    with open(os.path.join(out, "index.html"), "w") as f:
        f.write(html)


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--version", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--deb", required=True)
    p.add_argument("--pkg", required=True)
    a = p.parse_args()
    os.makedirs(a.out, exist_ok=True)
    write_apt(a.out, a.deb)
    arch_dir = os.path.join(a.out, "arch", "x86_64")
    os.makedirs(arch_dir, exist_ok=True)
    pkg_target = os.path.join(arch_dir, os.path.basename(a.pkg))
    shutil.copy2(a.pkg, pkg_target)
    write_arch_db(a.out, pkg_target)
    write_index(a.out, a.version)
    print(f"repository written to {a.out}")


if __name__ == "__main__":
    main()
