//! Backup and restore of our library data (File > Library > Backup Library / Restore Library).
//!
//! A backup is a zip next to which an optional `MUSIC_DIR` folder holds the music files ("back up music files as well?", zip
//! named with the date, music in `MUSIC_DIR`). Our zip holds `store.db` (a consistent copy made by
//! `VACUUM INTO`: collection, playlists, cues, grids, ratings, tags, history, devices …), `store.id` (the
//! `MOFD` identity), `settings.conf` (the settings file) and `music.tsv` (original path, tab, path relative to
//! `MUSIC_DIR`). Restore refuses a backup from another OpenFadeR build before attaching its database, copies every
//! table of a MyOpenFadeR backup over ours in one transaction (the backup must not be from a newer build nor lack a
//! table this build needs), keeps this computer's local tables, and puts music back where it was when nothing is there.
//!
//! The zip is written and read by [`crate::zip`], entries stored uncompressed.
//! ponytail: no deflate on write (the entries are one SQLite file and two text files; `flate2` is already a dependency if backups
//! get large) and no zip64.

use super::{identity, in_tx, Sql, Store, MIGRATIONS, R};
use crate::paths::at;
use crate::zip::{self, Zip};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Folder (beside the zip) holding the backed-up music.
pub const MUSIC_DIR: &str = "music-backup";

const SETTINGS: &str = "settings.conf";
const STORE: &str = "store.db";
const STORE_ID: &str = "store.id";
const MUSIC_INDEX: &str = "music.tsv";

/// Tables that belong to this computer and survive a restore: the pinned-folder index is rebuilt from
/// the sticks it names and a note describes this installation, so neither is the backup's to replace.
const LOCAL_TABLES: &[&str] = &["folder_index", "store_note"];

/// A zip error in the backup's wording.
fn zip_error(e: zip::Error) -> String {
    match e {
        zip::Error::NotZip => "not a MyOpenFadeR backup zip".into(),
        zip::Error::Crc(name) => format!("{name}: checksum mismatch, the backup is damaged"),
        zip::Error::Other(m) => m,
    }
}

/// What [`Store::backup`] made.
#[derive(Debug)]
pub struct Backup {
    pub zip: PathBuf,
    /// Music files copied into [`MUSIC_DIR`] / that could not be read.
    pub music: usize,
    pub music_failed: usize,
}

/// What [`Store::restore`] found in the zip.
#[derive(Debug)]
pub struct Restored {
    /// The backed-up settings file, for the caller to put in place.
    pub settings: Option<Vec<u8>>,
    /// Music files copied back to their original place / that were already there / that the backup does not hold.
    pub music: usize,
    pub music_present: usize,
    pub music_lost: usize,
    /// Rows the restored data references but does not hold. Zero for a healthy backup; any other value is
    /// reported instead of hidden, because the rows are already in place.
    pub foreign_key_violations: usize,
}

