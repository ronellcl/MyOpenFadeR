//! Local copies of video files, so a deck keeps playing when its drive is unplugged.
//!
//! A copy goes to `<cache folder>/media/<hash of the original path>.<ext>` with a `.meta` sidecar holding the
//! original's size and modification time. The original path names the copy (not its size or mtime), so the copy is
//! still found when the drive is gone; the sidecar rejects a copy whose original changed. Copies are atomic
//! (`.part` then rename), capped by `OPENFADER_MEDIA_CACHE_GB` (default 20 GB) and evicted least-recently-used
//! first, skipping the ones a deck is using ([`pin`]).

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// Free space kept after a copy, so a full disk never stops the app.
const MARGIN: u64 = 64 << 20;
/// Default cache size; `OPENFADER_MEDIA_CACHE_GB` overrides it (0 = no limit, for a big internal disk).
const DEFAULT_GB: u64 = 20;

/// `<cache folder>/media`: where the copies live.
pub fn dir() -> Option<PathBuf> {
    Some(crate::paths::cache_dir()?.join("media"))
}

/// Hash of the original path: the copy keeps this name even when the original is not mounted.
fn key(path: &Path) -> String {
    format!(
        "{:016x}",
        crate::analysis::cache::fnv1a(path.as_os_str().as_encoded_bytes().iter().copied())
    )
}

/// Extension kept for ffmpeg's sniffing (`mp4`, `mov`, …); `media` when the file has none.
fn ext(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_else(|| "media".into())
}

fn copy_of(dir: &Path, path: &Path) -> PathBuf {
    dir.join(format!("{}.{}", key(path), ext(path)))
}

fn meta_of(dir: &Path, path: &Path) -> PathBuf {
    dir.join(format!("{}.meta", key(path)))
}

/// Original size and modification time as nanoseconds since the epoch.
fn stat(path: &Path) -> Option<(u64, u128)> {
    let m = fs::metadata(path).ok()?;
    let mtime = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Some((m.len(), mtime))
}

fn write_meta(path: &Path, size: u64, mtime: u128) -> Result<(), String> {
    let part = path.with_extension("meta.part");
    fs::write(&part, format!("{size}\n{mtime}\n"))
        .map_err(|e| format!("{}: {e}", part.display()))?;
    fs::rename(&part, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_meta(path: &Path) -> Option<(u64, u128)> {
    let text = fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    Some((lines.next()?.parse().ok()?, lines.next()?.parse().ok()?))
}

/// Marks a copy as recently used, so the LRU keeps it.
fn touch(path: &Path) {
    if let Ok(f) = fs::File::options().write(true).open(path) {
        let _ = f.set_modified(SystemTime::now());
    }
}

/// The local copy of `path`, when it exists and still matches the original (or the original is unmounted).
pub fn cached(path: &Path) -> Option<PathBuf> {
    let dir = dir()?;
    let copy = copy_of(&dir, path);
    if !copy.is_file() {
        return None;
    }
    let (size, mtime) = read_meta(&meta_of(&dir, path))?;
    match stat(path) {
        Some((now_size, now_mtime)) if now_size == size && now_mtime == mtime => {}
        // The drive is gone: the copy and its sidecar are all we have, and they are what protects the deck.
        None => {}
        // The original changed: the copy is stale.
        Some(_) => return None,
    }
    Some(copy)
}

/// The path a deck should read: the local copy when there is one, else the original.
pub fn playable(path: &Path) -> PathBuf {
    match cached(path) {
        Some(copy) => {
            touch(&copy);
            copy
        }
        None => path.to_path_buf(),
    }
}

/// Copies in use by a deck: the LRU never deletes them while a stream may seek.
static PINNED: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Keeps `copy` out of the LRU until [`unpin`].
pub fn pin(copy: &Path) {
    PINNED
        .lock()
        .unwrap()
        .insert(copy.to_path_buf());
}

pub fn unpin(copy: &Path) {
    PINNED.lock().unwrap().remove(copy);
}

fn is_pinned(copy: &Path) -> bool {
    PINNED.lock().unwrap().contains(copy)
}

fn limit() -> u64 {
    std::env::var("OPENFADER_MEDIA_CACHE_GB")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_GB)
        .saturating_mul(1 << 30)
}

static COPY: Mutex<()> = Mutex::new(());

/// Copies `path` to the local cache if it is not there yet; returns the local path.
pub fn ensure(path: &Path) -> Result<PathBuf, String> {
    if let Some(copy) = cached(path) {
        touch(&copy);
        return Ok(copy);
    }
    let _guard = COPY.lock().unwrap();
    if let Some(copy) = cached(path) {
        touch(&copy);
        return Ok(copy);
    }
    let Some(dir) = dir() else {
        return Err("no cache directory (set XDG_CACHE_HOME or HOME)".into());
    };
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let Some((size, mtime)) = stat(path) else {
        return Err(format!("{}: the original is not available", path.display()));
    };
    crate::device::forget(&dir);
    if let Some((_, free)) = crate::device::space(&dir) {
        if free < size.saturating_add(MARGIN) {
            return Err(format!(
                "not enough free space for {} ({} MiB needed, {} MiB free)",
                path.display(),
                size / (1 << 20),
                free / (1 << 20)
            ));
        }
    }
    let copy = copy_of(&dir, path);
    let part = dir.join(format!("{}.part", key(path)));
    fs::copy(path, &part).map_err(|e| format!("{}: {e}", path.display()))?;
    let copied = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if copied != size {
        let _ = fs::remove_file(&part);
        return Err(format!(
            "{}: copied {copied} of {size} bytes",
            path.display()
        ));
    }
    fs::rename(&part, &copy).map_err(|e| format!("{}: {e}", copy.display()))?;
    write_meta(&meta_of(&dir, path), size, mtime)?;
    touch(&copy);
    evict(&dir, limit());
    Ok(copy)
}

/// Copies `path` in the background; a deck that is already playing keeps reading what it has.
pub fn ensure_async(path: PathBuf) {
    if cached(&path).is_some() {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = ensure(&path) {
            eprintln!("video cache: {e}");
        }
    });
}

/// Removes orphaned `.part` files and trims the cache to its limit. Call once at start-up.
pub fn init() {
    let Some(dir) = dir() else { return };
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "part") {
            let _ = fs::remove_file(path);
        }
    }
    evict(&dir, limit());
}

/// Deletes the least recently used copies until `dir` fits in `limit` bytes (0 = no limit).
fn evict(dir: &Path, limit: u64) {
    if limit == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files = Vec::new();
    let mut total = 0u64;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext == "meta" || ext == "part" {
            continue;
        }
        let Ok(m) = entry.metadata() else { continue };
        if !m.is_file() || is_pinned(&path) {
            continue;
        }
        total = total.saturating_add(m.len());
        let when = m.modified().unwrap_or(UNIX_EPOCH);
        files.push((when, m.len(), path));
    }
    if total <= limit {
        return;
    }
    files.sort_by_key(|(when, _, _)| *when);
    for (_, size, path) in files {
        if total <= limit {
            break;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let _ = fs::remove_file(dir.join(format!("{stem}.meta")));
        }
        if fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
}
