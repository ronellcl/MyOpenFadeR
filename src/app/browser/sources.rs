//! External sources beside the opened library in the sidebar (the Media Browser): the mounted devices under "Devices", the
//! library plugins that name a library file in their settings (each enabled one with a file set gets a row under its
//! manifest's label) and (when Preferences > View > Layout > Media Browser enables it) Explorer (a folder of music). A source
//! opens when its row is chosen; its lists are read-only and join the Collection through IMPORT TO COLLECTION (File and
//! Track menus, the strip button). Devices are polled once a second: plugging one in or out updates the tree, and a device
//! with a library is read as soon as it appears. Every source is read on a worker thread (a USB stick or a big Explorer tree
//! takes seconds), with its steps in the status bar's loading line ([`loading`]) and a toast when done.

use super::{
    loading,
    sidebar::{Entry, SourceRows},
    App, BrowserState, Home, View,
};
use crate::app::{prefs, toast};
use openfader::{
    device::{self, plug, Device},
    library::{self, folder::Folder, Library, Playlist, Track},
    plugin, store,
};
use slint::ComponentHandle;
use std::{path::PathBuf, rc::Rc, time::Instant};

/// Where a source comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Device(Device),
    /// A library plugin's library file (the plugin's Media Browser setting), shown under the manifest's `label`.
    Plugin {
        plugin: String,
        label: String,
        path: PathBuf,
    },
    /// The root folder of Explorer.
    Explorer(PathBuf),
    /// The folder where a plugin source keeps the audio it downloaded (`<data>/plugins/<plugin>/<source>`).
    Downloaded {
        plugin: String,
        source: String,
        label: String,
        path: PathBuf,
    },
}

pub struct Source {
    pub kind: Kind,
    pub name: String,
    /// Read when the source is first chosen.
    pub lib: Option<Rc<dyn Library>>,
    pub playlists: Vec<Playlist>,
    /// The tracks of the source the Collection does not have (the files it lists that the Collection lacks).
    pub missing: Vec<Track>,
    /// Being read on a worker thread ([`View::read_source`]).
    reading: bool,
}

impl Kind {
    /// Same source (a device by id, a library file by plugin and path, a folder by path), whatever its current label or
    /// free space.
    fn same(&self, other: &Kind) -> bool {
        match (self, other) {
            (Kind::Device(a), Kind::Device(b)) => a.id == b.id,
            (
                Kind::Plugin {
                    plugin: a, path: p, ..
                },
                Kind::Plugin {
                    plugin: b, path: q, ..
                },
            ) => a == b && p == q,
            (Kind::Explorer(a), Kind::Explorer(b)) => a == b,
            (Kind::Downloaded { path: a, .. }, Kind::Downloaded { path: b, .. }) => a == b,
            _ => false,
        }
    }

    fn name(&self) -> String {
        match self {
            Kind::Device(d) => format!("{}  ({}, {} free)", d.label, d.fs, device::size(d.free)),
            _ => self.short(),
        }
    }

    /// The name in the loading line and toasts.
    fn short(&self) -> String {
        match self {
            Kind::Device(d) => d.label.clone(),
            Kind::Plugin { label, .. } => label.clone(),
            Kind::Explorer(_) => "Explorer".to_owned(),
            Kind::Downloaded { label, .. } => label.clone(),
        }
    }

    /// What reading the source needs, opened.
    fn open(&self) -> Result<Box<dyn Library + Send>, String> {
        match self {
            Kind::Device(d) => device::open_library(d),
            Kind::Plugin { plugin, path, .. } => {
                Ok(Box::new(plugin::library::open_with(plugin, path)?))
            }
            Kind::Explorer(p) => Ok(Box::new(Folder::open(p)?)),
            Kind::Downloaded { path, .. } => Ok(Box::new(Folder::open(path)?)),
        }
    }
}

