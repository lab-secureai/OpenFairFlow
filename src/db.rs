#[cfg(feature = "server")]
use rusqlite::{params, Connection};

#[cfg(feature = "server")]
use crate::models::Dataset;

#[cfg(feature = "server")]
use crate::models::Workspace;

#[cfg(feature = "server")]
static DB_PATH: &str = "data/open_fair_flow.db";

#[cfg(feature = "server")]
fn get_connection() -> Result<Connection, rusqlite::Error> {
    Connection::open(DB_PATH)
}

#[cfg(feature = "server")]
pub fn init_db() -> Result<(), rusqlite::Error> {
    std::fs::create_dir_all("data/datasets").ok();
    let conn = get_connection()?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS datasets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            dataset_type TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            tags TEXT NOT NULL DEFAULT '[]',
            format TEXT NOT NULL DEFAULT '',
            num_samples INTEGER,
            num_classes INTEGER,
            file_size INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'local',
            file_path TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'ready'
        )",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            dataset_id TEXT NOT NULL,
            dataset_name TEXT NOT NULL DEFAULT '',
            code TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_run_result TEXT
        )",
    )?;
    // Migration: add last_run_result column if missing
    let _ = conn.execute_batch(
        "ALTER TABLE workspaces ADD COLUMN last_run_result TEXT",
    );
    Ok(())
}

#[cfg(feature = "server")]
pub fn insert_dataset(dataset: &Dataset) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    let tags_json = serde_json::to_string(&dataset.tags).unwrap_or_default();
    conn.execute(
        "INSERT INTO datasets (id, name, dataset_type, description, tags, format, num_samples, num_classes, file_size, source, file_path, created_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            dataset.id,
            dataset.name,
            dataset.dataset_type,
            dataset.description,
            tags_json,
            dataset.format,
            dataset.num_samples,
            dataset.num_classes,
            dataset.file_size as i64,
            dataset.source,
            dataset.file_path,
            dataset.created_at,
            dataset.status,
        ],
    )?;
    Ok(())
}

#[cfg(feature = "server")]
pub fn list_datasets() -> Result<Vec<Dataset>, rusqlite::Error> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, dataset_type, description, tags, format, num_samples, num_classes, file_size, source, file_path, created_at, status
         FROM datasets ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let tags_str: String = row.get(4)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        let file_size: i64 = row.get(8)?;
        Ok(Dataset {
            id: row.get(0)?,
            name: row.get(1)?,
            dataset_type: row.get(2)?,
            description: row.get(3)?,
            tags,
            format: row.get(5)?,
            num_samples: row.get(6)?,
            num_classes: row.get(7)?,
            file_size: file_size as u64,
            source: row.get(9)?,
            file_path: row.get(10)?,
            created_at: row.get(11)?,
            status: row.get(12)?,
        })
    })?;
    rows.collect()
}

#[cfg(feature = "server")]
pub fn get_dataset(id: &str) -> Result<Option<Dataset>, rusqlite::Error> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, dataset_type, description, tags, format, num_samples, num_classes, file_size, source, file_path, created_at, status
         FROM datasets WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        let tags_str: String = row.get(4)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        let file_size: i64 = row.get(8)?;
        Ok(Dataset {
            id: row.get(0)?,
            name: row.get(1)?,
            dataset_type: row.get(2)?,
            description: row.get(3)?,
            tags,
            format: row.get(5)?,
            num_samples: row.get(6)?,
            num_classes: row.get(7)?,
            file_size: file_size as u64,
            source: row.get(9)?,
            file_path: row.get(10)?,
            created_at: row.get(11)?,
            status: row.get(12)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

#[cfg(feature = "server")]
pub fn update_dataset_status(id: &str, status: &str, file_size: Option<u64>) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    if let Some(size) = file_size {
        conn.execute(
            "UPDATE datasets SET status = ?1, file_size = ?2 WHERE id = ?3",
            params![status, size as i64, id],
        )?;
    } else {
        conn.execute(
            "UPDATE datasets SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
    }
    Ok(())
}

#[cfg(feature = "server")]
pub fn delete_dataset_db(id: &str) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    conn.execute("DELETE FROM datasets WHERE id = ?1", params![id])?;
    Ok(())
}

// ---- Workspace CRUD ----

#[cfg(feature = "server")]
pub fn insert_workspace(ws: &Workspace) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    conn.execute(
        "INSERT INTO workspaces (id, name, dataset_id, dataset_name, code, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![ws.id, ws.name, ws.dataset_id, ws.dataset_name, ws.code, ws.created_at, ws.updated_at],
    )?;
    Ok(())
}

#[cfg(feature = "server")]
pub fn list_workspaces() -> Result<Vec<Workspace>, rusqlite::Error> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, dataset_id, dataset_name, code, created_at, updated_at
         FROM workspaces ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            dataset_id: row.get(2)?,
            dataset_name: row.get(3)?,
            code: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    rows.collect()
}

#[cfg(feature = "server")]
pub fn get_workspace(id: &str) -> Result<Option<Workspace>, rusqlite::Error> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, dataset_id, dataset_name, code, created_at, updated_at
         FROM workspaces WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            dataset_id: row.get(2)?,
            dataset_name: row.get(3)?,
            code: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

#[cfg(feature = "server")]
pub fn update_workspace_code(id: &str, code: &str) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE workspaces SET code = ?1, updated_at = ?2 WHERE id = ?3",
        params![code, now, id],
    )?;
    Ok(())
}

#[cfg(feature = "server")]
pub fn save_workspace_run_result(id: &str, result_json: &str) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    conn.execute(
        "UPDATE workspaces SET last_run_result = ?1 WHERE id = ?2",
        params![result_json, id],
    )?;
    Ok(())
}

#[cfg(feature = "server")]
pub fn get_workspace_run_result(id: &str) -> Result<Option<String>, rusqlite::Error> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT last_run_result FROM workspaces WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        let val: Option<String> = row.get(0)?;
        Ok(val)
    })?;
    match rows.next() {
        Some(Ok(val)) => Ok(val),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

#[cfg(feature = "server")]
pub fn delete_workspace_db(id: &str) -> Result<(), rusqlite::Error> {
    let conn = get_connection()?;
    conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
    Ok(())
}
