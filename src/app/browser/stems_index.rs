//! Which tracks have their stems ready in the cache: the browser's STEMS column reads this set instead of
//! stat'ing the cache for every row. Each list adds its files to a background pass; the batch and the live
//! separation add their files as they finish.

use super::App;
use openfader::stems::{Quality, cache};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};
use slint::ComponentHandle;

/// Files whose stems are in the cache (QUALITY or SPEED).
static READY: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Default::default);

/// Files already handled by a pass, so a new list only scans what it brings.
static SEEN: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Default::default);

/// Whether `file` has stems ready in the cache.
pub(super) fn ready(file: &Path) -> bool {
    READY
        .lock()
        .expect("the stems index is not poisoned")
        .contains(file)
}

/// Marks `file` ready (a separation just finished, or the batch found it cached).
pub(super) fn mark_ready(file: &Path) {
    SEEN
        .lock()
        .expect("the stems index is not poisoned")
        .insert(file.to_path_buf());
    READY
        .lock()
        .expect("the stems index is not poisoned")
        .insert(file.to_path_buf());
}

/// Starts a background pass over the files this list adds; the table refreshes once when it is done.
pub(super) fn ensure(app: &App, files: Vec<PathBuf>) {
    let pending: Vec<PathBuf> = {
        let mut seen = SEEN
            .lock()
            .expect("the stems index is not poisoned");
        files
            .into_iter()
            .filter(|file| seen.insert(file.clone()))
            .collect()
    };
    if pending.is_empty() {
        return;
    }
    let weak = app.as_weak();
    std::thread::spawn(move || {
        for file in pending {
            if cache::has(&file, Quality::Quality) || cache::has(&file, Quality::Speed) {
                mark_ready(&file);
            }
        }
        let _ = slint::invoke_from_event_loop(move || {
            let Some(app) = weak.upgrade() else { return };
            if let Some(view) = super::view() {
                view.borrow_mut().refresh(&app);
            }
        });
    });
}