/// What a worker read: the library, its playlists, its track count and the tracks the Collection does not have.
type Read = Result<(Box<dyn Library + Send>, Vec<Playlist>, usize, Vec<Track>), String>;
/// The sources the settings and the mounted devices call for now; those already in `old` keep their opened library (and
/// get their label refreshed).
pub fn collect(old: Vec<Source>, media_explorer: bool) -> Vec<Source> {
    let mut kinds: Vec<Kind> = Vec::new();
    // What the plugin sources downloaded: their folders in the data dir, first in the tree (the "Downloaded" section).
    kinds.extend(downloaded());
    kinds.extend(
        device::scan()
            .into_iter()
            .filter(Device::has_library)
            .map(Kind::Device),
    );
    kinds.extend(
        plugin::library::sources()
            .into_iter()
            .map(|s| Kind::Plugin {
                plugin: s.plugin,
                label: s.label,
                path: s.path,
            }),
    );
    // `$XDG_MUSIC_DIR`, else `~/Music`, else home: the whole home holds caches and build trees, read for minutes.
    let home = openfader::paths::home();
    let music = std::env::var_os("XDG_MUSIC_DIR")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join("Music")))
        .filter(|p| p.is_dir());
    if let (true, Some(p)) = (media_explorer, music.or(home).filter(|p| p.is_dir())) {
        kinds.push(Kind::Explorer(p));
    }
    let mut old: Vec<Option<Source>> = old.into_iter().map(Some).collect();
    kinds
        .into_iter()
        .map(|kind| {
            let kept = old
                .iter_mut()
                .find(|o| o.as_ref().is_some_and(|o| o.kind.same(&kind)))
                .and_then(Option::take);
            match kept {
                Some(o) => Source {
                    name: kind.name(),
                    kind,
                    ..o
                },
                None => Source {
                    name: kind.name(),
                    kind,
                    lib: None,
                    playlists: Vec::new(),
                    missing: Vec::new(),
                    reading: false,
                },
            }
        })
        .collect()
}

impl Source {
    pub fn is_device(&self) -> bool {
        matches!(self.kind, Kind::Device(_))
    }

    /// Whether the source is the folder a plugin source downloaded into (the "Downloaded" section).
    pub fn is_downloaded(&self) -> bool {
        matches!(self.kind, Kind::Downloaded { .. })
    }

    /// The path the source is read from again on a worker thread (`None` for a device: its own menus import from it).
    pub fn path(&self) -> Option<(PathBuf, SourceType)> {
        match &self.kind {
            Kind::Device(_) => None,
            Kind::Plugin { plugin, path, .. } => {
                Some((path.clone(), SourceType::Plugin(plugin.clone())))
            }
            Kind::Explorer(p) => Some((p.clone(), SourceType::Folder)),
            Kind::Downloaded {
                plugin,
                source,
                path,
                ..
            } => Some((
                path.clone(),
                SourceType::Downloaded {
                    plugin: plugin.clone(),
                    source: source.clone(),
                },
            )),
        }
    }
}

/// The folders where the plugin sources keep the audio they downloaded: `<data dir>/plugins/<plugin>/<source>`, when
/// the folder is there and holds audio. What is in them is a real file: it can be loaded, imported and searched.
fn downloaded() -> Vec<Kind> {
    let Some(base) = openfader::paths::data_dir() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for p in plugin::registry().plugins.iter() {
        for s in p.manifest.sources.iter() {
            let dir = base.join("plugins").join(&p.manifest.name).join(&s.id);
            if dir.is_dir() && holds_audio(&dir) {
                out.push(Kind::Downloaded {
                    plugin: p.manifest.name.clone(),
                    source: s.id.clone(),
                    label: s.name.clone(),
                    path: dir,
                });
            }
        }
    }
    out
}

/// Whether the folder holds audio (or a video) below it: a plugin that downloaded nothing leaves an empty folder.
fn holds_audio(dir: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.path()
            .is_file()
            .then(|| library::folder::is_audio(&e.path()))
            .unwrap_or(false)
    })
}

/// The tracks of the source the Collection does not have: the files it lists that the Collection does not list yet. A
/// downloaded file is the Collection's own cloud track (its name is the remote id), so it counts as in it.
fn missing_of(kind: &Kind, tracks: &[Track]) -> Vec<Track> {
    let store = store::global();
    let (Ok(files), Ok(remotes)) = (store.collection_files(), store.collection_remotes()) else {
        return Vec::new();
    };
    // A downloaded file is named after the remote id: the cloud tracks of the Collection name theirs the same way.
    let names: Vec<String> = match kind {
        Kind::Downloaded { plugin, source, .. } => remotes
            .iter()
            .filter(|(p, s, _)| p == plugin && s == source)
            .map(|(_, _, id)| plugin::source::file_name(id))
            .collect(),
        _ => Vec::new(),
    };
    tracks
        .iter()
        .filter(|t| match &t.file {
            Some(f) => {
                !files.contains(f)
                    && !names
                        .iter()
                        .any(|n| f.file_stem().is_some_and(|s| s == n.as_str()))
            }
            None => true,
        })
        .cloned()
        .collect()
}

