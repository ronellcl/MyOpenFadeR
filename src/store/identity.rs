//! Which program wrote a `store.db` (child of [`super`]): the fork and upstream OpenFadeR number their
//! migrations differently from v16 on, so `PRAGMA user_version` alone would let one open the other's file.
//!
//! New stores carry `PRAGMA application_id = 0x4D4F4644` (`MOFD`). Files written before that marker
//! are classified by their schema: the fork's `collection.plugin` or `folder_index`; upstream's
//! `track.title`, `health_failure`, `stage` or `saved_search`. A file with neither is a legacy store
//! up to v15 (the two trees were identical there) and is accepted; anything later without a fork
//! signature is foreign and is refused before it is written.

use super::{R, Sql};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// `PRAGMA application_id` of a MyOpenFadeR store: `MOFD`.
pub(super) const APPLICATION_ID: i32 = 0x4D4F_4644;

/// The `store.id` entry of a backup zip made by this build.
pub(super) const BACKUP_ID: &[u8] = b"MOFD";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// No database yet, or a database with no tables: migrations create them.
    Empty,
    /// This fork's schema.
    Fork,
    /// Upstream OpenFadeR, another program, or a fork file whose schema is too new to classify.
    Foreign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Inspected {
    pub kind: Kind,
    pub schema: usize,
}

impl Inspected {
    pub(super) fn is_fork(self) -> bool {
        self.kind == Kind::Fork
    }
}

/// Reads the header and schema of `path` without writing to it. A missing file is [`Kind::Empty`].
pub(super) fn inspect(path: &Path) -> R<Inspected> {
    if !path.exists() {
        return Ok(Inspected {
            kind: Kind::Empty,
            schema: 0,
        });
    }
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    inspect_conn(&db).map_err(|e| format!("{}: {e}", path.display()))
}

/// [`inspect`] on an open connection; it must run before any migration or journal trigger.
pub(super) fn inspect_conn(db: &Connection) -> R<Inspected> {
    let schema: i64 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .s()?;
    let schema = schema.max(0) as usize;
    let app: i64 = db
        .pragma_query_value(None, "application_id", |r| r.get(0))
        .s()?;
    let kind = if app == i64::from(APPLICATION_ID) {
        Kind::Fork
    } else if app != 0 {
        Kind::Foreign
    } else {
        let tables = table_names(db)?;
        if tables.is_empty() {
            // A bare `user_version` without schema is not a store this build made.
            if schema == 0 {
                Kind::Empty
            } else {
                Kind::Foreign
            }
        } else if tables.iter().any(|t| t == "folder_index")
            || has_column(db, "collection", "plugin")?
        {
            Kind::Fork
        } else if tables
            .iter()
            .any(|t| matches!(t.as_str(), "health_failure" | "stage" | "saved_search"))
            || has_column(db, "track", "title")?
        {
            Kind::Foreign
        } else if schema <= 15 {
            // v1..v15 are shared with upstream: there is nothing to lose and both trees migrate them.
            Kind::Fork
        } else {
            Kind::Foreign
        }
    };
    Ok(Inspected { kind, schema })
}

/// [`inspect`] refusing a foreign store with the message the user sees.
pub(super) fn check(path: &Path) -> R<Inspected> {
    let found = inspect(path)?;
    if found.kind == Kind::Foreign {
        return Err(foreign(found.schema));
    }
    Ok(found)
}

/// [`check`] for a connection that is already open.
pub(super) fn check_conn(db: &Connection) -> R<Inspected> {
    let found = inspect_conn(db)?;
    if found.kind == Kind::Foreign {
        return Err(foreign(found.schema));
    }
    Ok(found)
}

/// Marks a store that this build opened as its own. Legacy fork stores get the marker on their first open;
/// an empty database gets it after the migrations have created the schema.
pub(super) fn stamp(db: &Connection) -> R<()> {
    db.pragma_update(None, "application_id", APPLICATION_ID).s()
}

/// Rejects a backup zip that says it was made by another program, before its database is attached.
pub(super) fn check_backup_id(id: &[u8]) -> R<()> {
    if id == BACKUP_ID {
        Ok(())
    } else {
        Err("this backup was not made by MyOpenFadeR; it was not restored".into())
    }
}

pub(super) fn foreign(schema: usize) -> String {
    format!(
        "store.db is schema v{schema} from another OpenFadeR build or program, not a MyOpenFadeR store; \
         it was not opened because this build would lose data; export the library from that build and import it here"
    )
}

fn table_names(db: &Connection) -> R<Vec<String>> {
    db.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .s()?
        .query_map([], |r| r.get(0))
        .s()?
        .collect::<rusqlite::Result<_>>()
        .s()
}

fn has_column(db: &Connection, table: &str, column: &str) -> R<bool> {
    db.prepare_cached(&format!(
        "SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1"
    ))
    .s()?
    .exists([column])
    .s()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upstream(path: &Path, schema: usize, extra: &str) {
        let db = Connection::open(path).unwrap();
        db.execute_batch(&format!(
            "CREATE TABLE track (key TEXT PRIMARY KEY, title TEXT);
             CREATE TABLE collection (id INTEGER PRIMARY KEY);
             {extra}"
        ))
        .unwrap();
        db.pragma_update(None, "user_version", schema as i64)
            .unwrap();
    }

    #[test]
    fn a_missing_file_is_empty_and_a_fork_store_is_fork() {
        let d = std::env::temp_dir().join(format!("openfader-ident-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("empty.db");
        assert_eq!(inspect(&path).unwrap().kind, Kind::Empty);

        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE folder_index (id INTEGER PRIMARY KEY);")
            .unwrap();
        assert_eq!(inspect(&path).unwrap().kind, Kind::Fork);
        stamp(&db).unwrap();
        assert_eq!(inspect(&path).unwrap().kind, Kind::Fork);
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn upstream_v17_and_v23_are_foreign() {
        let d = std::env::temp_dir().join(format!("openfader-ident-up-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let (v17, v23) = (d.join("v17.db"), d.join("v23.db"));
        upstream(&v17, 17, "");
        upstream(
            &v23,
            23,
            "CREATE TABLE saved_search (id INTEGER PRIMARY KEY);",
        );
        assert_eq!(inspect(&v17).unwrap().kind, Kind::Foreign);
        assert_eq!(inspect(&v23).unwrap().kind, Kind::Foreign);
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn a_nonzero_foreign_application_id_is_foreign() {
        let d = std::env::temp_dir().join(format!("openfader-ident-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("other.db");
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "application_id", 0x1234).unwrap();
        assert_eq!(inspect(&path).unwrap().kind, Kind::Foreign);
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn an_unmarked_legacy_store_is_accepted() {
        let d = std::env::temp_dir().join(format!("openfader-ident-legacy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("legacy.db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE track (key TEXT PRIMARY KEY, rating INTEGER);")
            .unwrap();
        db.pragma_update(None, "user_version", 15).unwrap();
        assert_eq!(inspect(&path).unwrap().kind, Kind::Fork);
        std::fs::remove_dir_all(d).unwrap();
    }
}
