use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::core::db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamiMediaRecord {
    pub id: String,
    pub platform: String,
    pub post_id: String,
    pub author: Option<String>,
    pub url: String,
    pub file_path: String,
    pub file_size: Option<u64>,
    pub sha256: Option<String>,
    pub downloaded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileResult {
    pub checked: usize,
    pub removed: usize,
}

pub fn init_schema_on(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS nami_media (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            post_id TEXT NOT NULL,
            author TEXT,
            url TEXT,
            file_path TEXT NOT NULL,
            file_size INTEGER,
            sha256 TEXT,
            downloaded_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_nami_media_platform ON nami_media(platform);
        CREATE INDEX IF NOT EXISTS idx_nami_media_post_id ON nami_media(platform, post_id);
        CREATE INDEX IF NOT EXISTS idx_nami_media_author ON nami_media(author);",
    )
}

pub fn init() {
    let _ = db::with_conn(|conn| init_schema_on(conn));
}

pub fn record_media_on(conn: &Connection, record: &NamiMediaRecord) -> rusqlite::Result<()> {
    init_schema_on(conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO nami_media (id, platform, post_id, author, url, file_path, file_size, sha256, downloaded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            record.id,
            record.platform.to_lowercase(),
            record.post_id,
            record.author,
            record.url,
            record.file_path,
            record.file_size.map(|s| s as i64),
            record.sha256,
            record.downloaded_at,
        ],
    )?;
    Ok(())
}

pub fn record_media(record: &NamiMediaRecord) -> Result<(), String> {
    db::with_conn(|conn| record_media_on(conn, record))
        .ok_or_else(|| "Failed to access SQLite database".to_string())
}

pub fn has_media_on(conn: &Connection, platform: &str, post_id: &str) -> rusqlite::Result<bool> {
    init_schema_on(conn)?;
    let mut stmt = conn
        .prepare_cached("SELECT 1 FROM nami_media WHERE platform = ?1 AND post_id = ?2 LIMIT 1")?;
    let exists = stmt.exists(params![platform.to_lowercase(), post_id])?;
    Ok(exists)
}

pub fn has_media(platform: &str, post_id: &str) -> bool {
    db::with_conn(|conn| has_media_on(conn, platform, post_id)).unwrap_or(false)
}

pub fn get_history_on(
    conn: &Connection,
    platform: Option<&str>,
    limit: u32,
    offset: u32,
) -> rusqlite::Result<Vec<NamiMediaRecord>> {
    init_schema_on(conn)?;
    let mut records = Vec::new();

    if let Some(p) = platform {
        let mut stmt = conn.prepare_cached(
            "SELECT id, platform, post_id, author, url, file_path, file_size, sha256, downloaded_at
             FROM nami_media
             WHERE platform = ?1
             ORDER BY downloaded_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![p.to_lowercase(), limit, offset], row_to_record)?;
        for row in rows {
            records.push(row?);
        }
    } else {
        let mut stmt = conn.prepare_cached(
            "SELECT id, platform, post_id, author, url, file_path, file_size, sha256, downloaded_at
             FROM nami_media
             ORDER BY downloaded_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], row_to_record)?;
        for row in rows {
            records.push(row?);
        }
    }

    Ok(records)
}

