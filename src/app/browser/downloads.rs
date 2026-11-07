//! Downloading plugin tracks in parallel: the track menu's DOWNLOAD acts on every picked cloud track, a few workers
//! share the queue, and the status line reports each one as it finishes.

use super::{App, BrowserState};
use openfader::{library::Track, plugin::source};
use slint::ComponentHandle;
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

/// Workers that download at once: enough to keep the network busy without asking every plugin for everything at once.
const WORKERS: usize = 3;

/// Downloads the plugin tracks of `tracks` in parallel; local tracks and tracks without a remote are ignored.
pub(super) fn start(app: &App, tracks: Vec<Track>) {
    let mut seen = HashSet::new();
    let queue: Vec<Track> = tracks
        .into_iter()
        .filter(|t| t.remote.is_some() && t.file.is_none())
        .filter(|t| {
            let Some(r) = &t.remote else {
                return false;
            };
            seen.insert(format!("{}\u{1}{}\u{1}{}", r.plugin, r.source, r.id))
        })
        .collect();
    let total = queue.len();
    if total == 0 {
        app.global::<BrowserState>()
            .set_status("no plugin tracks to download".into());
        return;
    }
    app.global::<BrowserState>()
        .set_status(format!("downloading {total} track(s)…").into());
    let queue = Arc::new(Mutex::new(VecDeque::from(queue)));
    let done = Arc::new(AtomicUsize::new(0));
    let ok = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let weak = app.as_weak();
    std::thread::spawn(move || {
        std::thread::scope(|scope| {
            for _ in 0..WORKERS.min(total) {
                let queue = Arc::clone(&queue);
                let done = Arc::clone(&done);
                let ok = Arc::clone(&ok);
                let failed = Arc::clone(&failed);
                let weak = weak.clone();
                scope.spawn(move || loop {
                    let track = queue
                        .lock()
                        .expect("the download queue is not poisoned")
                        .pop_front();
                    let Some(track) = track else {
                        break;
                    };
                    let Some(remote) = track.remote.as_ref() else {
                        continue;
                    };
                    let result = source::fetch_named(remote, &track.title, &track.artist);
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    let message = match &result {
                        Ok(_) => {
                            ok.fetch_add(1, Ordering::Relaxed);
                            format!("downloaded {n}/{total}: {}", track.title)
                        }
                        Err(e) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            format!("failed {n}/{total}: {} — {e}", track.title)
                        }
                    };
                    let weak = weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = weak.upgrade() {
                            app.global::<BrowserState>().set_status(message.into());
                        }
                    });
                });
            }
        });
        let (ok, failed) = (ok.load(Ordering::Relaxed), failed.load(Ordering::Relaxed));
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = weak.upgrade() {
                app.global::<BrowserState>().set_status(
                    format!("downloads finished: {ok} ready, {failed} failed").into(),
                );
                if let Some(view) = super::view() {
                    let mut v = view.borrow_mut();
                    v.refresh(&app);
                }
            }
        });
    });
}
