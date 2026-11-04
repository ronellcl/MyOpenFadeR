//! A plain directory as a library ("Explorer"): every audio (or mp4 / mov / m4v video) file below the root is a
//! track, every subfolder holding audio is a playlist, nested like the disk. Tags come from
//! symphonia's metadata readers (observed: ID3v2.3/2.4, MP4 ilst, Vorbis comments, RIFF INFO, AIFF
//! chunks; ID3v1 is NOT read: the default probe only registers `Id3v2Reader`); the title falls back to the file stem, other fields stay empty/0 when untagged.
//! Open lists the files and `stat`s them; tags are read only for files whose (path, size, mtime) is not in the tag cache,
//! `$XDG_CACHE_HOME/openfader/folder/<FNV-1a 64 of the root path>.bin` (see [`crate::analysis::cache::root`]), one file per
//! root, rewritten (atomically, errors dropped: the tracks are still returned) when a file was read or one went away. Own
//! format, little-endian: `"RKFC"`, `version u32`, `count u32`, then per file `path`, `size u64`, `mtime_ns u128`, and the
//! [`read_track`] fields `title, artist, album, key, genre, comment` (each `len u32` + UTF-8), `bpm_x100 i64`,
//! `length_s i64`, `bitrate u32`, `sample_rate u32`. A wrong magic/version or a short or over-long file is ignored (all files
//! are read again). Bump [`CACHE_VERSION`] when [`read_track`] fills another field or reads tags differently.
//! [`Folder::open_partial`] is the same open without the tags: the cache's tags are lifted, a miss shows the stem,
//! and no file is opened — the rows are there almost at once, while [`Folder::open`] fills the tags behind it.
//! [`Folder::open_level`] lists one level like a file manager: the files directly in the root plus its immediate
//! subfolders, with no recursive walk, so a big disk opens at once instead of after a full scan.
//! A deck decode learns a file's tags for free: [`remember_tags`] keeps them in one small `learned.bin` cache,
//! keyed by the file's path, that any open merges over its listing cache: a played file keeps its tags at
//! whichever level or root the browser reaches it from.

use super::{Library, Playlist, PlaylistRow, Track, playlist_tree, walk_files};
use crate::analysis::cache;
use parking_lot::Mutex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, StandardTagKey};
use symphonia::core::probe::Hint;
use symphonia::core::units::TimeBase;

const AUDIO_EXTS: [&str; 7] = ["mp3", "m4a", "aac", "flac", "wav", "aif", "aiff"];
const CACHE_MAGIC: &[u8; 4] = b"RKFC";
const CACHE_VERSION: u32 = 1;
/// Learned-tag entries kept ([`remember_tags`]): enough to warm a session's picks across folders.
const HOT_MAX: usize = 4096;

pub struct Folder {
    /// (folder relative to root with `/` separators, "" = root; track), sorted by path.
    tracks: Vec<(String, Track)>,
    /// Folders holding audio at any depth, sorted.
    dirs: BTreeSet<String>,
}

/// Whether `p` has a music or video ([`crate::video::is_video`]) file extension.
pub fn is_audio(p: &Path) -> bool {
    crate::paths::has_extension(p, &AUDIO_EXTS) || crate::video::is_video(p)
}

/// Audio files below `dir`, symlinked directories not followed.
pub fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    walk_files(dir, usize::MAX, |p| {
        if is_audio(&p) {
            out.push(p);
        }
    });
}

/// BPM text → ×100: "128", "127.50", "128,5" and "128 BPM" all parse; anything else is 0 (unknown).
fn bpm_x100(v: &str) -> i64 {
    let n: String = v
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
        .collect();
    n.replace(',', ".")
        .parse::<f64>()
        .map_or(0, |b| (b * 100.0).round() as i64)
}

/// ID3v2.3 genre refs: "(17)Rock" → "Rock"; a bare "(17)" / "17" is an index into the ID3v1 list → "".
/// ponytail: symphonia keeps that table private, so unresolved indexes are dropped; depend on symphonia-metadata for `id3v1::util::genre_name` if such tags matter.
fn genre(v: &str) -> &str {
    let v = match v.strip_prefix('(').and_then(|r| r.split_once(')')) {
        Some((n, rest)) if n.bytes().all(|b| b.is_ascii_digit()) => rest,
        _ => v,
    };
    if v.bytes().all(|b| b.is_ascii_digit()) {
        ""
    } else {
        v
    }
}

