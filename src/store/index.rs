//! The index of the pinned folders: what they hold, read once and kept, so the search finds them without
//! walking the stick again.
//!
//! The index is a table of its own, beside the Collection: the tracks of a pinned folder are **read-only** (the app
//! never edits them) and they are not in the Collection, so they do not show up in the library's lists. Their key
//! is `f<id>` ([`id_key`]), like the Collection's `t<id>`.
//!
//! A row names the volume (its label, not its mount path) and the file's path below it: a stick that changes
//! letter keeps its index.

use super::{R, Store};
use crate::library::Track;
use rusqlite::Row;
use std::collections::HashSet;
use std::path::Path;

/// The key of an indexed track: `f<id>`.
pub fn id_key(id: i64) -> String {
    format!("f{id}")
}

/// A row of the index: the track and where it lives.
#[derive(Clone, Debug, PartialEq)]
pub struct Row_ {
    pub id: i64,
    /// The volume's label.
    pub volume: String,
    /// The file's path below the volume.
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub bpm_x100: i64,
    pub length_s: i64,
    pub key: String,
    /// The file's size and time, to know whether it changed.
    pub size: i64,
    pub mtime: i64,
}

impl Row_ {
    /// The track the row names: the file is on the volume, so the app can play it.
    pub fn track(&self, mount: &Path) -> Track {
        Track {
            id: None,
            file: Some(mount.join(&self.path)),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            genre: self.genre.clone(),
            bpm_x100: self.bpm_x100,
            length_s: self.length_s,
            key: self.key.clone(),
            ..Track::default()
        }
    }
}

const COLS: &str = "volume, path, title, artist, album, genre, bpm_x100, length_s, key, size, mtime";

fn from_row(r: &Row) -> rusqlite::Result<Row_> {
    Ok(Row_ {
        id: r.get(0)?,
        volume: r.get(1)?,
        path: r.get(2)?,
        title: r.get(3)?,
        artist: r.get(4)?,
        album: r.get(5)?,
        genre: r.get(6)?,
        bpm_x100: r.get(7)?,
        length_s: r.get(8)?,
        key: r.get(9)?,
        size: r.get(10)?,
        mtime: r.get(11)?,
    })
}

impl Store {
    /// The index of the volume `volume` (its label): the rows it holds.
    pub fn index_rows(&self, volume: &str) -> R<Vec<Row_>> {
        super::query_all(
            &self.db(),
            &format!("SELECT id, {COLS} FROM folder_index WHERE volume = ?1 ORDER BY path"),
            [volume],
            from_row,
        )
    }

    /// Every indexed row, whatever the volume.
    pub fn index_all(&self) -> R<Vec<Row_>> {
        super::query_all(
            &self.db(),
            &format!("SELECT id, {COLS} FROM folder_index ORDER BY volume, path"),
            [],
            from_row,
        )
    }

    /// Replaces the index of the volume `volume` with `rows` (one transaction): the index always says what the
    /// folder holds now.
    pub fn index_put(&self, volume: &str, rows: &[Row_]) -> R<usize> {
        self.write(|tx| {
            super::exec(tx, "DELETE FROM folder_index WHERE volume = ?1", [volume])?;
            for r in rows {
                super::exec(
                    tx,
                    &format!(
                        "INSERT INTO folder_index({COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"
                    ),
                    rusqlite::params![
                        &r.volume,
                        &r.path,
                        &r.title,
                        &r.artist,
                        &r.album,
                        &r.genre,
                        r.bpm_x100,
                        r.length_s,
                        &r.key,
                        r.size,
                        r.mtime
                    ],
                )?;
            }
            Ok(rows.len())
        })
    }

    /// The indexed rows whose title, artist or album holds `q` (case-insensitive).
    pub fn index_search(&self, q: &str) -> R<Vec<Row_>> {
        let like = format!("%{}%", q.to_lowercase());
        super::query_all(
            &self.db(),
            &format!(
                "SELECT id, {COLS} FROM folder_index WHERE lower(title) LIKE ?1 OR lower(artist) LIKE ?1 \
                 OR lower(album) LIKE ?1 ORDER BY volume, path"
            ),
            [&like],
            from_row,
        )
    }

    /// The rows of the index that are not there any more (the file is gone or changed).
    pub fn index_stale(&self, volume: &str) -> R<HashSet<String>> {
        let rows: Vec<(String, i64, i64)> = super::query_all(
            &self.db(),
            "SELECT path, size, mtime FROM folder_index WHERE volume = ?1",
            [volume],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok(rows.into_iter().map(|(p, _, _)| p).collect())
    }
}
