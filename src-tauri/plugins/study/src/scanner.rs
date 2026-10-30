use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::db::StudyDb;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v"];
const SUBTITLE_EXTENSIONS: &[&str] = &["vtt", "srt"];
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            VIDEO_EXTENSIONS
                .iter()
                .any(|&ext| ext.eq_ignore_ascii_case(e))
        })
        .unwrap_or(false)
}

pub fn is_subtitle_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            SUBTITLE_EXTENSIONS
                .iter()
                .any(|&ext| ext.eq_ignore_ascii_case(e))
        })
        .unwrap_or(false)
}

pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|&ext| ext.eq_ignore_ascii_case(e))
        })
        .unwrap_or(false)
}

pub fn extract_position_and_title(filename: &str, fallback_pos: i64) -> (i64, String) {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    let trimmed = stem.trim().trim_start_matches('[');
    let mut num_str = String::new();
    for c in trimmed.chars() {
        if c.is_ascii_digit() {
            num_str.push(c);
        } else {
            break;
        }
    }

    if !num_str.is_empty() {
        if let Ok(pos) = num_str.parse::<i64>() {
            let remainder = &trimmed[num_str.len()..];
            let clean_title = remainder
                .trim_start_matches(|c: char| {
                    c == '.' || c == '-' || c == '_' || c == ']' || c == ' '
                })
                .trim();
            if !clean_title.is_empty() {
                return (pos, clean_title.to_string());
            }
        }
    }

    (fallback_pos, stem.to_string())
}

pub fn find_subtitle_for_video(video_path: &Path) -> Option<PathBuf> {
    let parent = video_path.parent()?;
    let stem = video_path.file_stem()?.to_str()?;

    for ext in SUBTITLE_EXTENSIONS {
        let sub = parent.join(format!("{}.{}", stem, ext));
        if sub.exists() {
            return Some(sub);
        }
    }
    None
}

pub fn find_course_thumbnail(course_dir: &Path) -> Option<PathBuf> {
    for name in &["cover", "folder", "thumbnail", "poster"] {
        for ext in IMAGE_EXTENSIONS {
            let p = course_dir.join(format!("{}.{}", name, ext));
            if p.exists() {
                return Some(p);
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(course_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_image_file(&path) {
                return Some(path);
            }
        }
    }

    None
}

pub fn scan_course_dir(course_dir: &Path, db: &StudyDb) -> anyhow::Result<i64> {
    if !course_dir.is_dir() {
        anyhow::bail!("Path is not a directory: {:?}", course_dir);
    }

    let course_title = course_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled Course");

    let thumbnail_path =
        find_course_thumbnail(course_dir).and_then(|p| p.to_str().map(|s| s.to_string()));

    let source_path = course_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid course path encoding"))?;

    let course_id = db.upsert_course(course_title, source_path, thumbnail_path.as_deref())?;

    let mut direct_videos = Vec::new();
    let mut module_dirs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(course_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // Check if this subdirectory contains video files
                let has_videos = WalkDir::new(&p)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .any(|e| is_video_file(e.path()));

                if has_videos {
                    module_dirs.push(p);
                }
            } else if p.is_file() && is_video_file(&p) {
                direct_videos.push(p);
            }
        }
    }

    // Sort direct videos naturally
    direct_videos.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    for (idx, video) in direct_videos.iter().enumerate() {
        let filename = video.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let (pos, title) = extract_position_and_title(filename, (idx + 1) as i64);
        let video_str = video.to_str().unwrap_or("");
        let sub_str =
            find_subtitle_for_video(video).and_then(|p| p.to_str().map(|s| s.to_string()));

        let _ = db.upsert_lesson(
            course_id,
            None,
            pos,
            &title,
            video_str,
            sub_str.as_deref(),
            None,
        );
    }

    // Process module subdirectories
    module_dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    for (m_idx, m_dir) in module_dirs.iter().enumerate() {
        let m_name = m_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let (m_pos, m_title) = extract_position_and_title(m_name, (m_idx + 1) as i64);
        let m_str = m_dir.to_str().unwrap_or("");

        let mod_id = db.upsert_module(course_id, m_pos, &m_title, m_str)?;

        let mut mod_videos = Vec::new();
        if let Ok(entries) = std::fs::read_dir(m_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && is_video_file(&p) {
                    mod_videos.push(p);
                }
            }
        }

        mod_videos.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        for (l_idx, video) in mod_videos.iter().enumerate() {
            let filename = video.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let (pos, title) = extract_position_and_title(filename, (l_idx + 1) as i64);
            let video_str = video.to_str().unwrap_or("");
            let sub_str =
                find_subtitle_for_video(video).and_then(|p| p.to_str().map(|s| s.to_string()));

            let _ = db.upsert_lesson(
                course_id,
                Some(mod_id),
                pos,
                &title,
                video_str,
                sub_str.as_deref(),
                None,
            );
        }
    }

    Ok(course_id)
}