/// How to read a [`Source::path`] again.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceType {
    /// With the library plugin of that name.
    Plugin(String),
    Folder,
    /// The download folder of a plugin source: the files the Collection already holds as cloud tracks are left out.
    Downloaded { plugin: String, source: String },
}

impl View {
    /// The tree's view of the sources.
    pub(super) fn source_rows(&self) -> Vec<SourceRows> {
        self.sources
            .iter()
            .map(|s| SourceRows {
                device: s.is_device(),
                downloaded: s.is_downloaded(),
                name: s.name.clone(),
                open: s.lib.is_some(),
                playlists: s.playlists.clone(),
                missing: s.missing.clone(),
            })
            .collect()
    }

    /// `BrowserState.side-kind` of a source row: 10 nothing to do, 11 a list of iTunes / xml / Explorer (IMPORT TO COLLECTION),
    /// 12 a list of a device (import / delete through the device window), 13 the tracks a source does not have (IMPORT ALL),
    /// 14 a download folder (IMPORT TO COLLECTION, but no CONNECT: the app's own cache is not synced).
    pub(super) fn source_side_kind(&self, e: &Entry) -> Option<i32> {
        let device = |i: &usize| self.sources.get(*i).is_some_and(Source::is_device);
        let downloaded = |i: &usize| self.sources.get(*i).is_some_and(Source::is_downloaded);
        Some(match e {
            Entry::DevicesRoot | Entry::DownloadedRoot | Entry::Source(_) => 10,
            Entry::SourceAll(i) => {
                if device(i) {
                    10
                } else if downloaded(i) {
                    14
                } else {
                    11
                }
            }
            Entry::SourceMissing(i) => {
                // The tracks the Collection does not have: the import button adds them all.
                if device(i) {
                    10
                } else {
                    13
                }
            }
            Entry::SourceList(i, j)
                if self
                    .sources
                    .get(*i)
                    .and_then(|s| s.playlists.get(*j))
                    .is_some_and(|p| !p.is_folder) =>
            {
                if device(i) {
                    12
                } else {
                    11
                }
            }
            Entry::SourceList(..) => 10,
            _ => return None,
        })
    }

    pub(super) fn source_home(&self, e: &Entry) -> Option<Home> {
        Some(match e {
            Entry::PlaylistsAll => Home::Playlist(None),
            Entry::SourceAll(i) => Home::Source(*i, None),
            Entry::SourceList(i, j) => match self.sources.get(*i).and_then(|s| s.playlists.get(*j))
            {
                Some(p) if !p.is_folder => Home::Source(*i, Some(p.id.clone())),
                _ => Home::Nothing,
            },
            Entry::Source(i) => {
                // A downloaded folder opens its tracks; a device opens its own window and its lists are below it.
                if self.sources.get(*i).is_some_and(Source::is_downloaded) {
                    Home::Source(*i, None)
                } else {
                    Home::Nothing
                }
            }
            Entry::SourceMissing(i) => Home::SourceMissing(*i),
            Entry::DevicesRoot | Entry::DownloadedRoot => Home::Nothing,
            _ => return None,
        })
    }

