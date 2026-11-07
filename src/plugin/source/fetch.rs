//! Downloading a plugin track: the plugin's `fetch` request writes the audio into a per-source folder, and a finished
//! download found there is reused without asking the plugin again.
//!
//! Two downloads of the same track never run at once: the second one waits for the first (the prefetcher asks for
//! the track that comes next while the deck asks for the one it is loading).

use super::super::{Registry, registry};
use crate::library::Remote;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, LazyLock, Mutex, RwLock};
use std::time::{Duration, SystemTime};

/// Time a download may take.
const FETCH_TIMEOUT: Duration = Duration::from_secs(900);
/// Time the answer about the titles of a few tracks may take.
const INFO_TIMEOUT: Duration = Duration::from_secs(120);
/// Extensions of files a plugin may leave in the download folder while it is still writing them.
const UNFINISHED: [&str; 3] = ["part", "tmp", "ytdl"];
/// Hidden index of a source folder: track id → downloaded file name.
const INDEX_FILE: &str = ".openfader-downloads.json";
/// Longest name we build for a download, in characters.
const NAME_MAX: usize = 180;

/// A download that is running: the answer, and the door to wake the waiters (`None` while it is not there yet).
type Shared = Arc<(Mutex<Option<Result<PathBuf, String>>>, Condvar)>;

/// The downloads that are running, by the track they are for.
static RUNNING: LazyLock<Mutex<HashMap<String, Shared>>> = LazyLock::new(Default::default);

/// The folder downloads go to when the user chose one (`None` = the data folder).
static DOWNLOAD_ROOT: LazyLock<RwLock<Option<PathBuf>>> = LazyLock::new(Default::default);

/// The index of each source folder, with the mtime it was read at.
static INDEX: LazyLock<Mutex<HashMap<PathBuf, (Option<SystemTime>, HashMap<String, String>)>>> =
    LazyLock::new(Default::default);

/// One writer of the index files at a time.
static INDEX_LOCK: LazyLock<Mutex<()>> = LazyLock::new(Default::default);

/// `id` as a file name: ASCII letters, digits, `-` and `_` stay, every other byte becomes `%XX`, so two ids never
/// share a name.
pub fn file_name(id: &str) -> String {
    id.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The finished download of `name` in `dir`: a non-empty file whose name without extension is `name`.
pub(super) fn downloaded(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_stem().is_some_and(|s| s == name)
                && !crate::paths::has_extension(p, &UNFINISHED)
                && p.metadata().is_ok_and(|m| m.is_file() && m.len() > 0)
        })
}

/// The hidden index of `dir`: track id → file name.
fn index_path(dir: &Path) -> PathBuf {
    dir.join(INDEX_FILE)
}

