//! Turning the source list into the visible table rows: loading the list of the chosen [`Home`], the search / filter /
//! sort pass, and the cheaper row updates (a new reference, an edited track).

use super::{
    filter,
    table::{self, Facts, Look, Reference},
    App, BrowserState, Home, View,
};
use openfader::{
    device,
    library::{Library, Playlist, Track, folder},
    store,
};
use slint::{ComponentHandle, Model};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

/// Whether the track's file lies on one of the device / local volume roots (case-insensitive on Windows drive letters).
fn on_device(t: &Track, roots: &[PathBuf]) -> bool {
    let Some(file) = t.file.as_deref() else {
        return false;
    };
    roots.iter().any(|r| {
        #[cfg(windows)]
        let hit = file
            .to_string_lossy()
            .to_lowercase()
            .starts_with(&r.to_string_lossy().to_lowercase());
        #[cfg(not(windows))]
        let hit = file.starts_with(r);
        hit
    })
}

impl View {
    /// Rebuilds `list` from the current source and refreshes the rows.
    // ponytail: the whole-library list is cloned out of the cache per view change; share it (Arc) if that shows up in profiles.
    pub(super) fn load(&mut self, app: &App) {
        let (home, anchor) = (self.home.clone(), self.related.clone());
        self.slots.clear();
        let state = app.global::<BrowserState>();
        let deep_on = state.get_subfolders();
        let list = match (&anchor, &home) {
            (Some(a), _) => self.cache().map(|all| {
                let partners = self.store.matches_of(&store::key(a)).unwrap_or_default();
                filter::related_with(a, &all.tracks, &partners, &Look::now())
                    .into_iter()
                    .cloned()
                    .collect()
            }),
            (None, Home::Nothing) => Ok(Vec::new()),
            // The search results are already in the list (the search put them there).
            (None, Home::Search) => Ok(self.list.clone()),
            (None, Home::Tags) => Ok(self.tags.clone()),
            (None, Home::History) => Ok(self.history.clone()),
            (None, Home::Playlist(None)) => self.cache().map(|c| c.tracks.to_vec()),
            (None, Home::Playlist(id)) => self.lib.tracks(id.as_deref()),
            (None, Home::User(id)) => self.user_list(*id),
            (None, Home::Smart(id)) => self.smart_list(*id),
            (None, Home::Day(day)) => self.day_list(day),
            (None, Home::Tag(id)) => self.mytag_list(*id),
            (None, Home::Dupes) => self.dupes_list(),
            (None, Home::Bank(id)) => self.bank_list(*id),
            (None, Home::CueAnalysis) => self.cue_analysis_list(),
            (None, Home::Source(s, id)) => self
                .sources
                .get(*s)
                .and_then(|s| s.lib.clone())
                .ok_or_else(|| "the source is not open".to_owned())
                .and_then(|l| l.tracks(id.as_deref())),
            (None, Home::SourceMissing(s)) => self
                .sources
                .get(*s)
                .map(|s| s.missing.clone())
                .ok_or_else(|| "the source is not open".to_owned()),
            (None, Home::DeviceFolder(d, j)) if deep_on => {
                let key = (*d, *j, String::new());
                match self.deep.get(&key).cloned() {
                    Some(l) => l.tracks(None),
                    None => {
                        self.open_deep(app, *d, *j, String::new());
                        Ok(Vec::new())
                    }
                }
            }
            (None, Home::DeviceFolder(d, j)) => self
                .sources
                .get(*d)
                .and_then(|s| s.folder_libs.get(*j))
                .and_then(|l| l.clone())
                .ok_or_else(|| "the folder is not open".to_owned())
                .and_then(|l| l.tracks(None)),
            (None, Home::DeviceFolderList(d, j, k)) => {
                let id = self
                    .sources
                    .get(*d)
                    .and_then(|s| s.folders.get(*j))
                    .and_then(|f| f.playlists.get(*k))
                    .map(|p| p.id.clone());
                match id {
                    Some(id) if deep_on => {
                        let key = (*d, *j, id.clone());
                        match self.deep.get(&key).cloned() {
                            Some(l) => l.tracks(None),
                            None => {
                                self.open_deep(app, *d, *j, id);
                                Ok(Vec::new())
                            }
                        }
                    }
                    Some(id) => match self.levels.get(&(*d, *j, id.clone())).cloned() {
                        // A level already listed shows its own files (only those: [`Self::open_level`] lists one).
                        Some(l) => l.tracks(None),
                        None => {
                            self.open_level(app, *d, *j, id);
                            Ok(Vec::new())
                        }
                    },
                    None => Err("the folder is not open".to_owned()),
                }
            }
            // A cloud plugin's search: open its own search box; here we just say so.
            (None, Home::PluginSearch(_s, _)) => {
                state.set_status("use the plugin's search box to look up tracks".into());
                Ok(Vec::new())
            }
            // A cloud plugin's downloads: list what it downloaded (the Downloaded section already shows them all).
            (None, Home::PluginDownloads(s, _)) => {
                let plugin_name = self.sources.get(*s).map(|src| src.name.clone()).unwrap_or_default();
                self.sources
                    .iter()
                    .find(|src| {
                        matches!(src.kind, super::sources::Kind::Downloaded { ref plugin, .. } if plugin == &plugin_name)
                    })
                    .map(|src| src.missing.clone())
                    .ok_or_else(|| "the plugin's downloads are not open".to_owned())
            }
            // A cloud plugin's example playlist: list its tracks if they were read.
            (None, Home::PluginPlaylist(s, j)) => self
                .sources
                .get(*s)
                .and_then(|src| src.playlists.get(*j))
                .map(|p| p.id.clone())
                .ok_or_else(|| "the playlist is not open".to_owned())
                .and_then(|id| self.lib.tracks(Some(&id))),
        };
        let plain = anchor.is_none();
        let editing = plain && !self.suggest.on; // a ranked list is no longer in the order its edit buttons act on
        state.set_related(!plain);
        state.set_tag_view(editing && matches!(home, Home::Tags));
        state.set_user_view(editing && matches!(home, Home::User(_) | Home::Bank(_)));
        state.set_history_view(editing && matches!(home, Home::History | Home::Day(_)));
        state.set_remote_search(self.search_entry().is_some());
        match list {
            Ok(list) => {
                self.list = list;
                self.rank_list(app);
                self.refresh(app);
            }
            Err(e) => state.set_status(format!("query failed: {e}").into()),
        }
    }