impl Store {
    /// Writes `<dir>/MyOpenFadeR-backup-<stamp>.zip` (never over an existing file) with this store and `settings` (the settings
    /// file, when it exists); `music` files are copied to `<dir>/MUSIC_DIR/<n>/<name>` and listed in the zip.
    pub fn backup(
        &self,
        dir: &Path,
        settings: Option<&Path>,
        music: &[PathBuf],
        stamp: &str,
    ) -> R<Backup> {
        at(std::fs::create_dir_all(dir), dir)?;
        let mut zip = dir.join(format!("MyOpenFadeR-backup-{stamp}.zip"));
        for n in 2.. {
            if !zip.exists() {
                break;
            }
            zip = dir.join(format!("MyOpenFadeR-backup-{stamp}-{n}.zip"));
        }
        let snapshot = dir.join(format!(".store-{}.tmp", std::process::id()));
        let _ = std::fs::remove_file(&snapshot);
        // A store opened before the marker existed gets it here too, so the snapshot always carries it.
        identity::stamp(&self.db())?;
        self.run("VACUUM INTO ?1", [snapshot.to_string_lossy()])?;
        let db = at(std::fs::read(&snapshot), &snapshot);
        let _ = std::fs::remove_file(&snapshot);
        let mut entries = vec![(STORE, db?), (STORE_ID, identity::BACKUP_ID.to_vec())];
        if let Some(s) = settings.filter(|s| s.is_file()) {
            entries.push((SETTINGS, at(std::fs::read(s), s)?));
        }
        let (mut index, mut copied, mut failed) = (String::new(), 0, 0);
        for (i, f) in music.iter().enumerate() {
            let rel = format!(
                "{i}/{}",
                f.file_name()
                    .map_or_else(|| "file".into(), |n| n.to_string_lossy())
            );
            let to = dir.join(MUSIC_DIR).join(&rel);
            let ok = to
                .parent()
                .is_some_and(|p| std::fs::create_dir_all(p).is_ok())
                && std::fs::copy(f, &to).is_ok();
            if ok {
                index += &format!("{}\t{rel}\n", f.display());
                copied += 1;
            } else {
                failed += 1;
            }
        }
        entries.push((MUSIC_INDEX, index.into_bytes()));
        at(std::fs::write(&zip, zip::write(&entries)), &zip)?;
        Ok(Backup {
            zip,
            music: copied,
            music_failed: failed,
        })
    }

    /// Replaces our data with the backup in `zip`; music is copied back from `MUSIC_DIR` beside the zip where the original
    /// file is gone. The settings file is returned, not written.
    pub fn restore(&self, zip: &Path) -> R<Restored> {
        let mut z = Zip::open(zip).map_err(zip_error)?;
        let mut get = |n: &str| z.has(n).then(|| z.read(n).map_err(zip_error)).transpose();
        let db = get(STORE)?
            .ok_or("this zip holds no store.db (it is not a backup made by this program)")?;
        let id = get(STORE_ID)?;
        if let Some(id) = id {
            identity::check_backup_id(&id)?;
        }
        let (settings, index) = (get(SETTINGS)?, get(MUSIC_INDEX)?);
        // One file per restore: two stores restoring at once (the tests do) never share it.
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temp =
            std::env::temp_dir().join(format!("openfader-restore-{}-{n}.db", std::process::id()));
        at(std::fs::write(&temp, db), &temp)?;
        let r = self.copy_tables_from(&temp);
        let _ = std::fs::remove_file(&temp);
        r?;
        let mut out = Restored {
            settings,
            music: 0,
            music_present: 0,
            music_lost: 0,
            foreign_key_violations: 0,
        };
        let base = zip.parent().unwrap_or(Path::new("")).join(MUSIC_DIR);
        let index = index
            .as_deref()
            .map(String::from_utf8_lossy)
            .unwrap_or_default();
        for line in index.lines() {
            let Some((orig, rel)) = line.split_once('\t') else {
                continue;
            };
            let (orig, from) = (Path::new(orig), base.join(rel));
            if orig.exists() {
                out.music_present += 1;
            } else if from.is_file()
                && orig
                    .parent()
                    .is_some_and(|p| std::fs::create_dir_all(p).is_ok())
                && std::fs::copy(&from, orig).is_ok()
            {
                out.music += 1;
            } else {
                out.music_lost += 1;
            }
        }
        // A backup from before the canonical columns holds rows still to be backfilled and edits under legacy keys; the
        // music is back by now, so their files can be read.
        self.system(|| super::rekey::backfill(&mut self.db()))?;
        out.foreign_key_violations = self.foreign_key_violations()?;
        Ok(out)
    }

    /// Rows whose foreign keys point nowhere in this store. Runs after a restore, when the backup has already
    /// replaced the data and the caller still needs to know whether the result is coherent.
    fn foreign_key_violations(&self) -> R<usize> {
        let db = self.db();
        let mut st = db.prepare("PRAGMA foreign_key_check").s()?;
        let mut rows = st.query([]).s()?;
        let mut n = 0;
        while rows.next().s()?.is_some() {
            n += 1;
        }
        Ok(n)
    }