/// Fills still-empty fields; the first value of a field wins (multi-valued ID3v2.4/Vorbis frames list
/// the primary value first), except artists, which are joined with ", " so no collaborator is lost.
/// The key stays as written (Camelot "8A" or musical "F#m"); it is matched by the last `:` segment so
/// ID3 TXXX and MP4 `----` freeform tags (`…:initialkey`) count too.
fn apply(t: &mut Track, rev: &MetadataRevision) {
    for tag in rev.tags() {
        let v = tag.value.to_string();
        let v = v.trim_matches(|c: char| c.is_whitespace() || c == '\0');
        if v.is_empty() {
            continue;
        }
        match tag.std_key {
            Some(StandardTagKey::TrackTitle) if t.title.is_empty() => t.title = v.into(),
            Some(StandardTagKey::Album) if t.album.is_empty() => t.album = v.into(),
            Some(StandardTagKey::Artist)
                if t.artist != v && !t.artist.split(", ").any(|a| a == v) =>
            {
                if !t.artist.is_empty() {
                    t.artist.push_str(", ");
                }
                t.artist.push_str(v);
            }
            Some(StandardTagKey::Genre) if t.genre.is_empty() => t.genre = genre(v).into(),
            Some(StandardTagKey::Bpm) if t.bpm_x100 == 0 => t.bpm_x100 = bpm_x100(v),
            Some(StandardTagKey::Comment) if t.comment.is_empty() => t.comment = v.into(),
            None if t.key.is_empty()
                && matches!(
                    tag.key
                        .rsplit(':')
                        .next()
                        .unwrap_or("")
                        .to_ascii_uppercase()
                        .as_str(),
                    "TKEY" | "INITIALKEY" | "KEY"
                ) =>
            {
                t.key = v.into()
            }
            _ => {}
        }
    }
}

/// The file name without its extension: a track's title when nothing else names it.
fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A track that is only listed: its stem as the title, everything else default ([`Folder::open_partial`]).
fn stem_track(p: &Path) -> Track {
    Track {
        file: Some(p.to_path_buf()),
        title: stem(p),
        ..Track::default()
    }
}

/// Average bitrate in kbps from the file size and a playing time: what an MP3 or AAC file actually uses, where the
/// container's sample format has no bit depth to report. 0 when either side is unknown.
pub fn average_bitrate(path: &Path, length_s: i64) -> u32 {
    if length_s <= 0 {
        return 0;
    }
    let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
    if bytes == 0 {
        return 0;
    }
    let kbps = bytes.saturating_mul(8) / (length_s as u64).saturating_mul(1000);
    kbps.min(u64::from(u32::MAX)) as u32
}

/// The track a file is: tags and stream facts through symphonia, the title falling back to the file stem.
pub fn read_track(path: &Path) -> Track {
    let stem = stem(path);
    let mut t = Track {
        file: Some(path.to_path_buf()),
        ..Track::default()
    };
    let _ = (|| -> Option<()> {
        let mss = MediaSourceStream::new(
            Box::new(std::fs::File::open(path).ok()?),
            Default::default(),
        );
        let mut hint = Hint::new();
        hint.with_extension(path.extension()?.to_str()?);
        let mut probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .ok()?;
        if let Some(rev) = probed
            .metadata
            .get()
            .as_ref()
            .and_then(|m| m.current().cloned())
        {
            apply(&mut t, &rev);
        }
        if let Some(rev) = probed.format.metadata().current() {
            apply(&mut t, rev);
        }
        let p = &probed.format.default_track()?.codec_params;
        t.sample_rate = p.sample_rate.unwrap_or(0);
        // MP4's n_frames counts media-timescale ticks, which only equals samples by convention: go through the time base.
        if let (Some(n), Some(tb)) = (
            p.n_frames,
            p.time_base
                .or_else(|| p.sample_rate.map(|r| TimeBase::new(1, r))),
        ) {
            let s = tb.calc_time(n);
            t.length_s = (s.seconds as f64 + s.frac).round() as i64;
        }
        if let (Some(bits), Some(ch), Some(r)) = (p.bits_per_sample, p.channels, p.sample_rate) {
            t.bitrate = bits * ch.count() as u32 * r / 1000;
        }
        Some(())
    })();
    let avg = average_bitrate(path, t.length_s);
    if avg > 0 {
        t.bitrate = avg;
    }
    if t.title.is_empty() {
        t.title = stem;
    }
    t
}