fn read_index(dir: &Path) -> HashMap<String, String> {
    std::fs::read(index_path(dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn index_mtime(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(index_path(dir))
        .ok()
        .and_then(|m| m.modified().ok())
}

/// The index of `dir`, read again only when its file changed.
fn index_for(dir: &Path) -> HashMap<String, String> {
    let mtime = index_mtime(dir);
    let mut cache = INDEX.lock().expect("the download index is not poisoned");
    if let Some((at, map)) = cache.get(dir) {
        if *at == mtime {
            return map.clone();
        }
    }
    let map = read_index(dir);
    cache.insert(dir.to_path_buf(), (mtime, map.clone()));
    map
}

/// Writes the index of `dir` atomically and refreshes the cache.
fn write_index(dir: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    let _one = INDEX_LOCK.lock().expect("the index writer is not poisoned");
    crate::paths::at(std::fs::create_dir_all(dir), dir)?;
    let path = index_path(dir);
    let part = dir.join(format!("{INDEX_FILE}.part"));
    let bytes = serde_json::to_vec_pretty(map).map_err(|e| format!("download index: {e}"))?;
    crate::paths::at(std::fs::write(&part, &bytes), &part)?;
    crate::paths::at(std::fs::rename(&part, &path), &path)?;
    INDEX.lock()
        .expect("the download index is not poisoned")
        .insert(dir.to_path_buf(), (index_mtime(dir), map.clone()));
    Ok(())
}

/// The download of `id` in `dir`, from the index or (older downloads) from its id.
fn indexed(dir: &Path, id: &str) -> Option<PathBuf> {
    if let Some(name) = index_for(dir).get(id) {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    downloaded(dir, &file_name(id))
}

fn remember(dir: &Path, id: &str, file: &Path) -> Result<(), String> {
    let Some(name) = file.file_name() else {
        return Ok(());
    };
    let mut map = index_for(dir);
    map.insert(id.to_owned(), name.to_string_lossy().into_owned());
    write_index(dir, &map)
}

/// A file name built from `artist` and `title`, safe on every platform; the id when there is nothing usable.
fn safe_name(title: &str, artist: &str, id: &str) -> String {
    let (title, artist) = (title.trim(), artist.trim());
    let raw = if artist.is_empty() {
        title.to_owned()
    } else if title.is_empty() {
        artist.to_owned()
    } else {
        format!("{artist} - {title}")
    };
    let mut out: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                ' '
            } else {
                c
            }
        })
        .collect();
    out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    if out.is_empty() {
        return file_name(id);
    }
    if out.chars().count() > NAME_MAX {
        out = out.chars().take(NAME_MAX).collect();
    }
    let lower = out.to_lowercase();
    let reserved = matches!(
        lower.as_str(),
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    );
    if reserved {
        out.insert(0, '_');
    }
    out
}

/// `base`, or `base (2)`, `base (3)`… when another download of `dir` already uses that name.
fn unique_name(
    dir: &Path,
    base: &str,
    id: &str,
    map: &HashMap<String, String>,
) -> String {
    let mut name = base.to_owned();
    let mut n = 2;
    loop {
        let taken = map.iter().any(|(key, file)| {
            key != id
                && Path::new(file)
                    .file_stem()
                    .is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case(&name))
        }) || downloaded(dir, &name).is_some();
        if !taken {
            return name;
        }
        name = format!("{base} ({n})");
        n += 1;
    }
}

/// The folder downloads go to: the user's choice, else `<data dir>/plugins`.
pub fn download_root() -> Option<PathBuf> {
    DOWNLOAD_ROOT
        .read()
        .ok()
        .and_then(|root| root.clone())
        .or_else(|| crate::paths::data_dir().map(|d| d.join("plugins")))
}

/// Sets the folder downloads go to (`None` = `<data dir>/plugins`).
pub fn set_download_root(root: Option<PathBuf>) {
    *DOWNLOAD_ROOT
        .write()
        .expect("the download root is not poisoned") = root;
}

/// Whether `r`'s audio is already in the download folder, i.e. [`fetch`] would reuse it without asking the plugin.
/// Never blocks and never writes: it only looks the finished file up.
pub fn is_downloaded(r: &Remote) -> bool {
    download_root().is_some_and(|root| {
        let dir = root.join(&r.plugin).join(&r.source);
        indexed(&dir, &r.id).is_some()
    })
}

/// The audio file of a plugin track: the earlier download if there is one, else what the plugin's `fetch` request
/// writes into the download folder. Blocks until the download is done: call it from a worker.
pub fn fetch(r: &Remote) -> Result<PathBuf, String> {
    let root = download_root().ok_or("set HOME or XDG_DATA_HOME to download")?;
    fetch_in(registry(), &root, r)
}

/// [`fetch`] with the track's title and artist, so the file is named after them instead of the id.
pub fn fetch_named(r: &Remote, title: &str, artist: &str) -> Result<PathBuf, String> {
    let root = download_root().ok_or("set HOME or XDG_DATA_HOME to download")?;
    fetch_in_named(registry(), &root, r, Some((title, artist)))
}

/// [`fetch`] with the plugin registry and the download root given.
pub fn fetch_in(registry: &Registry, root: &Path, r: &Remote) -> Result<PathBuf, String> {
    fetch_in_named(registry, root, r, None)
}