    /// Every table of the database file `other` replaces the same-named table here (matching columns only).
    fn copy_tables_from(&self, other: &Path) -> R<()> {
        let found = identity::check(other)?;
        if !found.is_fork() {
            return Err("the backup's store.db holds no MyOpenFadeR library".into());
        }
        if found.schema > MIGRATIONS.len() {
            return Err(format!(
                "the backup is from a newer build (store schema v{})",
                found.schema
            ));
        }
        let mut db = self.db();
        db.execute("ATTACH DATABASE ?1 AS bak", [other.to_string_lossy()])
            .s()?;
        let done = self.system(|| replace_tables(&mut db));
        let _ = db.execute("DETACH DATABASE bak", []);
        done
    }
}

/// First column of every row of `sql`. Uncached: the statements below name the attached `bak` schema, which is gone
/// after the restore.
fn names(db: &Connection, sql: &str) -> R<Vec<String>> {
    db.prepare(sql)
        .s()?
        .query_map([], |r| r.get(0))
        .s()?
        .collect::<rusqlite::Result<_>>()
        .s()
}

fn tables(db: &Connection, schema: &str) -> R<Vec<String>> {
    names(
        db,
        &format!(
            "SELECT name FROM {schema}.sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
        ),
    )
}

fn columns(db: &Connection, schema: &str, table: &str) -> R<Vec<String>> {
    names(
        db,
        &format!("SELECT name FROM pragma_table_info('{table}', '{schema}')"),
    )
}