/// (size, mtime in ns since the epoch): what a changed file differs in.
type Stamp = (u64, u128);

fn stamp(p: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(p).ok()?;
    let t = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some((m.len(), t.as_nanos()))
}

/// `<cache root>/folder/<hash of the root path>.bin`; `None` when there is no cache directory.
fn cache_file(root: &Path) -> Option<PathBuf> {
    let h = cache::fnv1a(root.to_string_lossy().bytes());
    Some(cache::root()?.join("folder").join(format!("{h:016x}.bin")))
}

/// `<cache folder>/folder/learned.bin`: the tags a deck learned ([`remember_tags`]), keyed by the file's absolute
/// path. One file for every folder: the same track shows the same tags at whichever level or root it is reached.
fn learned_file() -> Option<PathBuf> {
    Some(cache::root()?.join("folder").join("learned.bin"))
}

/// Keeps the tags a deck learned for `track` (its decode already opened the file: bitrate, length, sample rate,
/// BPM and key come for free) in the learned-tag cache, merged over a folder's listing cache by the next open.
/// Small and separate so this never rewrites a folder's big tag cache; a failed write is dropped.
pub fn remember_tags(track: &Track) {
    if let Some(file) = learned_file() {
        remember_at(&file, track);
    }
}

/// [`remember_tags`] writing to `hot` (the test chooses the path).
fn remember_at(hot: &Path, track: &Track) {
    let Some(file) = track.file.as_deref() else {
        return;
    };
    let Some(stamp) = stamp(file) else {
        return;
    };
    // One writer at a time: two deck workers would read-modify-write the same cache.
    static WRITE: Mutex<()> = Mutex::new(());
    let _write = WRITE.lock();
    let mut entries = std::fs::read(hot)
        .map(|b| decode(&b))
        .unwrap_or_default();
    entries.insert(file.to_string_lossy().into_owned(), (stamp, track.clone()));
    if entries.len() > HOT_MAX {
        let mut keys: Vec<_> = entries.keys().cloned().collect();
        keys.sort();
        for key in &keys[..keys.len() - HOT_MAX] {
            entries.remove(key);
        }
    }
    let rows: Vec<_> = entries.iter().map(|(_, (s, t))| (t, *s)).collect();
    let tmp = hot.with_extension(format!("{}.tmp", std::process::id()));
    let _ = hot.parent().map_or(Ok(()), std::fs::create_dir_all);
    let _ = std::fs::write(&tmp, encode(&rows)).and_then(|()| std::fs::rename(&tmp, hot));
}

fn put_str(o: &mut Vec<u8>, s: &str) {
    o.extend_from_slice(&(s.len() as u32).to_le_bytes());
    o.extend_from_slice(s.as_bytes());
}

fn encode(entries: &[(&Track, Stamp)]) -> Vec<u8> {
    let mut o = Vec::new();
    o.extend_from_slice(CACHE_MAGIC);
    o.extend_from_slice(&CACHE_VERSION.to_le_bytes());
    o.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (t, (size, mtime)) in entries {
        put_str(
            &mut o,
            &t.file.as_deref().unwrap_or(Path::new("")).to_string_lossy(),
        );
        o.extend_from_slice(&size.to_le_bytes());
        o.extend_from_slice(&mtime.to_le_bytes());
        for s in [&t.title, &t.artist, &t.album, &t.key, &t.genre, &t.comment] {
            put_str(&mut o, s);
        }
        o.extend_from_slice(&t.bpm_x100.to_le_bytes());
        o.extend_from_slice(&t.length_s.to_le_bytes());
        o.extend_from_slice(&t.bitrate.to_le_bytes());
        o.extend_from_slice(&t.sample_rate.to_le_bytes());
    }
    o
}