    /// Lists the sub-folder `id` of the device folder `(d, j)` on a worker ([`folder::Folder::open_level`]: its own
    /// files and its immediate sub-folders), then shows it through [`View::level_read`]. The table stays empty until
    /// it lands; a second click on the same folder while one is reading does not start another walk.
    fn open_level(&mut self, app: &App, d: usize, j: usize, id: String) {
        let key = (d, j, id.clone());
        if !self.levels_reading.insert(key.clone()) {
            return;
        }
        let Some(path) = self
            .sources
            .get(d)
            .and_then(|s| s.folders.get(j))
            .map(|f| f.path.join(&id))
        else {
            self.levels_reading.remove(&key);
            return;
        };
        let weak = app.as_weak();
        std::thread::spawn(move || {
            let listed = folder::Folder::open_level(&path)
                .map(|lib| Box::new(lib) as Box<dyn Library + Send>)
                .map_err(|e| format!("{}: {e}", path.display()));
            if let Err(e) = &listed {
                eprintln!("sources: {e}");
            }
            let _ = slint::invoke_from_event_loop(move || {
                if let (Some(app), Some(view)) = (weak.upgrade(), super::view()) {
                    view.borrow_mut().level_read(&app, d, j, id, listed);
                }
            });
        });
    }

    /// A sub-folder level listed by [`View::open_level`]: its folders join the tree under their parent and, if the
    /// level is the one on screen, its files replace the empty table.
    fn level_read(
        &mut self,
        app: &App,
        d: usize,
        j: usize,
        id: String,
        listed: Result<Box<dyn Library + Send>, String>,
    ) {
        self.levels_reading.remove(&(d, j, id.clone()));
        let lib: Rc<dyn Library> = match listed {
            Ok(lib) => Rc::<dyn Library + Send>::from(lib),
            Err(e) => {
                app.global::<BrowserState>()
                    .set_status(format!("query failed: {e}").into());
                return;
            }
        };
        if let Ok(playlists) = lib.playlists() {
            if let Some(folder) = self.sources.get_mut(d).and_then(|s| s.folders.get_mut(j)) {
                // The parent now has sub-folders: the tree marks it and lists them after its own subtree, prefixed
                // with its id so a deeper level opens the right path. A level already read is in `levels`.
                if let Some(at) = folder.playlists.iter().position(|p| p.id == id) {
                    let depth = folder.playlists[at].depth;
                    folder.playlists[at].is_folder = true;
                    let mut end = at + 1;
                    while folder.playlists.get(end).is_some_and(|p| p.depth > depth) {
                        end += 1;
                    }
                    let fresh: Vec<Playlist> = playlists
                        .iter()
                        .map(|p| Playlist {
                            id: format!("{id}/{}", p.id),
                            name: p.name.clone(),
                            depth: depth + 1 + p.depth,
                            is_folder: p.is_folder,
                        })
                        .filter(|p| !folder.playlists.iter().any(|q| q.id == p.id))
                        .collect();
                    folder.playlists.splice(end..end, fresh);
                }
            }
        }
        self.levels.insert((d, j, id.clone()), lib);
        self.rebuild(app);
        let showing = match &self.home {
            Home::DeviceFolderList(dd, jj, k) if *dd == d && *jj == j => self
                .sources
                .get(d)
                .and_then(|s| s.folders.get(j))
                .and_then(|f| f.playlists.get(*k))
                .is_some_and(|p| p.id == id),
            _ => false,
        };
        if showing {
            self.load(app);
        }
    }