/// Empties every table of `main` and refills the ones `bak` also has from it, in one transaction.
fn replace_tables(db: &mut Connection) -> R<()> {
    // The journal stays this store's own: it describes edits of the data being replaced, so it is emptied below.
    // The high-water mark of ids only grows, and the local tables describe this computer, not the backup.
    let mine: Vec<String> = tables(db, "main")?
        .into_iter()
        .filter(|t| {
            !t.starts_with("journal")
                && t != "collection_hwm"
                && !LOCAL_TABLES.contains(&t.as_str())
        })
        .collect();
    let bak = tables(db, "bak")?;
    let shared: Vec<String> = bak.iter().filter(|t| mine.contains(t)).cloned().collect();
    // A same-build store has every table; a missing one would be silently emptied. Older backups may lack
    // tables this build added, and are refused rather than turned into a half-empty store.
    let missing: Vec<&str> = mine
        .iter()
        .map(String::as_str)
        .filter(|t| !bak.iter().any(|b| b == t))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "the backup is missing table(s) this build needs: {}; it was not restored",
            missing.join(", ")
        ));
    }
    in_tx(db, true, |tx| {
        tx.execute("DELETE FROM main.journal", []).s()?;
        tx.execute("DELETE FROM main.journal_action WHERE id > 0", [])
            .s()?;
        for t in &mine {
            tx.execute(&format!("DELETE FROM main.\"{t}\""), []).s()?;
        }
        for t in &shared {
            let (a, b) = (columns(tx, "main", t)?, columns(tx, "bak", t)?);
            let cols = a
                .iter()
                .filter(|c| b.contains(c))
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            if cols.is_empty() {
                continue;
            }
            tx.execute(
                &format!("INSERT INTO main.\"{t}\"({cols}) SELECT {cols} FROM bak.\"{t}\""),
                [],
            )
            .s()?;
        }
        // The journal restarts at seq 1: a cursor into the old one would hide every new edit from write-back.
        tx.execute("UPDATE main.connection SET last_seq = 0", [])
            .s()?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{Remote, Track};
    use crate::store::NodeKind;
    use crate::store::index::Row_;
    use rusqlite::Connection;

    #[test]
    fn backup_then_restore_brings_back_data_settings_and_music() {
        let d = std::env::temp_dir().join(format!("openfader-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("music")).unwrap();
        let (song, conf) = (d.join("music/song.mp3"), d.join("settings.conf"));
        std::fs::write(&song, b"audio").unwrap();
        std::fs::write(&conf, b"key = classic\n").unwrap();

        let s = Store::in_memory().unwrap();
        s.set_rating("a|1", Some(4)).unwrap();
        s.set_gain_adjust("a|1", 1.5).unwrap();
        let pl = s.add_node(None, "Mine", NodeKind::Playlist).unwrap();
        s.add_entries(pl, &["a|1".into()]).unwrap();
        s.set_taglist(&["a|1".into()]).unwrap();
        let b = s
            .backup(
                &d.join("out"),
                Some(&conf),
                std::slice::from_ref(&song),
                "t",
            )
            .unwrap();
        assert_eq!((b.music, b.music_failed), (1, 0));
        assert!(d.join("out").join(MUSIC_DIR).join("0/song.mp3").is_file());
        assert_eq!(
            s.backup(&d.join("out"), None, &[], "t")
                .unwrap()
                .zip
                .file_name()
                .unwrap(),
            "MyOpenFadeR-backup-t-2.zip"
        );

        // The library changes after the backup; the music file is lost.
        s.set_rating("a|1", Some(1)).unwrap();
        s.set_gain_adjust("a|1", -3.0).unwrap();
        s.delete_node(pl).unwrap();
        s.set_rating("late|2", Some(5)).unwrap();
        std::fs::remove_file(&song).unwrap();

        let r = s.restore(&b.zip).unwrap();
        assert_eq!(s.edits("a|1").unwrap().rating, Some(4));
        assert_eq!(s.edits("late|2").unwrap().rating, None);
        assert_eq!(s.gain("a|1").unwrap().unwrap().adjust_db, 1.5); // AUTO GAIN comes back too
        let nodes = s.nodes().unwrap();
        assert_eq!(
            (nodes[0].name.as_str(), s.entries(nodes[0].id).unwrap()),
            ("Mine", vec!["a|1".to_owned()])
        );
        assert_eq!(s.taglist().unwrap(), ["a|1"]);
        assert_eq!(r.settings.as_deref(), Some(&b"key = classic\n"[..]));
        assert_eq!((r.music, r.music_present, r.music_lost), (1, 0, 0));
        assert_eq!(std::fs::read(&song).unwrap(), b"audio");
        // Restoring twice: the music is there now.
        assert_eq!(s.restore(&b.zip).unwrap().music_present, 1);
        assert!(s.restore(&d.join("missing.zip")).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn restore_resets_connection_cursors_with_the_journal() {
        let d = std::env::temp_dir().join(format!("openfader-backup-seq-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = Store::in_memory().unwrap();
        s.connection_add("p", Path::new("/x"), "L", 500).unwrap();
        let b = s.backup(&d, None, &[], "t").unwrap();
        s.restore(&b.zip).unwrap();
        assert_eq!(s.connections().unwrap()[0].last_seq, 0);
        std::fs::remove_dir_all(&d).unwrap();
    }

    fn index_row(path: &str) -> Row_ {
        Row_ {
            id: 0,
            volume: "USB".into(),
            path: path.into(),
            title: "T".into(),
            artist: "A".into(),
            album: String::new(),
            genre: String::new(),
            bpm_x100: 12800,
            length_s: 180,
            key: "1A".into(),
            size: 10,
            mtime: 20,
        }
    }

    #[test]
    fn restore_keeps_the_local_folder_index() {
        let d = std::env::temp_dir().join(format!("openfader-backup-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = Store::in_memory().unwrap();
        s.index_put("USB", &[index_row("backup.mp3")]).unwrap();
        s.set_rating("a|1", Some(5)).unwrap();
        let b = s.backup(&d, None, &[], "t").unwrap();
        s.index_put("USB", &[index_row("current.mp3")]).unwrap();
        s.set_rating("a|1", Some(1)).unwrap();
        let r = s.restore(&b.zip).unwrap();
        assert_eq!(r.foreign_key_violations, 0);
        assert_eq!(s.edits("a|1").unwrap().rating, Some(5));
        let rows = s.index_rows("USB").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "current.mp3");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn restore_keeps_the_cloud_columns() {
        let d = std::env::temp_dir().join(format!("openfader-backup-cloud-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = Store::in_memory().unwrap();
        let cloud = Track {
            title: "Cloud".into(),
            artist: "A".into(),
            remote: Some(Remote {
                plugin: "p".into(),
                source: "s".into(),
                id: "r1".into(),
            }),
            ..Default::default()
        };
        s.collection_put(&[cloud], false).unwrap();
        let b = s.backup(&d, None, &[], "t").unwrap();
        s.run("DELETE FROM collection", []).unwrap();
        assert!(s.collection_remotes().unwrap().is_empty());
        s.restore(&b.zip).unwrap();
        assert!(
            s.collection_remotes()
                .unwrap()
                .contains(&("p".into(), "s".into(), "r1".into()))
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn restore_refuses_an_upstream_backup_before_writing() {
        let d =
            std::env::temp_dir().join(format!("openfader-backup-foreign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let up = d.join("upstream.db");
        let c = Connection::open(&up).unwrap();
        c.execute_batch(
            "CREATE TABLE track (key TEXT PRIMARY KEY, title TEXT);
             CREATE TABLE collection (id INTEGER PRIMARY KEY);",
        )
        .unwrap();
        c.pragma_update(None, "user_version", 17).unwrap();
        drop(c);
        let z = d.join("foreign.zip");
        let entries = [
            ("store.db", std::fs::read(&up).unwrap()),
            ("store.id", super::super::identity::BACKUP_ID.to_vec()),
        ];
        std::fs::write(&z, crate::zip::write(&entries)).unwrap();

        let s = Store::in_memory().unwrap();
        s.set_rating("keep|1", Some(5)).unwrap();
        let err = s.restore(&z).err().unwrap();
        assert!(err.contains("another"), "{err}");
        assert_eq!(s.edits("keep|1").unwrap().rating, Some(5));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn restore_checks_store_id_but_accepts_an_old_backup_without_it() {
        let d = std::env::temp_dir().join(format!("openfader-backup-id-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = Store::in_memory().unwrap();
        s.set_rating("keep|1", Some(5)).unwrap();
        let b = s.backup(&d, None, &[], "t").unwrap();
        let mut z = crate::zip::Zip::open(&b.zip).unwrap();
        let db = z.read("store.db").unwrap();
        drop(z);

        let wrong = d.join("wrong-id.zip");
        let entries = [("store.db", db.clone()), ("store.id", b"NOPE".to_vec())];
        std::fs::write(&wrong, crate::zip::write(&entries)).unwrap();
        let err = s.restore(&wrong).err().unwrap();
        assert!(err.contains("not made by MyOpenFadeR"), "{err}");
        assert_eq!(s.edits("keep|1").unwrap().rating, Some(5));

        let old = d.join("old.zip");
        let entries = [("store.db", db)];
        std::fs::write(&old, crate::zip::write(&entries)).unwrap();
        s.restore(&old).unwrap();
        assert_eq!(s.edits("keep|1").unwrap().rating, Some(5));
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn restore_refuses_a_backup_missing_a_table_before_writing() {
        let d = std::env::temp_dir().join(format!("openfader-backup-table-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = Store::in_memory().unwrap();
        s.set_rating("keep|1", Some(5)).unwrap();
        let b = s.backup(&d, None, &[], "t").unwrap();

        let mut z = crate::zip::Zip::open(&b.zip).unwrap();
        let db = z.read("store.db").unwrap();
        drop(z);
        let edited = d.join("edited.db");
        std::fs::write(&edited, db).unwrap();
        let c = Connection::open(&edited).unwrap();
        c.execute("DROP TABLE gain", []).unwrap();
        drop(c);
        let bad = d.join("missing-table.zip");
        let entries = [
            ("store.db", std::fs::read(&edited).unwrap()),
            ("store.id", super::super::identity::BACKUP_ID.to_vec()),
        ];
        std::fs::write(&bad, crate::zip::write(&entries)).unwrap();
        let err = s.restore(&bad).err().unwrap();
        assert!(err.contains("missing"), "{err}");
        assert_eq!(s.edits("keep|1").unwrap().rating, Some(5));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
