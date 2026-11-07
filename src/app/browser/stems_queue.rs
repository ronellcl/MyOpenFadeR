//! The queue of a batch stem run, kept as JSON so closing the app pauses rather than loses it: what is still
//! pending, how many files were done, and whether the user paused it.

use openfader::library::Track;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Item {
    pub title: String,
    pub artist: String,
    pub file: PathBuf,
    pub length_s: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Queue {
    pub version: u32,
    pub title: String,
    pub pending: Vec<Item>,
    pub done: u64,
    pub skipped: u64,
    pub failed: u64,
    pub paused: bool,
}

impl Queue {
    pub(super) fn new(title: &str, tracks: &[Track]) -> Self {
        let mut seen = HashSet::new();
        let pending = tracks
            .iter()
            .filter_map(|t| {
                let file = t.file.clone()?;
                if !seen.insert(file.clone()) {
                    return None;
                }
                Some(Item {
                    title: t.title.clone(),
                    artist: t.artist.clone(),
                    file,
                    length_s: t.length_s,
                })
            })
            .collect();
        Self {
            version: VERSION,
            title: title.to_owned(),
            pending,
            done: 0,
            skipped: 0,
            failed: 0,
            paused: false,
        }
    }

    pub(super) fn tracks(&self) -> Vec<Track> {
        self.pending
            .iter()
            .map(|i| Track {
                file: Some(i.file.clone()),
                title: i.title.clone(),
                artist: i.artist.clone(),
                length_s: i.length_s,
                ..Default::default()
            })
            .collect()
    }
}

fn path() -> Option<PathBuf> {
    Some(openfader::paths::data_dir()?.join("stems-queue.json"))
}

pub(super) fn load() -> Option<Queue> {
    let bytes = std::fs::read(path()?).ok()?;
    let q: Queue = serde_json::from_slice(&bytes).ok()?;
    (q.version == VERSION && !q.pending.is_empty()).then_some(q)
}

pub(super) fn save(q: &Queue) {
    let Some(path) = path() else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec_pretty(q) else {
        return;
    };
    let part = path.with_extension("json.part");
    if std::fs::write(&part, bytes).is_ok() {
        let _ = std::fs::rename(&part, &path);
    }
}

pub(super) fn clear() {
    if let Some(path) = path() {
        let _ = std::fs::remove_file(&path);
    }
}