    /// Lists every file under sub-folder `id` of the device folder `(d, j)` when SUBFOLDERS is on: the
    /// explicit recursive mode ([`folder::Folder::open_partial`], cache-only, no tag reads). It runs on a
    /// worker and the table stays empty until it lands; the deep result stays cached per level.
    fn open_deep(&mut self, app: &App, d: usize, j: usize, id: String) {
        let key = (d, j, id.clone());
        if !self.deep_reading.insert(key.clone()) {
            return;
        }
        let Some(folder) = self
            .sources
            .get(d)
            .and_then(|s| s.folders.get(j))
        else {
            self.deep_reading.remove(&key);
            return;
        };
        let path = if id.is_empty() {
            folder.path.clone()
        } else {
            folder.path.join(&id)
        };
        let name = path
            .file_name()
            .map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        super::loading(
            app,
            &format!("{name}: listing every file under this folder…"),
            Some(0.0),
        );
        let weak = app.as_weak();
        std::thread::spawn(move || {
            let listed = folder::Folder::open_partial(&path)
                .map(|lib| Box::new(lib) as Box<dyn Library + Send>)
                .map_err(|e| format!("{}: {e}", path.display()));
            if let Err(e) = &listed {
                eprintln!("sources: {e}");
            }
            let _ = slint::invoke_from_event_loop(move || {
                if let (Some(app), Some(view)) = (weak.upgrade(), super::view()) {
                    view.borrow_mut().deep_read(&app, d, j, id, listed);
                }
            });
        });
    }

    /// The recursive listing of [`View::open_deep`] landed: keep it and, if the same folder is still on
    /// screen with SUBFOLDERS on, fill the table with its whole subtree.
    fn deep_read(
        &mut self,
        app: &App,
        d: usize,
        j: usize,
        id: String,
        listed: Result<Box<dyn Library + Send>, String>,
    ) {
        self.deep_reading.remove(&(d, j, id.clone()));
        let lib: Rc<dyn Library> = match listed {
            Ok(lib) => Rc::<dyn Library + Send>::from(lib),
            Err(e) => {
                let state = app.global::<BrowserState>();
                if state.get_subfolders() {
                    super::loading(app, "", None);
                    state.set_status(format!("query failed: {e}").into());
                }
                return;
            }
        };
        self.deep.insert((d, j, id.clone()), lib);
        let state = app.global::<BrowserState>();
        if !state.get_subfolders() {
            return;
        }
        let showing = match &self.home {
            Home::DeviceFolder(dd, jj) => id.is_empty() && *dd == d && *jj == j,
            Home::DeviceFolderList(dd, jj, k) if *dd == d && *jj == j => self
                .sources
                .get(d)
                .and_then(|s| s.folders.get(j))
                .and_then(|f| f.playlists.get(*k))
                .is_some_and(|p| p.id == id),
            _ => false,
        };
        if showing {
            super::loading(app, "", None);
            self.load(app);
        }
    }