/// Cursor over the cache bytes; every read is `None` past the end.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (h, r) = self.0.split_at_checked(n)?;
        self.0 = r;
        Some(h)
    }
    fn num<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }
    fn str(&mut self) -> Option<String> {
        let n = u32::from_le_bytes(self.num()?) as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
}

/// Cached tracks by path; empty when the file is missing, corrupt or from another version.
fn decode(b: &[u8]) -> HashMap<String, (Stamp, Track)> {
    fn go(b: &[u8]) -> Option<HashMap<String, (Stamp, Track)>> {
        let mut r = Reader(b);
        if r.take(4)? != CACHE_MAGIC || r.num()? != CACHE_VERSION.to_le_bytes() {
            return None;
        }
        let n = u32::from_le_bytes(r.num()?);
        let mut out = HashMap::new();
        for _ in 0..n {
            let path = r.str()?;
            let stamp = (u64::from_le_bytes(r.num()?), u128::from_le_bytes(r.num()?));
            let t = Track {
                title: r.str()?,
                artist: r.str()?,
                album: r.str()?,
                key: r.str()?,
                genre: r.str()?,
                comment: r.str()?,
                bpm_x100: i64::from_le_bytes(r.num()?),
                length_s: i64::from_le_bytes(r.num()?),
                bitrate: u32::from_le_bytes(r.num()?),
                sample_rate: u32::from_le_bytes(r.num()?),
                file: Some(PathBuf::from(&path)),
                ..Track::default()
            };
            out.insert(path, (stamp, t));
        }
        r.0.is_empty().then_some(out)
    }
    go(b).unwrap_or_default()
}

/// Whether an open reads tags off the disk: [`Folder::open`]'s full open, or [`Folder::open_partial`]'s
/// listing-only one, which lifts what the tag cache holds and lists a miss without opening the file.
#[derive(Clone, Copy)]
enum Tag {
    Full,
    Cached,
}

impl Folder {
    pub fn open(root: &Path) -> Result<Self, String> {
        Self::open_with(
            root,
            cache_file(root).as_deref(),
            learned_file().as_deref(),
            read_track,
            Tag::Full,
            usize::MAX,
            false,
        )
    }

    /// [`Folder::open`] without opening a file: whatever the tag cache holds is listed, a miss shows the file
    /// stem with no other tags, and the cache is left alone. Almost instant on any disk (the walk and the
    /// `stat`s only): the listing the browser starts from while [`Folder::open`] fills the tags behind it.
    pub fn open_partial(root: &Path) -> Result<Self, String> {
        Self::open_partial_with(root, cache_file(root).as_deref())
    }

    /// One level of the tree, like a file manager: the audio directly in `root` plus its immediate subfolders,
    /// with no recursive walk. Tags are lifted from the cache as in [`Folder::open_partial`] (a miss is the file
    /// stem), so it is the first paint of a folder; its subfolders are opened the same way when picked.
    pub fn open_level(root: &Path) -> Result<Self, String> {
        Self::open_with(
            root,
            cache_file(root).as_deref(),
            learned_file().as_deref(),
            read_track,
            Tag::Cached,
            0,
            true,
        )
    }

    /// [`Folder::open_partial`] with the tag cache file given (the test counts reads).
    fn open_partial_with(root: &Path, cache: Option<&Path>) -> Result<Self, String> {
        Self::open_with(
            root,
            cache,
            learned_file().as_deref(),
            read_track,
            Tag::Cached,
            usize::MAX,
            false,
        )
    }

