//! The folders the user pinned (FAVORITES): the ones they use every time, wherever the volume is.
//!
//! A pin is the volume's **label** and the folder's path **below** it (`USB/Musica/Rock`), never the mount
//! path: a stick that changes letter (E: → F:) keeps its pins. The settings file holds one `pinned_folder`
//! line per pin, the value without quotes (the parser does not take them off).
//!
//! A pin whose volume is not mounted is not shown: the tree lists what is there now, and the pins come back
//! when the volume is plugged in again.

use crate::app::prefs;
use openfader::device::{self, Device};
use openfader::library::Library;
use slint::ComponentHandle;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// A pinned folder: the volume's label and the path of the folder below it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pinned {
    /// The volume's label, as the device shows it.
    pub label: String,
    /// The folder's path below the volume, with the platform's separator.
    pub path: PathBuf,
}

impl Pinned {
    /// The pin as the settings file holds it: the label and the path, separated by a slash.
    pub fn as_text(&self) -> String {
        format!("{}/{}", self.label, self.path.to_string_lossy().replace('\\', "/"))
    }

    /// The pin of a volume and a folder below it.
    pub fn of(d: &Device, path: &Path) -> Option<Self> {
        let rel = path.strip_prefix(&d.mount).ok()?;
        Some(Self {
            label: d.label.clone(),
            path: rel.to_path_buf(),
        })
    }

    /// The pin of a text from the settings file.
    pub fn from_text(text: &str) -> Option<Self> {
        let (label, path) = text.split_once('/')?;
        if label.is_empty() {
            return None;
        }
        Some(Self {
            label: label.to_owned(),
            path: PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR)),
        })
    }
}

/// The pins of the settings file, in the order they are stored.
pub fn stored() -> Vec<Pinned> {
    prefs::get()
        .pinned_folders
        .iter()
        .filter_map(|t| Pinned::from_text(t))
        .collect()
}

/// The pins whose volume is mounted now, with the folder's path on the disk. Alphabetical, as the tree
/// shows them.
pub fn mounted() -> Vec<(Pinned, PathBuf)> {
    let devices = device::scan();
    let mut out: Vec<(Pinned, PathBuf)> = stored()
        .into_iter()
        .filter_map(|p| {
            let d = devices.iter().find(|d| d.label == p.label)?;
            let path = d.mount.join(&p.path);
            path.is_dir().then(|| (p, path))
        })
        .collect();
    out.sort_by(|a, b| a.0.label.cmp(&b.0.label).then(a.0.path.cmp(&b.0.path)));
    out
}

/// Whether the folder is pinned.
pub fn is_pinned(p: &Pinned) -> bool {
    stored().contains(p)
}

/// Pins the folder (and saves the settings).
#[allow(dead_code)]
pub fn pin(p: &Pinned) {
    let text = p.as_text();
    prefs::update(|s| {
        if !s.pinned_folders.contains(&text) {
            s.pinned_folders.push(text);
        }
    });
}

/// Takes the pin off (and saves the settings).
#[allow(dead_code)]
pub fn unpin(p: &Pinned) {
    let text = p.as_text();
    prefs::update(|s| s.pinned_folders.retain(|t| *t != text));
}

/// Re-indexes the pins of the volume `label` (it has just been plugged in): the search finds its tracks again without
/// waiting for the folder to be pinned again. On a worker thread: reading a big folder takes a while.
pub fn reindex_volume(label: String) {
    std::thread::spawn(move || {
        let Some(dev) = device::scan().into_iter().find(|d| d.label == label) else {
            return;
        };
        for p in stored().into_iter().filter(|p| p.label == label) {
            index(&p, &dev.mount);
        }
    });
}

