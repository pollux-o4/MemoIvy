use rusqlite::Connection;
pub(super) fn initialize(db: &mut Connection) -> Result<(), String> {
    let schema: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if schema > 2 {
        return Err(format!(
            "database schema {schema} is newer than supported schema 2"
        ));
    }
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
   CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
   INSERT OR IGNORE INTO meta(key,value) VALUES ('revision',0);
   CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, body TEXT NOT NULL);
   CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, name TEXT NOT NULL, mime TEXT NOT NULL, data BLOB NOT NULL, added INTEGER NOT NULL, last_note TEXT, unused_since INTEGER);
   CREATE TABLE IF NOT EXISTS preferences (key TEXT PRIMARY KEY, body TEXT NOT NULL);
   CREATE TABLE IF NOT EXISTS note_revisions (id TEXT PRIMARY KEY, revision INTEGER NOT NULL);
   CREATE TABLE IF NOT EXISTS preference_revisions (key TEXT PRIMARY KEY, revision INTEGER NOT NULL);").map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let revision: i64 = tx
        .query_row("SELECT value FROM meta WHERE key='revision'", [], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT OR IGNORE INTO note_revisions(id,revision) SELECT id,?1 FROM notes",
        [revision],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT OR IGNORE INTO preference_revisions(key,revision) SELECT key,?1 FROM preferences",
        [revision],
    )
    .map_err(|e| e.to_string())?;
    tx.pragma_update(None, "user_version", 2)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