    /// [`Folder::open`] with the tag cache, the learned-tag cache and the tag reader given (the tests choose
    /// them); `mode` picks the full open or the listing-only one ([`Tag`]), `depth` how far [`walk_files`]
    /// descends and `subdirs` whether the immediate subfolders are listed even when they hold no audio directly.
    fn open_with(
        root: &Path,
        cache: Option<&Path>,
        learned: Option<&Path>,
        read: impl Fn(&Path) -> Track,
        mode: Tag,
        depth: usize,
        subdirs: bool,
    ) -> Result<Self, String> {
        if !root.is_dir() {
            return Err(format!("{} is not a folder", root.display()));
        }
        // Phase 0 instrumentation: where a slow stick spends its time. Everything here runs on a worker thread.
        let mut files = Vec::new();
        let t = Instant::now();
        walk_files(root, depth, |p| {
            if is_audio(&p) {
                files.push(p);
            }
        });
        files.sort();
        let walk_ms = t.elapsed().as_millis();
        let t = Instant::now();
        let mut old = cache
            .and_then(|c| std::fs::read(c).ok())
            .map(|b| decode(&b))
            .unwrap_or_default();
        // What a deck learned is newer than the listing cache ([`remember_tags`]); only the entries under this
        // root matter, keyed as the cache is (the file's full path).
        if let Some(hot) = learned.and_then(|h| std::fs::read(h).ok()) {
            for (path, entry) in decode(&hot) {
                if Path::new(&path).starts_with(root) {
                    old.insert(path, entry);
                }
            }
        }
        let cache_ms = t.elapsed().as_millis();
        let old_len = old.len();
        let mut dirs = BTreeSet::new();
        // A level open shows its subfolders even when they hold no audio directly: they are the way down.
        if subdirs {
            let entries = std::fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))?;
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_dir()) {
                    dirs.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        let mut tracks = Vec::with_capacity(files.len());
        let mut stamps = Vec::with_capacity(files.len());
        let mut dirty = false;
        let mut tags = 0;
        let t = Instant::now();
        let n_files = files.len();
        for f in files {
            let rel = f
                .strip_prefix(root)
                .unwrap_or(&f)
                .parent()
                .unwrap_or(Path::new(""));
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let mut d = rel.as_str();
            while !d.is_empty() {
                dirs.insert(d.to_string());
                d = d.rfind('/').map_or("", |i| &d[..i]);
            }
            let st = stamp(&f);
            let hit = st.and_then(|st| {
                let (old_st, t) = old.remove(&*f.to_string_lossy())?;
                (old_st == st).then_some(t)
            });
            let t = match (mode, hit) {
                (Tag::Full, None) => {
                    dirty = true;
                    tags += 1;
                    read(&f)
                }
                // A listing-only miss names the file and carries no tags: nothing is read ([`Folder::open_partial`]).
                (Tag::Cached, None) => stem_track(&f),
                (_, Some(t)) => t,
            };
            stamps.push(st);
            tracks.push((rel, t));
        }
        let checked_ms = t.elapsed().as_millis();
        let mut saved_ms = 0;
        // A listing-only open never writes: its misses carry no tags ([`Folder::open_partial`]).
        if matches!(mode, Tag::Full)
            && let Some(c) = cache.filter(|_| dirty || old_len != tracks.len())
        {
            let entries: Vec<_> = tracks
                .iter()
                .zip(&stamps)
                .filter_map(|((_, t), s)| Some((t, (*s)?)))
                .collect();
            let tmp = c.with_extension(format!("{}.tmp", std::process::id()));
            let _ = c.parent().map_or(Ok(()), std::fs::create_dir_all);
            // ponytail: a failed cache write is dropped; the tracks are still returned.
            let t = Instant::now();
            let _ = std::fs::write(&tmp, encode(&entries)).and_then(|()| std::fs::rename(&tmp, c));
            saved_ms = t.elapsed().as_millis();
        }
        // Fase 0 instrumentation: where a slow stick spends its time; the listing-only read marks itself.
        let only = match (mode, depth) {
            (Tag::Cached, 0) => ", level",
            (Tag::Full, _) => "",
            (Tag::Cached, _) => ", listing-only",
        };
        eprintln!(
            "sources: Explorer {}: {} files, walk {walk_ms} ms, cache {cache_ms} ms, \
            checked {checked_ms} ms, tags {tags}, saved {saved_ms} ms{only}",
            root.display(),
            n_files
        );
        Ok(Self { tracks, dirs })
    }
}

impl Library for Folder {
    fn kind(&self) -> &str {
        "Folder"
    }