    pub(super) fn source_hint(&self, e: &Entry) -> Option<&'static str> {
        Some(match e {
            Entry::DownloadedRoot => {
                "What the plugin sources downloaded: choose a folder to open it (the files are real ones: they can be loaded, imported and searched)"
            }
            Entry::DevicesRoot => {
                "Devices: choose a device to open it and its window (eject, sync, backup, update collection …)"
            }
            Entry::Source(_) => {
                "Choose All tracks or a playlist; IMPORT TO COLLECTION (File and Track menus) adds what you choose to the Collection"
            }
            Entry::SourceList(..) => "Folder of the source: choose a playlist inside it",
            _ => return None,
        })
    }

    /// The library plugin / Explorer list on screen, for IMPORT TO COLLECTION.
    pub(super) fn source_sel(&self) -> Option<(PathBuf, SourceType, Option<String>)> {
        let (i, list) = match &self.entry {
            Entry::SourceAll(i) => (*i, None),
            Entry::SourceList(i, j) => (
                *i,
                Some(self.sources.get(*i)?.playlists.get(*j)?.id.clone()),
            ),
            _ => return None,
        };
        let (path, kind) = self.sources.get(i)?.path()?;
        Some((path, kind, list))
    }

    /// The tracks of the "Not in the Collection" list on screen, for IMPORT ALL.
    pub(super) fn missing_sel(&self) -> Vec<Track> {
        match &self.entry {
            Entry::SourceMissing(i) => self
                .sources
                .get(*i)
                .map(|s| s.missing.clone())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// The chosen device's id and playlist, for the strip's device buttons.
    pub(super) fn publish_source(&self, app: &App) {
        let (dev, list) = match &self.entry {
            Entry::SourceList(i, j) => match self.sources.get(*i) {
                Some(Source {
                    kind: Kind::Device(d),
                    playlists,
                    ..
                }) => (
                    d.id.clone(),
                    playlists.get(*j).map(|p| p.id.clone()).unwrap_or_default(),
                ),
                _ => (String::new(), String::new()),
            },
            _ => (String::new(), String::new()),
        };
        let s = app.global::<BrowserState>();
        s.set_source_history(library::is_history(&list));
        s.set_source_device(dev.into());
        s.set_source_list(list.into());
    }

    /// Source row `i` was chosen: read it (once) and list what is in it; a device also opens its window.
    pub(super) fn open_source(&mut self, app: &App, i: usize) {
        self.read_source(app, i);
        if let Some(Source {
            kind: Kind::Device(d),
            ..
        }) = self.sources.get(i)
        {
            crate::app::device::show_panel(app, &d.id);
        }
    }

    /// Starts reading source `i` on a worker unless it is read or being read ([`read`]).
    fn read_source(&mut self, app: &App, i: usize) {
        let Some(s) = self
            .sources
            .get_mut(i)
            .filter(|s| s.lib.is_none() && !s.reading)
        else {
            return;
        };
        s.reading = true;
        loading(app, &format!("{}: reading…", s.kind.short()), Some(0.0));
        read(app, s.kind.clone());
    }

    /// Source `kind` was read ([`read`]): list it, say so, and show it when its row is still chosen.
    fn source_read(&mut self, app: &App, kind: Kind, read: Read, secs: f32) {
        loading(app, "", None);
        let Some(i) = self.sources.iter().position(|s| s.kind.same(&kind)) else {
            return; // unplugged or switched off meanwhile
        };
        let s = &mut self.sources[i];
        s.reading = false;
        let text = match read {
            Ok((lib, playlists, tracks, missing)) => {
                let lists = playlists.iter().filter(|p| !p.is_folder).count();
                let lib: Rc<dyn Library> = Rc::<dyn Library + Send>::from(lib);
                (s.lib, s.playlists, s.missing) = (Some(lib), playlists, missing);
                let s_ = |n: usize| if n == 1 { "" } else { "s" };
                format!(
                    "{}: {tracks} track{}, {lists} playlist{} ready ({secs:.1} s)",
                    kind.short(),
                    s_(tracks),
                    s_(lists),
                )
            }
            Err(e) => format!("{}: cannot read it: {e}", kind.short()),
        };
        eprintln!("{text}");
        toast::show(app, &text);
        self.say(app, text);
        self.rebuild(app);
        if matches!(self.entry, Entry::Source(j) | Entry::SourceAll(j) | Entry::SourceList(j, _) if j == i)
        {
            self.show(app);
        }
    }

    /// Re-reads what is mounted and what the Media Browser settings enable; rebuilds the tree when that changed. The chosen
    /// row follows its source when the indexes shift, and falls back to All tracks when its source is gone.
    pub(super) fn refresh_sources(&mut self, app: &App) {
        let explorer = prefs::with(|p| p.view.media_explorer);
        let chosen = match &self.entry {
            Entry::Source(i) | Entry::SourceAll(i) | Entry::SourceList(i, _) => {
                self.sources.get(*i).map(|s| s.kind.clone())
            }
            _ => None,
        };
        let before: Vec<Kind> = self.sources.iter().map(|s| s.kind.clone()).collect();
        let old = std::mem::take(&mut self.sources);
        self.sources = collect(old, explorer);
        if before.len() == self.sources.len()
            && before
                .iter()
                .zip(&self.sources)
                .all(|(a, s)| a.same(&s.kind))
        {
            return;
        }
        // A device that just appeared with a library is read straight away.
        let fresh: Vec<usize> = (0..self.sources.len())
            .filter(|&i| {
                matches!(self.sources[i].kind, Kind::Device(_))
                    && !before.iter().any(|b| b.same(&self.sources[i].kind))
            })
            .collect();
        for i in fresh {
            self.read_source(app, i);
        }
        let find = || {
            chosen
                .as_ref()
                .and_then(|k| self.sources.iter().position(|s| s.kind.same(k)))
        };
        let was = self.entry.clone();
        self.entry = match &was {
            Entry::Source(_) => find().map_or(Entry::All, Entry::Source),
            Entry::SourceAll(_) => find().map_or(Entry::All, Entry::SourceAll),
            Entry::SourceList(_, j) => find().map_or(Entry::All, |n| Entry::SourceList(n, *j)),
            _ => was.clone(),
        };
        self.rebuild(app);
        if self.entry != was {
            self.show(app);
        }
    }
}

/// Reads `kind` off the UI thread and hands it to the browser ([`View::source_read`]). A device's library files are read
/// through once first with byte progress (the slow part on a USB stick); opening them then comes from the page cache.
fn read(app: &App, kind: Kind) {
    let weak = app.as_weak();
    std::thread::spawn(move || {
        let (start, name) = (Instant::now(), kind.short());
        let step = |text: String, at: f32| {
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = weak.upgrade() {
                    loading(&app, &text, Some(at));
                }
            });
        };
        if let Kind::Device(d) = &kind {
            let mut shown = u64::MAX;
            plug::prefetch(d, |done, total| {
                let pct = done * 100 / total.max(1);
                if pct / 5 != shown / 5 {
                    shown = pct;
                    let (done, total) = (device::size(done), device::size(total));
                    step(
                        format!("{name}: reading the library, {done} of {total}"),
                        0.8 * pct as f32 / 100.0,
                    );
                }
            });
        }
        step(format!("{name}: listing tracks and playlists"), 0.85);
        let read: Read = kind.open().and_then(|lib| {
            let playlists = lib.playlists()?;
            let tracks = lib.tracks(None)?;
            let count = tracks.len();
            let missing = missing_of(&kind, &tracks);
            Ok((lib, playlists, count, missing))
        });
        let secs = start.elapsed().as_secs_f32();
        let _ = slint::invoke_from_event_loop(move || {
            if let (Some(app), Some(view)) = (weak.upgrade(), super::view()) {
                view.borrow_mut().source_read(&app, kind, read, secs);
            }
        });
    });
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_is_the_same_by_id_or_path_not_by_label() {
        let dev = |id: &str, label: &str| Device {
            id: id.into(),
            mount: "/m".into(),
            source: String::new(),
            label: label.into(),
            fs: "vfat".into(),
            kind: device::Kind::Usb,
            support: device::Support::Ok,
            read_only: false,
            total: 0,
            free: 0,
            provider: None,
            info: Default::default(),
            formats: Vec::new(),
        };
        assert!(Kind::Device(dev("u1", "A")).same(&Kind::Device(dev("u1", "B"))));
        assert!(!Kind::Device(dev("u1", "A")).same(&Kind::Device(dev("u2", "A"))));
        let plugin = |name: &str, path: &str| Kind::Plugin {
            plugin: name.into(),
            label: "label".into(),
            path: path.into(),
        };
        assert!(plugin("a", "/a.xml").same(&plugin("a", "/a.xml")));
        assert!(!plugin("a", "/a.xml").same(&plugin("b", "/a.xml")));
        assert!(!plugin("a", "/a.xml").same(&plugin("a", "/b.xml")));
    }
}