pub fn get_history(
    platform: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<Vec<NamiMediaRecord>, String> {
    db::with_conn(|conn| get_history_on(conn, platform, limit, offset))
        .ok_or_else(|| "Failed to query history from database".to_string())
}

pub fn delete_record_on(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    init_schema_on(conn)?;
    let rows = conn.execute("DELETE FROM nami_media WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

pub fn delete_record(id: &str) -> Result<bool, String> {
    db::with_conn(|conn| delete_record_on(conn, id))
        .ok_or_else(|| "Failed to delete record from database".to_string())
}

pub fn reconcile_disk_on(conn: &Connection) -> rusqlite::Result<ReconcileResult> {
    init_schema_on(conn)?;
    let mut stmt = conn.prepare_cached("SELECT id, file_path FROM nami_media")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut to_delete = Vec::new();
    let mut checked = 0;

    for row in rows {
        let (id, file_path) = row?;
        checked += 1;
        let p = Path::new(&file_path);
        if let Some(parent) = p.parent() {
            if parent.exists() && !p.exists() {
                to_delete.push(id);
            }
        }
    }

    let removed = to_delete.len();
    for id in to_delete {
        conn.execute("DELETE FROM nami_media WHERE id = ?1", params![id])?;
    }

    Ok(ReconcileResult { checked, removed })
}

pub fn reconcile_disk() -> Result<ReconcileResult, String> {
    db::with_conn(|conn| reconcile_disk_on(conn))
        .ok_or_else(|| "Failed to reconcile disk with database".to_string())
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<NamiMediaRecord> {
    let size_raw: Option<i64> = row.get(6)?;
    Ok(NamiMediaRecord {
        id: row.get(0)?,
        platform: row.get(1)?,
        post_id: row.get(2)?,
        author: row.get(3)?,
        url: row.get(4)?,
        file_path: row.get(5)?,
        file_size: size_raw.map(|s| s as u64),
        sha256: row.get(7)?,
        downloaded_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_has_media() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema_on(&conn).unwrap();

        let record = NamiMediaRecord {
            id: "tiktok_123456789".to_string(),
            platform: "tiktok".to_string(),
            post_id: "123456789".to_string(),
            author: Some("creator".to_string()),
            url: "https://www.tiktok.com/@creator/video/123456789".to_string(),
            file_path: "/downloads/Nami/TikTok/creator/video.mp4".to_string(),
            file_size: Some(10485760),
            sha256: Some("abcdef123456".to_string()),
            downloaded_at: 1731280000,
        };

        assert!(!has_media_on(&conn, "tiktok", "123456789").unwrap());
        record_media_on(&conn, &record).unwrap();
        assert!(has_media_on(&conn, "tiktok", "123456789").unwrap());
        assert!(has_media_on(&conn, "TikTok", "123456789").unwrap());
        assert!(!has_media_on(&conn, "instagram", "123456789").unwrap());
    }

    #[test]
    fn test_get_history_pagination_and_filter() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema_on(&conn).unwrap();

        for i in 1..=5 {
            let record = NamiMediaRecord {
                id: format!("instagram_{}", i),
                platform: "instagram".to_string(),
                post_id: format!("{}", i),
                author: Some("ig_creator".to_string()),
                url: format!("https://instagram.com/p/{}", i),
                file_path: format!("/downloads/{}.jpg", i),
                file_size: Some(5000),
                sha256: None,
                downloaded_at: 1731280000 + i,
            };
            record_media_on(&conn, &record).unwrap();
        }

        let history = get_history_on(&conn, Some("instagram"), 3, 0).unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].id, "instagram_5");
        assert_eq!(history[1].id, "instagram_4");
        assert_eq!(history[2].id, "instagram_3");

        let paged = get_history_on(&conn, Some("instagram"), 3, 3).unwrap();
        assert_eq!(paged.len(), 2);
        assert_eq!(paged[0].id, "instagram_2");
        assert_eq!(paged[1].id, "instagram_1");
    }

    #[test]
    fn test_delete_record() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema_on(&conn).unwrap();

        let record = NamiMediaRecord {
            id: "x_100".to_string(),
            platform: "x".to_string(),
            post_id: "100".to_string(),
            author: Some("user".to_string()),
            url: "https://x.com/user/status/100".to_string(),
            file_path: "/downloads/100.mp4".to_string(),
            file_size: None,
            sha256: None,
            downloaded_at: 1731280000,
        };

        record_media_on(&conn, &record).unwrap();
        assert!(has_media_on(&conn, "x", "100").unwrap());

        assert!(delete_record_on(&conn, "x_100").unwrap());
        assert!(!has_media_on(&conn, "x", "100").unwrap());
        assert!(!delete_record_on(&conn, "x_100").unwrap());
    }

    #[test]
    fn test_reconcile_disk_removes_missing_files() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema_on(&conn).unwrap();

        let temp_dir = std::env::temp_dir().join(format!(
            "nami_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let existing_file = temp_dir.join("present.mp4");
        std::fs::write(&existing_file, "content").unwrap();
        let missing_file = temp_dir.join("missing.mp4");

        let rec_present = NamiMediaRecord {
            id: "rec_1".to_string(),
            platform: "tiktok".to_string(),
            post_id: "p1".to_string(),
            author: None,
            url: "https://tiktok.com/1".to_string(),
            file_path: existing_file.to_string_lossy().to_string(),
            file_size: Some(7),
            sha256: None,
            downloaded_at: 100,
        };
        let rec_missing = NamiMediaRecord {
            id: "rec_2".to_string(),
            platform: "tiktok".to_string(),
            post_id: "p2".to_string(),
            author: None,
            url: "https://tiktok.com/2".to_string(),
            file_path: missing_file.to_string_lossy().to_string(),
            file_size: Some(10),
            sha256: None,
            downloaded_at: 200,
        };

        record_media_on(&conn, &rec_present).unwrap();
        record_media_on(&conn, &rec_missing).unwrap();

        let res = reconcile_disk_on(&conn).unwrap();
        assert_eq!(res.checked, 2);
        assert_eq!(res.removed, 1);

        assert!(has_media_on(&conn, "tiktok", "p1").unwrap());
        assert!(!has_media_on(&conn, "tiktok", "p2").unwrap());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