    fn playlists(&self) -> Result<Vec<Playlist>, String> {
        let parents: HashSet<&str> = self
            .dirs
            .iter()
            .filter_map(|d| d.rsplit_once('/').map(|(p, _)| p))
            .collect();
        let rows: Vec<PlaylistRow> = self
            .dirs
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let (parent, name) = d
                    .rsplit_once('/')
                    .map_or((None, d.as_str()), |(p, n)| (Some(p.to_string()), n));
                PlaylistRow {
                    id: d.clone(),
                    name: name.into(),
                    is_folder: parents.contains(d.as_str()),
                    parent,
                    order: i as i64,
                }
            })
            .collect();
        Ok(playlist_tree(&rows))
    }

    /// A folder lists its own audio and that of everything below it.
    fn tracks(&self, playlist: Option<&str>) -> Result<Vec<Track>, String> {
        let t = Instant::now();
        let below = playlist.map(|p| (p, format!("{p}/")));
        let out = self
            .tracks
            .iter()
            .filter(|(d, _)| {
                below
                    .as_ref()
                    .is_none_or(|(p, b)| d == p || d.starts_with(b.as_str()))
            })
            .map(|(_, t)| t.clone())
            .collect::<Vec<_>>();
        // Instrumentation: a UI-thread listing of a big folder shows up here.
        if t.elapsed().as_millis() >= 20 {
            eprintln!(
                "sources: Explorer {}: listed {}/{} tracks in {} ms",
                playlist.unwrap_or("(all)"),
                out.len(),
                self.tracks.len(),
                t.elapsed().as_millis()
            );
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn fake(p: &Path) -> Track {
        Track {
            title: format!("T {}", p.file_stem().unwrap().to_string_lossy()),
            artist: "Ä, B".into(),
            album: "Al\nbum".into(),
            key: "8A".into(),
            genre: "House".into(),
            comment: "c".into(),
            bpm_x100: 12850,
            length_s: 241,
            bitrate: 1411,
            sample_rate: 44100,
            file: Some(p.to_path_buf()),
            ..Track::default()
        }
    }

    #[test]
    fn average_bitrate_reads_the_file_size_over_its_playing_time() {
        let dir = std::env::temp_dir().join(format!("rk-bitrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.mp3");
        std::fs::write(&file, vec![0u8; 1_000_000]).unwrap();
        assert_eq!(average_bitrate(&file, 40), 200);
        assert_eq!(average_bitrate(&file, 0), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_open_reads_no_tags_from_unchanged_files() {
        let dir = std::env::temp_dir().join(format!("rk-foldercache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("lib");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.wav"), b"aaaa").unwrap();
        std::fs::write(root.join("sub/b.mp3"), b"bb").unwrap();
        let cache = dir.join("cache/folder.bin");

        let reads = Cell::new(0);
        let open = || {
            reads.set(0);
            let f = Folder::open_with(
                &root,
                Some(&cache),
                None,
                |p| {
                    reads.set(reads.get() + 1);
                    fake(p)
                },
                Tag::Full,
                usize::MAX,
                false,
            )
            .unwrap();
            f.tracks(None).unwrap()
        };

        let first = open();
        assert_eq!((reads.get(), first.len()), (2, 2));
        assert_eq!(first[0], fake(&root.join("a.wav")));
        assert_eq!(open(), first);
        assert_eq!(reads.get(), 0, "unchanged files come from the cache");

        // A changed file is read again, the other one is not.
        std::fs::write(root.join("a.wav"), b"aaaaaa").unwrap();
        assert_eq!(open(), first);
        assert_eq!(reads.get(), 1);

        // A removed file leaves the cache; a corrupt cache reads everything again.
        std::fs::remove_file(root.join("sub/b.mp3")).unwrap();
        assert_eq!(open().len(), 1);
        assert_eq!(reads.get(), 0);
        std::fs::write(&cache, b"RKFC garbage").unwrap();
        assert_eq!(open().len(), 1);
        assert_eq!(reads.get(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_listing_only_open_lists_the_cache_and_stems_without_reading() {
        let dir = std::env::temp_dir().join(format!("rk-folderpartial-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("lib");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.wav"), b"aaaa").unwrap();
        std::fs::write(root.join("sub/b.mp3"), b"bb").unwrap();
        let cache = dir.join("cache/folder.bin");

        // The full open reads both files and writes the cache.
        let reads = Cell::new(0);
        let full = Folder::open_with(
            &root,
            Some(&cache),
            None,
            |p| {
                reads.set(reads.get() + 1);
                fake(p)
            },
            Tag::Full,
            usize::MAX,
            false,
        )
        .unwrap();
        let full_rows = full.tracks(None).unwrap();
        assert_eq!(reads.get(), 2);

        // A listing-only open reads nothing and lifts the cache's tags: the rows are the same.
        reads.set(0);
        let listed = Folder::open_partial_with(&root, Some(&cache)).unwrap();
        assert_eq!(reads.get(), 0, "the listing-only open reads no tags");
        assert_eq!(listed.tracks(None).unwrap(), full_rows);

        // A new file is listed with its stem and no tags, the read comes later; the folders are there, so the
        // tree's subfolder rows list from the listing-only open too.
        std::fs::write(root.join("new.m4a"), b"n").unwrap();
        let partial = Folder::open_partial_with(&root, Some(&cache)).unwrap();
        assert_eq!(reads.get(), 0);
        assert_eq!(partial.tracks(Some("sub")).unwrap().len(), 1);
        let listed = partial.tracks(None).unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0], full_rows[0]);
        assert_eq!(listed[2], full_rows[1]);
        let new = &listed[1];
        assert_eq!(new.title, "new");
        assert_eq!(new.file.as_deref(), Some(root.join("new.m4a").as_path()));
        assert!(new.artist.is_empty() && new.bpm_x100 == 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_level_open_lists_only_the_root_and_its_immediate_folders() {
        let dir = std::env::temp_dir().join(format!("rk-folderlevel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("lib");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("videos")).unwrap();
        std::fs::write(root.join("a.wav"), b"aaaa").unwrap();
        std::fs::write(root.join("sub/b.mp3"), b"bb").unwrap();
        let cache = dir.join("cache/folder.bin");

        let reads = Cell::new(0);
        let full = Folder::open_with(
            &root,
            Some(&cache),
            None,
            |p| {
                reads.set(reads.get() + 1);
                fake(p)
            },
            Tag::Full,
            usize::MAX,
            false,
        )
        .unwrap();
        assert_eq!(full.tracks(None).unwrap().len(), 2);
        assert_eq!(reads.get(), 2);

        // A level open walks no deeper, reads nothing and still offers the way down: every immediate folder.
        reads.set(0);
        let level = Folder::open_with(
            &root,
            Some(&cache),
            None,
            |p| {
                reads.set(reads.get() + 1);
                fake(p)
            },
            Tag::Cached,
            0,
            true,
        )
        .unwrap();
        assert_eq!(reads.get(), 0);
        let rows = level.tracks(None).unwrap();
        assert_eq!(rows.len(), 1, "only the root's own file");
        assert_eq!(rows[0], fake(&root.join("a.wav")));
        assert!(level.tracks(Some("sub")).unwrap().is_empty());
        assert!(level.dirs.contains("sub") && level.dirs.contains("videos"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_deck_learned_tag_wins_over_the_listing_cache() {
        let dir = std::env::temp_dir().join(format!("rk-folderhot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("lib");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("a.mp3");
        std::fs::write(&file, b"aaaa").unwrap();
        let cache = dir.join("cache/folder.bin");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();

        // No listing cache yet: the tags a deck learned come from the learned-tag cache, without opening the file.
        let mut learned = fake(&file);
        learned.bitrate = 320;
        let hot = dir.join("learned.bin");
        remember_at(&hot, &learned);
        let t = &Folder::open_with(
            &root,
            Some(&cache),
            Some(&hot),
            read_track,
            Tag::Cached,
            usize::MAX,
            false,
        )
        .unwrap()
        .tracks(None)
        .unwrap()[0];
        assert_eq!((t.title.as_str(), t.bitrate), ("T a", 320));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
