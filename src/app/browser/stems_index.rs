//! Which tracks have their stems ready in the cache: the browser's STEMS column reads this set instead of
//! stat'ing the cache for every row. A background pass fills it once; the batch and the live separation add
//! their files as they finish.

use super::App;
use openfader::stems::{Quality, cache};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use slint::ComponentHandle;

/// Files whose stems are in the cache (QUALITY or SPEED).
static READY: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Default::default);

/// Whether the one background pass over the library already started.
static SCANNED: AtomicBool = AtomicBool::new(false);

/// Whether `file` has stems ready in the cache.
pub(super) fn ready(file: &Path) -> bool {
    READY
        .lock()
        .expect("the stems index is not poisoned")
        .contains(file)
}

/// Marks `file` ready (a separation just finished).
pub(super) fn mark_ready(file: &Path) {
    READY
        .lock()
        .expect("the stems index is not poisoned")
        .insert(file.to_path_buf());
}

/// Starts the one background pass over `files`; the table refreshes once when it is done.
pub(super) fn ensure(app: &App, files: Vec<PathBuf>) {
    if files.is_empty() || SCANNED.swap(true, Ordering::Relaxed) {
        return;
    }
    let weak = app.as_weak();
    std::thread::spawn(move || {
        for file in files {
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
