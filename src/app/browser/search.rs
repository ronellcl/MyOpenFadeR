//! The unified search: what the search box finds, everywhere.
//!
//! The text of the search box looks in the **Collection** (every row, also the ones of the folders that are open),
//! in the **index of the pinned folders** and in the **local sources that are read** (the download folders, the
//! device folders). It never asks a plugin: the online searches of YouTube and Deezer stay in their own box, and
//! typing here never starts one.
//!
//! The results are a list of their own, with the **origin** of each row (where it came from), and a row whose
//! volume is not mounted any more says so.

use super::{App, Home, View};
use openfader::store::index::Row_;
use openfader::device;
use openfader::library::Track;
use std::path::PathBuf;

/// What a result comes from.
pub const COLLECTION: &str = "Collection";

/// Whether the track's text holds `q` (case-insensitive): the title, the artist, the album, the genre or the comment.
pub fn matches(t: &Track, q: &str) -> bool {
    let q = q.to_lowercase();
    [&t.title, &t.artist, &t.album, &t.genre, &t.comment]
        .iter()
        .any(|f| f.to_lowercase().contains(&q))
}

/// The volume of a label, when it is mounted now.
fn mount_of(label: &str) -> Option<PathBuf> {
    device::scan()
        .into_iter()
        .find(|d| d.label == label)
        .map(|d| d.mount)
}

impl View {
    /// Searches everything local and shows the results. The online searches of the plugins are never started.
    pub(super) fn search_all(&mut self, app: &App) {
        let q = self.query.trim().to_owned();
        // In a plugin's own search box the box belongs to the plugin: the filter does not take it.
        if self.search_entry().is_some() {
            self.refresh(app);
            return;
        }
        if q.is_empty() {
            self.home = Home::Playlist(None);
            self.origins.clear();
            self.load(app);
            return;
        }
        let mut list: Vec<Track> = Vec::new();
        let mut origins: Vec<String> = Vec::new();
        // The Collection: every row of it, not only the ones the list on screen shows.
        if let Ok(cache) = self.cache() {
            for t in cache.tracks.iter() {
                if matches(t, &q) {
                    list.push(t.clone());
                    origins.push(COLLECTION.to_owned());
                }
            }
        }
        // The index of the pinned folders.
        for row in self.store.index_search(&q).unwrap_or_default() {
            list.push(indexed(&row));
            origins.push(origin_of(&row));
        }
        // The local sources that are read: the download folders and the pinned folders (the device libraries are
        // big and their tracks are in the Collection).
        for s in &self.sources {
            if s.is_device() {
                continue;
            }
            let Some(lib) = s.lib.clone() else { continue };
            let Ok(tracks) = lib.tracks(None) else { continue };
            for t in tracks {
                if matches(&t, &q) {
                    list.push(t);
                    origins.push(s.name.clone());
                }
            }
        }
        self.home = Home::Search;
        self.list = list;
        self.origins = origins;
        self.refresh(app);
    }
}

/// The track an indexed row names: the file is on the volume, if it is still there.
fn indexed(row: &Row_) -> Track {
    match mount_of(&row.volume) {
        Some(mount) => row.track(&mount),
        None => Track {
            title: row.title.clone(),
            artist: row.artist.clone(),
            album: row.album.clone(),
            genre: row.genre.clone(),
            bpm_x100: row.bpm_x100,
            length_s: row.length_s,
            key: row.key.clone(),
            ..Track::default()
        },
    }
}

/// Where an indexed row comes from: the volume and the folder, or that the volume is not there.
fn origin_of(row: &Row_) -> String {
    if mount_of(&row.volume).is_some() {
        format!("{}/{}", row.volume, row.path)
    } else {
        format!("{} (device disconnected)", row.volume)
    }
}