/// Reads the folder of the pin `p` (the volume is mounted at `mount`) and keeps what it holds in the index: the
/// search finds the tracks of a pinned folder without walking the stick again. Returns how many rows it holds.
pub fn index(p: &Pinned, mount: &Path) -> usize {
    let t_walk = Instant::now();
    let mut files = Vec::new();
    openfader::library::folder::walk(&mount.join(&p.path), &mut files);
    let walk_ms = t_walk.elapsed().as_millis();
    let t_tags = Instant::now();
    let mut rows = Vec::with_capacity(files.len());
    for f in &files {
        let t = openfader::library::folder::read_track(f);
        let rel = f.strip_prefix(mount).unwrap_or(f);
        let (size, mtime) = match f.metadata() {
            Ok(m) => (
                m.len() as i64,
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs() as i64),
            ),
            Err(_) => (0, 0),
        };
        rows.push(openfader::store::index::Row_ {
            id: 0,
            volume: p.label.clone(),
            path: rel.to_string_lossy().into_owned(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            genre: t.genre.clone(),
            bpm_x100: t.bpm_x100,
            length_s: t.length_s,
            key: t.key.clone(),
            size,
            mtime,
        });
    }
    let store = openfader::store::global();
    let n = rows.len();
    let tags_ms = t_tags.elapsed().as_millis();
    let t_put = Instant::now();
    let written = store.index_put(&p.label, &rows).ok();
    let put_ms = t_put.elapsed().as_millis();
    eprintln!(
        "sources: pin {}: {} files, walk {walk_ms} ms, tags {tags_ms} ms, index {put_ms} ms",
        p.as_text(),
        files.len()
    );
    if let Some(written) = written {
        return written;
    }
    n
}

impl super::View {
    /// The pin of the chosen tree row, when it is a folder that can be pinned.
    pub(super) fn chosen_pin(&self) -> Option<Pinned> {
        match &self.entry {
            super::sidebar::Entry::DeviceFolder(d, j) => {
                let source = self.sources.get(*d)?;
                let device = match &source.kind {
                    super::sources::Kind::Device(d) => d,
                    _ => return None,
                };
                let path = source.folders.get(*j)?.path.clone();
                Pinned::of(device, &path)
            }
            super::sidebar::Entry::Pinned(i) => {
                let source = self.sources.get(*i)?;
                let (label, path) = match &source.kind {
                    super::sources::Kind::Pinned { label, path } => (label.clone(), path.clone()),
                    _ => return None,
                };
                // The pin is the label and the path below the volume: the path is what the volume holds now.
                let device = device::scan().into_iter().find(|d| d.label == label)?;
                let rel = path.strip_prefix(&device.mount).ok()?.to_path_buf();
                Some(Pinned {
                    label,
                    path: rel,
                })
            }
            super::sidebar::Entry::DeviceFolderList(d, j, k) => {
                let source = self.sources.get(*d)?;
                let device = match &source.kind {
                    super::sources::Kind::Device(d) => d,
                    _ => return None,
                };
                let folder = source.folders.get(*j)?;
                let path = folder.path.join(&folder.playlists.get(*k)?.id);
                Pinned::of(device, &path)
            }
            _ => None,
        }
    }

    /// The absolute path of a folder row (a device folder or one of its levels).
    fn folder_path(&self, entry: &super::sidebar::Entry) -> Option<std::path::PathBuf> {
        match entry {
            super::sidebar::Entry::DeviceFolder(d, j) => {
                Some(self.sources.get(*d)?.folders.get(*j)?.path.clone())
            }
            super::sidebar::Entry::DeviceFolderList(d, j, k) => {
                let folder = self.sources.get(*d)?.folders.get(*j)?;
                Some(folder.path.join(&folder.playlists.get(*k)?.id))
            }
            _ => None,
        }
    }