/// [`fetch_in`] with an optional human name (`title`, `artist`) for the downloaded file.
pub fn fetch_in_named(
    registry: &Registry,
    root: &Path,
    r: &Remote,
    display: Option<(&str, &str)>,
) -> Result<PathBuf, String> {
    let dir = root.join(&r.plugin).join(&r.source);
    if let Some(done) = indexed(&dir, &r.id) {
        return Ok(done);
    }
    let name = match display {
        Some((title, artist)) => {
            let base = safe_name(title, artist, &r.id);
            unique_name(&dir, &base, &r.id, &index_for(&dir))
        }
        None => file_name(&r.id),
    };
    let key = format!("{}\u{1}{}\u{1}{}", r.plugin, r.source, r.id);
    // The download that is already running for this track: wait for its answer instead of asking the plugin again
    // (the deck loads a track the prefetcher is already downloading).
    let (shared, mine) = {
        let mut running = RUNNING.lock().expect("the running downloads are not poisoned");
        match running.get(&key) {
            Some(shared) => (shared.clone(), false),
            None => {
                let shared: Shared = Arc::new((Mutex::new(None), Condvar::new()));
                running.insert(key.clone(), shared.clone());
                (shared, true)
            }
        }
    };
    if !mine {
        let (lock, door) = &*shared;
        let mut answer = lock.lock().expect("the answer of the download is not poisoned");
        while answer.is_none() {
            answer = door.wait(answer).expect("the answer of the download is not poisoned");
        }
        return answer.clone().expect("the answer of the download is there");
    }
    let result = fetch_now(registry, &dir, &name, r);
    if let Ok(file) = &result {
        // The index is only a finding aid: a failed write must not lose a good download.
        let _ = remember(&dir, &r.id, file);
    }
    {
        let (lock, door) = &*shared;
        *lock.lock().expect("the answer of the download is not poisoned") = Some(result.clone());
        door.notify_all();
    }
    RUNNING
        .lock()
        .expect("the running downloads are not poisoned")
        .remove(&key);
    result
}

/// The title and the artist of the tracks of a source, by their id: what a file downloaded before the plugin wrote its
/// tags shows (the file is named after the id). The plugin answers from its own catalogue.
pub fn info(plugin: &str, source: &str, ids: &[String]) -> Result<Vec<(String, String, String)>, String> {
    let p = registry()
        .get(plugin)
        .ok_or_else(|| format!("plugin {plugin} is not installed"))?;
    let reply = p.call(
        &json!({"op": "info", "source": source, "ids": ids}),
        INFO_TIMEOUT,
    )?;
    let mut out = Vec::new();
    for t in reply["tracks"].as_array().into_iter().flatten() {
        let (Some(id), Some(title)) = (t["id"].as_str(), t["title"].as_str()) else {
            continue;
        };
        out.push((
            id.to_owned(),
            title.to_owned(),
            t["artist"].as_str().unwrap_or_default().to_owned(),
        ));
    }
    Ok(out)
}

/// The download itself: the plugin writes the audio into the folder and answers with its path.
fn fetch_now(registry: &Registry, dir: &Path, name: &str, r: &Remote) -> Result<PathBuf, String> {    let plugin = registry
        .get(&r.plugin)
        .ok_or_else(|| format!("plugin {} is not installed", r.plugin))?;
    crate::paths::at(std::fs::create_dir_all(dir), dir)?;
    let reply = plugin.call(
        &json!({"op": "fetch", "source": r.source, "id": r.id, "dir": dir.to_string_lossy(), "name": name}),
        FETCH_TIMEOUT,
    )?;
    let file = reply["file"]
        .as_str()
        .map(|f| dir.join(f)) // an absolute path stays as it is
        .ok_or_else(|| format!("{}: the reply has no `file`", r.plugin))?;
    if !file.is_file() {
        return Err(format!("{}: {} was not written", r.plugin, file.display()));
    }
    Ok(file)
}
