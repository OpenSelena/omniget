use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CourseRecord {
    pub id: i64,
    pub title: String,
    pub source_path: String,
    pub thumbnail_path: Option<String>,
    pub added_at: String,
    pub last_scan_at: String,
    pub favorite: bool,
    pub total_lessons: i64,
    pub completed_lessons: i64,
    pub progress_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CourseHeader {
    pub id: i64,
    pub title: String,
    pub source_path: String,
    pub thumbnail_path: Option<String>,
    pub added_at: String,
    pub last_scan_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LessonRecord {
    pub id: i64,
    pub course_id: i64,
    pub module_id: Option<i64>,
    pub position: i64,
    pub title: String,
    pub video_path: String,
    pub subtitle_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub current_seconds: f64,
    pub completed: bool,
    pub description_raw: Option<String>,
    pub description_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleRecord {
    pub id: i64,
    pub course_id: i64,
    pub position: i64,
    pub title: String,
    pub source_path: String,
    pub lessons: Vec<LessonRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CourseDetailRecord {
    pub course: CourseHeader,
    pub modules: Vec<ModuleRecord>,
    pub loose_lessons: Vec<LessonRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LessonProgressRecord {
    pub lesson_id: i64,
    pub course_id: i64,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub completed: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentCourseRecord {
    pub course_id: i64,
    pub title: String,
    pub source_path: String,
    pub thumbnail_path: Option<String>,
    pub last_watched_at: i64,
}

#[derive(Clone)]
pub struct StudyDb {
    conn: Arc<Mutex<Connection>>,
}

impl StudyDb {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
        let db = Self::new(conn);
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_default() -> rusqlite::Result<Self> {
        let path = omniget_core::core::paths::app_data_dir()
            .map(|d| d.join("study.db"))
            .unwrap_or_else(|| PathBuf::from("study.db"));
        Self::open(&path)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self::new(conn);
        db.init_schema()?;
        Ok(db)
    }

    pub fn init_schema(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS courses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                source_path TEXT NOT NULL UNIQUE,
                thumbnail_path TEXT,
                added_at TEXT NOT NULL,
                last_scan_at TEXT NOT NULL,
                favorite INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS modules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                course_id INTEGER NOT NULL,
                position INTEGER NOT NULL,
                title TEXT NOT NULL,
                source_path TEXT NOT NULL,
                FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS lessons (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                course_id INTEGER NOT NULL,
                module_id INTEGER,
                position INTEGER NOT NULL,
                title TEXT NOT NULL,
                video_path TEXT NOT NULL UNIQUE,
                subtitle_path TEXT,
                duration_ms INTEGER,
                description_raw TEXT,
                description_format TEXT,
                FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE,
                FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS lesson_progress (
                lesson_id INTEGER PRIMARY KEY,
                course_id INTEGER NOT NULL,
                position_secs REAL NOT NULL DEFAULT 0.0,
                duration_secs REAL NOT NULL DEFAULT 0.0,
                completed INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_lesson_progress_course ON lesson_progress(course_id);

            CREATE TABLE IF NOT EXISTS course_meta (
                course_id INTEGER PRIMARY KEY,
                playback_speed REAL NOT NULL DEFAULT 1.0,
                last_watched_at INTEGER,
                updated_at INTEGER NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    // --- Course Discovery & Management ---

    pub fn upsert_course(
        &self,
        title: &str,
        source_path: &str,
        thumbnail: Option<&str>,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = now_iso();
        conn.execute(
            "INSERT INTO courses (title, source_path, thumbnail_path, added_at, last_scan_at, favorite)
             VALUES (?1, ?2, ?3, ?4, ?4, 0)
             ON CONFLICT(source_path) DO UPDATE SET
                 title = excluded.title,
                 thumbnail_path = COALESCE(excluded.thumbnail_path, courses.thumbnail_path),
                 last_scan_at = excluded.last_scan_at",
            params![title, source_path, thumbnail, now],
        )?;

        let id = conn.query_row(
            "SELECT id FROM courses WHERE source_path = ?1",
            params![source_path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn upsert_module(
        &self,
        course_id: i64,
        position: i64,
        title: &str,
        source_path: &str,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM modules WHERE course_id = ?1 AND source_path = ?2",
                params![course_id, source_path],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            conn.execute(
                "UPDATE modules SET position = ?1, title = ?2 WHERE id = ?3",
                params![position, title, id],
            )?;
            Ok(id)
        } else {
            conn.execute(
                "INSERT INTO modules (course_id, position, title, source_path) VALUES (?1, ?2, ?3, ?4)",
                params![course_id, position, title, source_path],
            )?;
            Ok(conn.last_insert_rowid())
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_lesson(
        &self,
        course_id: i64,
        module_id: Option<i64>,
        position: i64,
        title: &str,
        video_path: &str,
        subtitle_path: Option<&str>,
        duration_ms: Option<i64>,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO lessons (course_id, module_id, position, title, video_path, subtitle_path, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(video_path) DO UPDATE SET
                 course_id = excluded.course_id,
                 module_id = excluded.module_id,
                 position = excluded.position,
                 title = excluded.title,
                 subtitle_path = COALESCE(excluded.subtitle_path, lessons.subtitle_path),
                 duration_ms = COALESCE(excluded.duration_ms, lessons.duration_ms)",
            params![course_id, module_id, position, title, video_path, subtitle_path, duration_ms],
        )?;

        let id = conn.query_row(
            "SELECT id FROM lessons WHERE video_path = ?1",
            params![video_path],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn list_courses(&self) -> rusqlite::Result<Vec<CourseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT
                c.id, c.title, c.source_path, c.thumbnail_path, c.added_at, c.last_scan_at, c.favorite,
                COUNT(l.id) AS total_lessons,
                COUNT(CASE WHEN lp.completed = 1 THEN 1 END) AS completed_lessons
             FROM courses c
             LEFT JOIN lessons l ON l.course_id = c.id
             LEFT JOIN lesson_progress lp ON lp.lesson_id = l.id
             GROUP BY c.id
             ORDER BY c.favorite DESC, c.added_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let total_lessons: i64 = row.get(7)?;
            let completed_lessons: i64 = row.get(8)?;
            let progress_pct = if total_lessons > 0 {
                (completed_lessons as f64 / total_lessons as f64) * 100.0
            } else {
                0.0
            };

            let favorite_int: i64 = row.get(6)?;
            Ok(CourseRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                source_path: row.get(2)?,
                thumbnail_path: row.get(3)?,
                added_at: row.get(4)?,
                last_scan_at: row.get(5)?,
                favorite: favorite_int != 0,
                total_lessons,
                completed_lessons,
                progress_pct,
            })
        })?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    pub fn get_course_detail(
        &self,
        course_id: i64,
    ) -> rusqlite::Result<Option<CourseDetailRecord>> {
        let conn = self.conn.lock().unwrap();
        let course_header: Option<CourseHeader> = conn
            .query_row(
                "SELECT id, title, source_path, thumbnail_path, added_at, last_scan_at
                 FROM courses WHERE id = ?1",
                params![course_id],
                |row| {
                    Ok(CourseHeader {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        source_path: row.get(2)?,
                        thumbnail_path: row.get(3)?,
                        added_at: row.get(4)?,
                        last_scan_at: row.get(5)?,
                    })
                },
            )
            .optional()?;

        let course = match course_header {
            Some(c) => c,
            None => return Ok(None),
        };

        // Fetch modules
        let mut mod_stmt = conn.prepare(
            "SELECT id, course_id, position, title, source_path
             FROM modules WHERE course_id = ?1 ORDER BY position ASC, id ASC",
        )?;
        let modules_raw = mod_stmt.query_map(params![course_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut modules = Vec::new();
        for m in modules_raw {
            let (id, c_id, pos, title, src) = m?;
            modules.push(ModuleRecord {
                id,
                course_id: c_id,
                position: pos,
                title,
                source_path: src,
                lessons: Vec::new(),
            });
        }

        // Fetch all lessons for course with progress
        let mut lesson_stmt = conn.prepare(
            "SELECT
                l.id, l.course_id, l.module_id, l.position, l.title, l.video_path,
                l.subtitle_path, l.duration_ms, l.description_raw, l.description_format,
                COALESCE(lp.position_secs, 0.0),
                COALESCE(lp.completed, 0)
             FROM lessons l
             LEFT JOIN lesson_progress lp ON lp.lesson_id = l.id
             WHERE l.course_id = ?1
             ORDER BY l.position ASC, l.id ASC",
        )?;

        let lesson_rows = lesson_stmt.query_map(params![course_id], |row| {
            let completed_int: i64 = row.get(11)?;
            Ok(LessonRecord {
                id: row.get(0)?,
                course_id: row.get(1)?,
                module_id: row.get(2)?,
                position: row.get(3)?,
                title: row.get(4)?,
                video_path: row.get(5)?,
                subtitle_path: row.get(6)?,
                duration_ms: row.get(7)?,
                description_raw: row.get(8)?,
                description_format: row.get(9)?,
                current_seconds: row.get(10)?,
                completed: completed_int != 0,
            })
        })?;

        let mut loose_lessons = Vec::new();
        for l in lesson_rows {
            let lesson = l?;
            if let Some(mod_id) = lesson.module_id {
                if let Some(m) = modules.iter_mut().find(|m| m.id == mod_id) {
                    m.lessons.push(lesson);
                } else {
                    loose_lessons.push(lesson);
                }
            } else {
                loose_lessons.push(lesson);
            }
        }

        Ok(Some(CourseDetailRecord {
            course,
            modules,
            loose_lessons,
        }))
    }

    pub fn get_lesson(&self, lesson_id: i64) -> rusqlite::Result<Option<LessonRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT
                l.id, l.course_id, l.module_id, l.position, l.title, l.video_path,
                l.subtitle_path, l.duration_ms, l.description_raw, l.description_format,
                COALESCE(lp.position_secs, 0.0),
                COALESCE(lp.completed, 0)
             FROM lessons l
             LEFT JOIN lesson_progress lp ON lp.lesson_id = l.id
             WHERE l.id = ?1",
            params![lesson_id],
            |row| {
                let completed_int: i64 = row.get(11)?;
                Ok(LessonRecord {
                    id: row.get(0)?,
                    course_id: row.get(1)?,
                    module_id: row.get(2)?,
                    position: row.get(3)?,
                    title: row.get(4)?,
                    video_path: row.get(5)?,
                    subtitle_path: row.get(6)?,
                    duration_ms: row.get(7)?,
                    description_raw: row.get(8)?,
                    description_format: row.get(9)?,
                    current_seconds: row.get(10)?,
                    completed: completed_int != 0,
                })
            },
        )
        .optional()
    }

    // --- Progress & Player Persistence ---

    pub fn save_progress(
        &self,
        lesson_id: i64,
        course_id: Option<i64>,
        position_secs: f64,
        duration_secs: Option<f64>,
        completed: Option<bool>,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        let resolved_course_id = match course_id {
            Some(cid) => cid,
            None => conn
                .query_row(
                    "SELECT course_id FROM lessons WHERE id = ?1",
                    params![lesson_id],
                    |r| r.get(0),
                )
                .unwrap_or(0),
        };

        let now = now_secs();
        let completed_int = completed.map(|c| if c { 1 } else { 0 });

        conn.execute(
            "INSERT INTO lesson_progress (lesson_id, course_id, position_secs, duration_secs, completed, updated_at)
             VALUES (?1, ?2, ?3, COALESCE(?4, 0.0), COALESCE(?5, 0), ?6)
             ON CONFLICT(lesson_id) DO UPDATE SET
                 position_secs = excluded.position_secs,
                 duration_secs = CASE WHEN excluded.duration_secs > 0 THEN excluded.duration_secs ELSE lesson_progress.duration_secs END,
                 completed = CASE WHEN ?5 IS NOT NULL THEN excluded.completed ELSE lesson_progress.completed END,
                 updated_at = excluded.updated_at",
            params![
                lesson_id,
                resolved_course_id,
                position_secs,
                duration_secs,
                completed_int,
                now
            ],
        )?;

        // Also touch course recent if we know the course
        if resolved_course_id > 0 {
            let _ = conn.execute(
                "INSERT INTO course_meta (course_id, playback_speed, last_watched_at, updated_at)
                 VALUES (?1, 1.0, ?2, ?2)
                 ON CONFLICT(course_id) DO UPDATE SET
                     last_watched_at = excluded.last_watched_at,
                     updated_at = excluded.updated_at",
                params![resolved_course_id, now],
            );
        }

        Ok(())
    }

    pub fn get_progress(&self, lesson_id: i64) -> rusqlite::Result<Option<LessonProgressRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT lesson_id, course_id, position_secs, duration_secs, completed, updated_at
             FROM lesson_progress WHERE lesson_id = ?1",
            params![lesson_id],
            |row| {
                let completed_int: i64 = row.get(4)?;
                Ok(LessonProgressRecord {
                    lesson_id: row.get(0)?,
                    course_id: row.get(1)?,
                    position_secs: row.get(2)?,
                    duration_secs: row.get(3)?,
                    completed: completed_int != 0,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
    }

    pub fn mark_lesson_completed(
        &self,
        lesson_id: i64,
        seconds: f64,
        completed: bool,
    ) -> rusqlite::Result<()> {
        self.save_progress(lesson_id, None, seconds, None, Some(completed))
    }

    pub fn mark_lessons_bulk(&self, lesson_ids: &[i64], completed: bool) -> rusqlite::Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = now_secs();
        let completed_int = if completed { 1 } else { 0 };

        for &lid in lesson_ids {
            let cid: i64 = tx
                .query_row(
                    "SELECT course_id FROM lessons WHERE id = ?1",
                    params![lid],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            tx.execute(
                "INSERT INTO lesson_progress (lesson_id, course_id, position_secs, duration_secs, completed, updated_at)
                 VALUES (?1, ?2, 0.0, 0.0, ?3, ?4)
                 ON CONFLICT(lesson_id) DO UPDATE SET
                     completed = excluded.completed,
                     updated_at = excluded.updated_at",
                params![lid, cid, completed_int, now],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    // --- Course Meta & Playback Speed ---

    pub fn get_playback_speed(&self, course_id: i64) -> rusqlite::Result<Option<f64>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT playback_speed FROM course_meta WHERE course_id = ?1",
            params![course_id],
            |row| row.get(0),
        )
        .optional()
    }

    pub fn set_playback_speed(&self, course_id: i64, speed: Option<f64>) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        conn.execute(
            "INSERT INTO course_meta (course_id, playback_speed, updated_at)
             VALUES (?1, COALESCE(?2, 1.0), ?3)
             ON CONFLICT(course_id) DO UPDATE SET
                 playback_speed = excluded.playback_speed,
                 updated_at = excluded.updated_at",
            params![course_id, speed, now],
        )?;
        Ok(())
    }

    pub fn touch_course_recent(&self, course_id: i64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        conn.execute(
            "INSERT INTO course_meta (course_id, playback_speed, last_watched_at, updated_at)
             VALUES (?1, 1.0, ?2, ?2)
             ON CONFLICT(course_id) DO UPDATE SET
                 last_watched_at = excluded.last_watched_at,
                 updated_at = excluded.updated_at",
            params![course_id, now],
        )?;
        Ok(())
    }

    pub fn toggle_favorite(&self, course_id: i64) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE courses SET favorite = 1 - favorite WHERE id = ?1",
            params![course_id],
        )?;
        let fav: i64 = conn.query_row(
            "SELECT favorite FROM courses WHERE id = ?1",
            params![course_id],
            |r| r.get(0),
        )?;
        Ok(fav != 0)
    }

    pub fn list_recents(&self, limit: usize) -> rusqlite::Result<Vec<RecentCourseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.source_path, c.thumbnail_path, cm.last_watched_at
             FROM courses c
             JOIN course_meta cm ON c.id = cm.course_id
             WHERE cm.last_watched_at IS NOT NULL
             ORDER BY cm.last_watched_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(RecentCourseRecord {
                course_id: row.get(0)?,
                title: row.get(1)?,
                source_path: row.get(2)?,
                thumbnail_path: row.get(3)?,
                last_watched_at: row.get(4)?,
            })
        })?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation_and_course_crud() {
        let db = StudyDb::open_in_memory().expect("in-memory db must initialize");

        let cid = db
            .upsert_course("Rust Masterclass", "/downloads/courses/rust", None)
            .expect("course upsert should succeed");
        assert!(cid > 0);

        let mid = db
            .upsert_module(
                cid,
                1,
                "Module 1: Basics",
                "/downloads/courses/rust/01-basics",
            )
            .expect("module upsert should succeed");
        assert!(mid > 0);

        let lid1 = db
            .upsert_lesson(
                cid,
                Some(mid),
                1,
                "01 Intro.mp4",
                "/downloads/courses/rust/01-basics/01.mp4",
                None,
                Some(120_000),
            )
            .expect("lesson 1 upsert should succeed");
        assert!(lid1 > 0);

        let lid2 = db
            .upsert_lesson(
                cid,
                Some(mid),
                2,
                "02 Setup.mp4",
                "/downloads/courses/rust/01-basics/02.mp4",
                None,
                Some(300_000),
            )
            .expect("lesson 2 upsert should succeed");
        assert!(lid2 > 0);

        // Check course listing
        let list = db.list_courses().expect("list courses should succeed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Rust Masterclass");
        assert_eq!(list[0].total_lessons, 2);
        assert_eq!(list[0].completed_lessons, 0);
        assert_eq!(list[0].progress_pct, 0.0);
        assert!(!list[0].favorite);

        // Save progress for lesson 1
        db.save_progress(lid1, Some(cid), 45.5, Some(120.0), Some(false))
            .expect("save progress should succeed");

        let p1 = db
            .get_progress(lid1)
            .expect("get progress")
            .expect("progress exists");
        assert_eq!(p1.position_secs, 45.5);
        assert!(!p1.completed);

        // Mark lesson 1 completed
        db.mark_lesson_completed(lid1, 120.0, true)
            .expect("mark completed should succeed");

        let p1_after = db
            .get_progress(lid1)
            .expect("get progress")
            .expect("progress exists");
        assert!(p1_after.completed);

        // Check updated course progress
        let list_after = db.list_courses().expect("list courses");
        assert_eq!(list_after[0].completed_lessons, 1);
        assert_eq!(list_after[0].progress_pct, 50.0);

        // Test bulk completion
        db.mark_lessons_bulk(&[lid2], true)
            .expect("bulk completion should succeed");
        let list_bulk = db.list_courses().expect("list courses");
        assert_eq!(list_bulk[0].completed_lessons, 2);
        assert_eq!(list_bulk[0].progress_pct, 100.0);

        // Test course detail query
        let detail = db
            .get_course_detail(cid)
            .expect("course detail")
            .expect("detail exists");
        assert_eq!(detail.course.title, "Rust Masterclass");
        assert_eq!(detail.modules.len(), 1);
        assert_eq!(detail.modules[0].lessons.len(), 2);
        assert_eq!(detail.modules[0].lessons[0].title, "01 Intro.mp4");
        assert!(detail.modules[0].lessons[0].completed);
        assert_eq!(detail.modules[0].lessons[0].current_seconds, 120.0);

        // Test single lesson query
        let single = db
            .get_lesson(lid1)
            .expect("get lesson")
            .expect("lesson exists");
        assert_eq!(single.title, "01 Intro.mp4");
        assert_eq!(single.current_seconds, 120.0);
        assert!(single.completed);

        // Test favorite toggling
        let fav1 = db.toggle_favorite(cid).expect("toggle favorite");
        assert!(fav1);
        let list_fav = db.list_courses().expect("list courses");
        assert!(list_fav[0].favorite);

        let fav2 = db.toggle_favorite(cid).expect("toggle favorite");
        assert!(!fav2);

        // Test playback speed
        db.set_playback_speed(cid, Some(1.5)).expect("set speed");
        let speed = db.get_playback_speed(cid).expect("get speed");
        assert_eq!(speed, Some(1.5));

        // Test recents
        let recents = db.list_recents(10).expect("list recents");
        assert_eq!(recents.len(), 1);
        assert_eq!(recents[0].title, "Rust Masterclass");
    }
}