    /// A folder command from the tree's right-click menu.
    pub(super) fn folder_action(&mut self, app: &super::App, row: usize, what: &str) {
        let Some(entry) = self.entries.get(row).cloned() else {
            return;
        };
        let folder = matches!(
            entry,
            super::sidebar::Entry::DeviceFolder(..)
                | super::sidebar::Entry::DeviceFolderList(..)
                | super::sidebar::Entry::Pinned(_)
        );
        let playlist = matches!(
            &entry,
            super::sidebar::Entry::User(id)
                if self
                    .nodes
                    .iter()
                    .any(|n| n.id == *id && n.kind == openfader::store::NodeKind::Playlist)
        );
        if !folder && !playlist {
            return;
        }
        match what {
            "stems" => super::stems_batch::prepare_entry(self, app, &entry),
            _ if !folder => {}
            "favorite" => self.set_favorite(app, entry, true),
            "unfavorite" => self.set_favorite(app, entry, false),
            "analyze" => {
                let Some(path) = self.folder_path(&entry) else {
                    return;
                };
                let weak = app.as_weak();
                app.global::<super::BrowserState>().set_status(
                    format!("{}: listing the folder to analyse…", path.display()).into(),
                );
                std::thread::spawn(move || {
                    let tracks = openfader::library::folder::Folder::open_partial(&path)
                        .and_then(|f| f.tracks(None))
                        .map(|tracks| {
                            tracks
                                .into_iter()
                                .filter(|t| {
                                    t.file.is_some()
                                        && openfader::analysis::cache::analyzer_version(t)
                                            < openfader::analysis::cache::ANALYZER_VERSION
                                })
                                .collect::<Vec<_>>()
                        });
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(app) = weak.upgrade() else { return };
                        match tracks {
                            Ok(tracks) if tracks.is_empty() => app
                                .global::<super::BrowserState>()
                                .set_status("every track in this folder is already analysed".into()),
                            Ok(tracks) => {
                                let n = tracks.len();
                                crate::app::analyze::request(&tracks);
                                app.global::<super::BrowserState>().set_status(
                                    format!("analysing {n} track(s) from the folder").into(),
                                );
                            }
                            Err(e) => app.global::<super::BrowserState>().set_status(
                                format!("cannot read the folder: {e}").into(),
                            ),
                        }
                    });
                });
            }
            "refresh" => {
                self.levels.clear();
                self.deep.clear();
                self.refresh_sources(app);
                self.load(app);
            }
            _ => {}
        }
    }

    /// The FAV button: pins the chosen folder, or takes the pin off.
    pub(super) fn toggle_favorite(&mut self, app: &super::App) {
        let Some(p) = self.chosen_pin() else {
            return;
        };
        let pinned = !is_pinned(&p);
        self.apply_pin(app, p, pinned);
    }

    /// The tree menu's "Favorite folder" / "Remove from favorites": pins or unpins the
    /// folder of `entry` (the right-clicked row, which is not necessarily the chosen one).
    pub(super) fn set_favorite(
        &mut self,
        app: &super::App,
        entry: super::sidebar::Entry,
        pinned: bool,
    ) {
        self.entry = entry;
        let Some(p) = self.chosen_pin() else {
            return;
        };
        if is_pinned(&p) == pinned {
            self.say(
                app,
                if pinned {
                    format!("{} is already a favorite", p.label)
                } else {
                    format!("{} is not a favorite", p.label)
                },
            );
            return;
        }
        self.apply_pin(app, p, pinned);
    }

    /// Pins or unpins `p`, then saves the settings and rebuilds the tree on the event loop
    /// (never while the View borrow is held: `prefs::update` runs its listeners).
    fn apply_pin(&mut self, app: &super::App, p: Pinned, pinned: bool) {
        let text = p.as_text().to_owned();
        let app_weak = app.as_weak();

        if pinned {
            self.say(app, format!("{} is pinned", p.label));
            self.index_pin(app, &p);
        } else {
            self.say(app, format!("{} is not pinned any more", p.label));
        }
        // The tree lists the pins: build it again.
        self.refresh_sources(app);
        // Persist the prefs change on the Slint event loop, after this handler returns and
        // the View borrow is released. Calling prefs::update() synchronously here triggers
        // prefs::on_change listeners that borrow View again while folder_action still holds
        // the RefCell borrow, causing a RefCell already borrowed panic.
        let _ = slint::invoke_from_event_loop(move || {
            let Some(app) = app_weak.upgrade() else { return };
            prefs::update(|s| {
                if pinned {
                    if !s.pinned_folders.contains(&text) {
                        s.pinned_folders.push(text);
                    }
                } else {
                    s.pinned_folders.retain(|t| *t != text);
                }
            });
            // Notify listeners now that View is no longer borrowed, so the UI reflects
            // the new prefs (e.g. favorites section in the tree).
            prefs::notify();
            // Rebuild the sources tree so the newly pinned folder shows up immediately,
            // not only after a restart. collect() reads prefs::get().pinned_folders, which
            // now contains the added pin (or no longer contains the removed one).
            if let Some(view) = super::view() {
                let mut v = view.borrow_mut();
                v.refresh_sources(&app);
            }
        });
    }

    /// Indexes the pinned folder in the background: the search finds its tracks without walking the stick again.
    fn index_pin(&self, app: &super::App, p: &Pinned) {
        let Some(mount) = device::scan()
            .into_iter()
            .find(|d| d.label == p.label)
            .map(|d| d.mount)
        else {
            return;
        };
        let p = p.clone();
        let weak = app.as_weak();
        std::thread::spawn(move || {
            let n = index(&p, &mount);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = weak.upgrade() {
                    app.global::<super::BrowserState>()
                        .set_status(format!("{}: {n} tracks indexed", p.label).into());
                }
            });
        });
    }
}