    /// The SUBFOLDERS chip changed: reload the folder on screen, one level or its whole subtree.
    pub(super) fn subfolders_changed(&mut self, app: &App) {
        if !app.global::<BrowserState>().get_subfolders() {
            super::loading(app, "", None);
        }
        self.load(app);
    }

    /// Search box, filter bar, tag filter and header sort over `list`, then the rows and the status line.
    ///
    /// BPM and key are resolved once per track ([`Facts`]) and the search compares without lowercasing each field
    /// ([`filter::shown`], [`filter::sort`]).
    pub(super) fn refresh(&mut self, app: &App) {
        let t = Instant::now();
        self.look = Look::now();
        let look = self.look;
        let q = if self.search_entry().is_some() {
            String::new() // the box searches online there: it does not filter the results
        } else {
            self.query.to_lowercase()
        };
        // The "Devices" origin needs the mounted volumes: ask only when that filter is on (the scan probes the OS).
        let device_roots: Vec<PathBuf> = if self.filter.origin == filter::Origin::Devices {
            device::scan()
                .into_iter()
                .map(|d| d.mount)
                .chain(device::local_disks().into_iter().map(|l| l.mount))
                .collect()
        } else {
            Vec::new()
        };
        let facts: Vec<Facts> = self
            .list
            .iter()
            .enumerate()
            .map(|(i, t)| Facts {
                origin: self.origins.get(i).cloned().unwrap_or_default(),
                on_device: on_device(t, &device_roots),
                ..Facts::of(t)
            })
            .collect();
        self.shown = filter::shown(
            &self.list,
            &facts,
            &q,
            &self.filter,
            &self.reference,
            &look,
            |t| super::tagging::passes(&self.tag_map, &self.tag_filter, t),
        );
        if let Some((col, asc)) = self.sort.filter(|_| !self.ordered()) {
            filter::sort(&mut self.shown, &self.list, &facts, col, asc, &look);
        }
        // A row filtered or sorted out stops being picked; the rest keep their track identity.
        self.picked.retain(|i| self.shown.contains(i));
        let rows: Vec<_> = self
            .shown
            .iter()
            .map(|&i| self.table_row(i, &facts[i]))
            .collect();
        self.rows.set_vec(rows);
        self.strips.reset(self.shown.len());
        let state = app.global::<BrowserState>();
        state.set_picked_count(self.picked.len() as i32);
        state.set_selection_mode(self.picked.len() > 1);
        state.set_location(self.location().into());
        state.set_list_title(self.list_name().into());
        state.set_list_detail(self.list_detail().into());
        state.set_folder_view(matches!(
            self.home,
            Home::DeviceFolder(..) | Home::DeviceFolderList(..)
        ));
        // The selection follows the track, not the row index.
        let at = self.selected.as_ref().and_then(|s| self.row_of(s));
        state.set_current_row(at.map_or(-1, |n| n as i32));
        // The search results say where each row comes from.
        state.set_search_view(matches!(self.home, Home::Search));
        let mut status = match (&self.related, &self.home, self.suggest_status()) {
            (Some(a), ..) => format!(
                "{} tracks relate to “{}” (registered partners, then compatible key, BPM ±6%)",
                self.shown.len(),
                a.title
            ),
            (None, Home::Nothing, _) => self.entry_hint().to_owned(),
            (None, _, Some(s)) => s.to_owned(),
            (None, Home::Bank(_), _) => format!(
                "{} of 8 banks, A first · MORE adds a cue from a deck or the chosen track, loads the list onto a deck",
                self.list.len()
            ),
            (None, Home::Dupes, _) => format!(
                "{} tracks share title, artist and length (±1 s) with another one, in {} groups",
                self.list.len(),
                self.dupe_groups
            ),
            (None, ..) => format!("{} of {} tracks", self.shown.len(), self.list.len()),
        };
        if self.filter.ignored(&self.reference) {
            status += " · BPM ±6% / KEY MATCH wait for a loaded deck";
        }
        state.set_status(status.into());
        state.set_tag_filter_count(self.tag_filter.len() as i32);
        // Instrumentation: how long the UI thread takes to turn the list into rows.
        if t.elapsed().as_millis() >= 50 {
            eprintln!(
                "sources: refresh: {} of {} tracks in {} ms (filter, sort, rows)",
                self.shown.len(),
                self.list.len(),
                t.elapsed().as_millis()
            );
        }
    }

