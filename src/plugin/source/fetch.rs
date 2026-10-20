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
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::Duration;

/// Time a download may take.
const FETCH_TIMEOUT: Duration = Duration::from_secs(900);
/// Extensions of files a plugin may leave in the download folder while it is still writing them.
const UNFINISHED: [&str; 3] = ["part", "tmp", "ytdl"];

/// A download that is running: the answer, and the door to wake the waiters (`None` while it is not there yet).
type Shared = Arc<(Mutex<Option<Result<PathBuf, String>>>, Condvar)>;

/// The downloads that are running, by the track they are for.
static RUNNING: LazyLock<Mutex<HashMap<String, Shared>>> = LazyLock::new(Default::default);

/// `id` as a file name: ASCII letters, digits, `-` and `_` stay, every other byte becomes `%XX`, so two ids never
/// share a name.
pub(super) fn file_name(id: &str) -> String {
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

/// Whether `r`'s audio is already in the download cache, i.e. [`fetch`] would reuse it instead of asking the plugin.
/// Never blocks and never writes: it only looks the finished file up (a cheap `read_dir` on the source's folder).
pub fn is_downloaded(r: &Remote) -> bool {
    crate::paths::data_dir()
        .map(|base| base.join("plugins").join(&r.plugin).join(&r.source))
        .is_some_and(|dir| downloaded(&dir, &file_name(&r.id)).is_some())
}

/// The audio file of a plugin track: the earlier download if there is one, else what the plugin's `fetch` request
/// writes into `<data dir>/plugins/<plugin>/<source>/`. Blocks until the download is done: call it from a worker.
pub fn fetch(r: &Remote) -> Result<PathBuf, String> {
    let base = crate::paths::data_dir().ok_or("set HOME or XDG_DATA_HOME to download")?;
    fetch_in(registry(), &base.join("plugins"), r)
}

/// [`fetch`] with the plugin registry and the download root given.
pub fn fetch_in(registry: &Registry, root: &Path, r: &Remote) -> Result<PathBuf, String> {
    let dir = root.join(&r.plugin).join(&r.source);
    let name = file_name(&r.id);
    if let Some(done) = downloaded(&dir, &name) {
        return Ok(done);
    }
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

/// The download itself: the plugin writes the audio into the folder and answers with its path.
fn fetch_now(registry: &Registry, dir: &Path, name: &str, r: &Remote) -> Result<PathBuf, String> {
    let plugin = registry
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
