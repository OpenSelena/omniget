use serde_json::Value;
use std::path::PathBuf;

use crate::db::StudyDb;
use crate::scanner;

fn extract_id(args: &Value, keys: &[&str]) -> Option<i64> {
    for &k in keys {
        if let Some(v) = args.get(k) {
            if let Some(n) = v.as_i64() {
                return Some(n);
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

pub async fn dispatch(db: &StudyDb, command: &str, args: Value) -> Result<Value, String> {
    match command {
        "study:courses:list" => {
            let courses = db.list_courses().map_err(|e| e.to_string())?;
            // If empty, attempt a scan of default download roots
            if courses.is_empty() {
                for root in scanner::default_courses_roots() {
                    let _ = scanner::scan_root_dir(&root, db);
                }
                let res = db.list_courses().map_err(|e| e.to_string())?;
                return serde_json::to_value(res).map_err(|e| e.to_string());
            }
            Ok(serde_json::to_value(courses).map_err(|e| e.to_string())?)
        }

        "study:courses:get" => {
            let course_id = extract_id(&args, &["courseId", "id", "course_id"])
                .ok_or_else(|| "missing courseId".to_string())?;
            let detail = db.get_course_detail(course_id).map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(detail).map_err(|e| e.to_string())?)
        }

        "study:lesson:get" => {
            let lesson_id = extract_id(&args, &["lessonId", "id", "lesson_id"])
                .ok_or_else(|| "missing lessonId".to_string())?;
            let lesson = db.get_lesson(lesson_id).map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(lesson).map_err(|e| e.to_string())?)
        }

        "study:player:time_changed" => {
            let lesson_id = extract_id(&args, &["lessonId", "id", "lesson_id"])
                .ok_or_else(|| "missing lessonId".to_string())?;
            let time_ms = args.get("timeMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let duration_ms = args.get("durationMs").and_then(|v| v.as_f64());
            let course_id = extract_id(&args, &["courseId", "course_id"]);

            let pos_secs = time_ms / 1000.0;
            let dur_secs = duration_ms.map(|d| d / 1000.0);

            db.save_progress(lesson_id, course_id, pos_secs, dur_secs, None)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true }))
        }

        "study:player:seek" => {
            let lesson_id = extract_id(&args, &["lessonId", "id", "lesson_id"])
                .ok_or_else(|| "missing lessonId".to_string())?;
            let to_ms = args.get("toMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
            db.save_progress(lesson_id, None, to_ms / 1000.0, None, None)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true }))
        }

        "study:progress:mark_lesson" => {
            let lesson_id = extract_id(&args, &["lessonId", "id", "lesson_id"])
                .ok_or_else(|| "missing lessonId".to_string())?;
            let seconds = args.get("seconds").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let completed = args
                .get("completed")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            db.mark_lesson_completed(lesson_id, seconds, completed)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true }))
        }

        "study:progress:mark_lessons_bulk" => {
            let completed = args
                .get("completed")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let mut ids = Vec::new();
            if let Some(arr) = args.get("lessonIds").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(n) = item.as_i64() {
                        ids.push(n);
                    }
                }
            }
            db.mark_lessons_bulk(&ids, completed)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "count": ids.len() }))
        }

        "study:player:speed:get" => {
            let course_id = extract_id(&args, &["courseId", "id", "course_id"])
                .ok_or_else(|| "missing courseId".to_string())?;
            let speed = db
                .get_playback_speed(course_id)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "course_id": course_id,
                "playback_speed": speed
            }))
        }

        "study:player:speed:set" => {
            let course_id = extract_id(&args, &["courseId", "id", "course_id"])
                .ok_or_else(|| "missing courseId".to_string())?;
            let speed = args.get("speed").and_then(|v| v.as_f64());
            db.set_playback_speed(course_id, speed)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "course_id": course_id,
                "playback_speed": speed
            }))
        }

        "study:courses:recents:touch" => {
            let course_id = extract_id(&args, &["courseId", "id", "course_id"])
                .ok_or_else(|| "missing courseId".to_string())?;
            db.touch_course_recent(course_id)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true }))
        }

        "study:courses:recents:list" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let recents = db.list_recents(limit).map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(recents).map_err(|e| e.to_string())?)
        }

        "study:courses:toggle_favorite" => {
            let course_id = extract_id(&args, &["courseId", "id", "course_id"])
                .ok_or_else(|| "missing courseId".to_string())?;
            let favorite = db.toggle_favorite(course_id).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "id": course_id,
                "favorite": favorite
            }))
        }

        "study:courses:scan" | "study:rescan" => {
            let mut total_scanned = 0;
            for root in scanner::default_courses_roots() {
                if let Ok(c) = scanner::scan_root_dir(&root, db) {
                    total_scanned += c;
                }
            }
            Ok(serde_json::json!({ "ok": true, "scanned": total_scanned }))
        }

        "study:library:telegram:default_output_dir" => {
            let dir = dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .to_string_lossy()
                .to_string();
            Ok(serde_json::json!({ "path": dir }))
        }

        "study:settings:get" => Ok(serde_json::json!({
            "player": {
                "default_rate": 1.0
            }
        })),

        "study:settings:save" => Ok(args
            .get("settings")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}))),

        "study:gamification:state" => Ok(serde_json::json!({
            "xp": 0,
            "level": 1,
            "streak_days": 0
        })),

        "study:player:context" => Ok(serde_json::json!({
            "thumbnails": [],
            "audio_tracks": []
        })),

        "study:player:detect_skip_gaps" | "study:player:skip_gaps:get" => {
            Ok(serde_json::json!(null))
        }

        "study:player:next_lesson" => Ok(serde_json::json!(null)),

        "study:lesson:thumbnails" => Ok(serde_json::json!([])),

        // Safe catch-all fallback
        other => {
            tracing::debug!(
                "[study] unhandled command fallback: '{}' with args: {:?}",
                other,
                args
            );
            Ok(serde_json::json!({}))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_commands_dispatch_lifecycle() {
        let db = StudyDb::open_in_memory().expect("in-memory db");
        let cid = db
            .upsert_course("Test Course", "/courses/test", None)
            .expect("course upsert");
        let lid = db
            .upsert_lesson(
                cid,
                None,
                1,
                "01 Video",
                "/courses/test/01.mp4",
                None,
                Some(60_000),
            )
            .expect("lesson upsert");

        // 1. Test study:courses:list
        let list_res = dispatch(&db, "study:courses:list", serde_json::json!({}))
            .await
            .expect("study:courses:list");
        assert!(!list_res.as_array().unwrap().is_empty());

        // 2. Test study:courses:get
        let get_res = dispatch(
            &db,
            "study:courses:get",
            serde_json::json!({ "courseId": cid }),
        )
        .await
        .expect("study:courses:get");
        assert_eq!(get_res["course"]["title"], "Test Course");

        // 3. Test study:lesson:get
        let lesson_res = dispatch(
            &db,
            "study:lesson:get",
            serde_json::json!({ "lessonId": lid }),
        )
        .await
        .expect("study:lesson:get");
        assert_eq!(lesson_res["title"], "01 Video");

        // 4. Test study:player:time_changed
        let time_res = dispatch(
            &db,
            "study:player:time_changed",
            serde_json::json!({
                "lessonId": lid,
                "timeMs": 25000.0,
                "durationMs": 60000.0
            }),
        )
        .await
        .expect("time_changed");
        assert_eq!(time_res["ok"], true);

        let p = db.get_progress(lid).unwrap().unwrap();
        assert_eq!(p.position_secs, 25.0);

        // 5. Test study:progress:mark_lesson
        let mark_res = dispatch(
            &db,
            "study:progress:mark_lesson",
            serde_json::json!({
                "lessonId": lid,
                "seconds": 60.0,
                "completed": true
            }),
        )
        .await
        .expect("mark_lesson");
        assert_eq!(mark_res["ok"], true);

        let p_after = db.get_progress(lid).unwrap().unwrap();
        assert!(p_after.completed);

        // 6. Test playback speed get/set
        dispatch(
            &db,
            "study:player:speed:set",
            serde_json::json!({ "courseId": cid, "speed": 1.75 }),
        )
        .await
        .expect("speed:set");

        let speed_res = dispatch(
            &db,
            "study:player:speed:get",
            serde_json::json!({ "courseId": cid }),
        )
        .await
        .expect("speed:get");
        assert_eq!(speed_res["playback_speed"], 1.75);

        // 7. Test toggle favorite
        let fav_res = dispatch(
            &db,
            "study:courses:toggle_favorite",
            serde_json::json!({ "courseId": cid }),
        )
        .await
        .expect("toggle_favorite");
        assert_eq!(fav_res["favorite"], true);

        // 8. Test recents
        dispatch(
            &db,
            "study:courses:recents:touch",
            serde_json::json!({ "courseId": cid }),
        )
        .await
        .expect("recents:touch");

        let recents_res = dispatch(
            &db,
            "study:courses:recents:list",
            serde_json::json!({ "limit": 5 }),
        )
        .await
        .expect("recents:list");
        assert_eq!(recents_res.as_array().unwrap().len(), 1);

        // 9. Test default output dir
        let dir_res = dispatch(
            &db,
            "study:library:telegram:default_output_dir",
            serde_json::json!({}),
        )
        .await
        .expect("default_output_dir");
        assert!(dir_res["path"].as_str().is_some());
    }
}