    /// "48 tracks · 4 h 52 min" for the rows on screen, "12 of 80 tracks · …" while a filter hides some.
    fn list_detail(&self) -> String {
        let shown = self.shown.len();
        let secs: u64 = self
            .shown
            .iter()
            .filter_map(|&i| self.list.get(i))
            .map(|t| t.length_s.max(0) as u64)
            .sum();
        let count = if shown == self.list.len() {
            format!("{shown} track{}", if shown == 1 { "" } else { "s" })
        } else {
            format!("{shown} of {} tracks", self.list.len())
        };
        if secs == 0 {
            return count;
        }
        let (hours, mins) = (secs / 3600, secs % 3600 / 60);
        if hours > 0 {
            format!("{count} · {hours} h {mins:02} min")
        } else {
            format!("{count} · {mins} min")
        }
    }

    /// A deck decode learned tags for `t` (bitrate, length, BPM, key, sample rate): update every copy the browser
    /// holds, redraw its visible row in place, and let the learned-tag cache keep them (written on a worker).
    pub(super) fn tags_learned(&mut self, t: &Track) {
        let Some(file) = t.file.clone() else { return };
        for copy in self
            .list
            .iter_mut()
            .chain(self.tags.iter_mut())
            .chain(self.history.iter_mut())
            .chain(self.selected.iter_mut())
        {
            if copy.file.as_deref() == Some(file.as_path()) {
                *copy = t.clone();
            }
        }
        if let Some(n) = self.shown.iter().position(|&i| {
            self.list
                .get(i)
                .is_some_and(|t| t.file.as_deref() == Some(file.as_path()))
        }) {
            let i = self.shown[n];
            let facts = Facts {
                origin: self.origins.get(i).cloned().unwrap_or_default(),
                ..Facts::of(&self.list[i])
            };
            let row = self.table_row(i, &facts);
            self.rows.set_row_data(n, row);
        }
        if t.file.is_some() {
            let learned = t.clone();
            std::thread::spawn(move || folder::remember_tags(&learned));
        }
        // A level already listed keeps its rows in memory: drop the ones that contain this file, so going back
        // to them re-opens from the cache with the tag just learned.
        let sources = &self.sources;
        let outside = |d: usize, j: usize, id: &str| {
            sources
                .get(d)
                .and_then(|s| s.folders.get(j))
                .is_none_or(|f| !file.starts_with(f.path.join(id)))
        };
        self.levels.retain(|(d, j, id), _| outside(*d, *j, id));
        self.deep.retain(|(d, j, id), _| outside(*d, *j, id));
    }

    /// Re-marks the visible rows for a new reference: only each row's hint cell changes, in place (the selection stays).
    pub(super) fn remark(&mut self, r: Reference) {
        self.reference = r;
        for (n, &i) in self.shown.iter().enumerate() {
            if let Some(row) = self.rows.row_data(n) {
                table::remark(&row, &self.list[i], &self.reference, &self.look);
            }
        }
    }

    /// The track panel edited a rating / comment: every copy of that track here shows the new values.
    pub(super) fn track_edited(&mut self, app: &App, edited: &Track) {
        let key = store::key(edited);
        let put = |t: &mut Track| {
            if store::key(t) == key {
                (t.rating, t.comment) = (edited.rating, edited.comment.clone());
                (t.title, t.artist, t.album, t.genre, t.label) = (
                    edited.title.clone(),
                    edited.artist.clone(),
                    edited.album.clone(),
                    edited.genre.clone(),
                    edited.label.clone(),
                );
            }
        };
        [&mut self.list, &mut self.tags, &mut self.history]
            .into_iter()
            .flatten()
            .for_each(put);
        self.selected.iter_mut().for_each(put);
        self.cache = None;
        if matches!(self.home, Home::Smart(_)) && self.related.is_none() {
            self.load(app);
        } else {
            self.refresh(app);
        }
    }
}