pub fn scan_root_dir(root: &Path, db: &StudyDb) -> anyhow::Result<usize> {
    if !root.is_dir() {
        return Ok(0);
    }

    let mut scanned = 0;
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if directory has any video files anywhere inside
                let has_videos = WalkDir::new(&path)
                    .max_depth(4)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .any(|e| is_video_file(e.path()));

                if has_videos && scan_course_dir(&path, db).is_ok() {
                    scanned += 1;
                }
            }
        }
    }

    Ok(scanned)
}

pub fn default_courses_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dl) = dirs::download_dir() {
        roots.push(dl.join("OmniGet").join("Courses"));
        roots.push(dl.join("courses"));
        roots.push(dl);
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_position_and_title() {
        assert_eq!(
            extract_position_and_title("01 - Welcome.mp4", 99),
            (1, "Welcome".to_string())
        );
        assert_eq!(
            extract_position_and_title("02. Setup Environment.mkv", 99),
            (2, "Setup Environment".to_string())
        );
        assert_eq!(
            extract_position_and_title("[05] Functions and Closures.webm", 99),
            (5, "Functions and Closures".to_string())
        );
        assert_eq!(
            extract_position_and_title("Unnumbered Lesson.mp4", 7),
            (7, "Unnumbered Lesson".to_string())
        );
    }

    #[test]
    fn test_scan_course_directory_structure() {
        let dir = tempdir().expect("tempdir");
        let course_dir = dir.path().join("Complete Rust Course");
        let mod_dir = course_dir.join("01 - Getting Started");
        std::fs::create_dir_all(&mod_dir).expect("create_dir_all");

        // Create loose lesson and thumbnail in course root
        std::fs::write(course_dir.join("00 - Course Overview.mp4"), b"video").unwrap();
        std::fs::write(course_dir.join("cover.jpg"), b"cover").unwrap();

        // Create module lessons and subtitle
        std::fs::write(mod_dir.join("01 - Installation.mp4"), b"video").unwrap();
        std::fs::write(mod_dir.join("01 - Installation.vtt"), b"subtitle").unwrap();
        std::fs::write(mod_dir.join("02 - Hello World.mp4"), b"video").unwrap();

        let db = StudyDb::open_in_memory().expect("in-memory db");
        let cid = scan_course_dir(&course_dir, &db).expect("scan course");
        assert!(cid > 0);

        let detail = db
            .get_course_detail(cid)
            .expect("course detail")
            .expect("exists");

        assert_eq!(detail.course.title, "Complete Rust Course");
        assert!(detail.course.thumbnail_path.is_some());
        assert_eq!(detail.loose_lessons.len(), 1);
        assert_eq!(detail.loose_lessons[0].title, "Course Overview");

        assert_eq!(detail.modules.len(), 1);
        assert_eq!(detail.modules[0].title, "Getting Started");
        assert_eq!(detail.modules[0].lessons.len(), 2);
        assert_eq!(detail.modules[0].lessons[0].title, "Installation");
        assert!(detail.modules[0].lessons[0].subtitle_path.is_some());
        assert_eq!(detail.modules[0].lessons[1].title, "Hello World");

        let courses = db.list_courses().expect("list courses");
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].total_lessons, 3);
        assert_eq!(courses[0].completed_lessons, 0);
    }
}
