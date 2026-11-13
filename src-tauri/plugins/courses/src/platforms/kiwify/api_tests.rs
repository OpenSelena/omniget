use super::*;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Entirely synthetic: no authenticated response, account or course data.
fn normalized_fixture() -> Value {
    let mut modules = serde_json::Map::new();
    let mut lessons = serde_json::Map::new();
    let mut module_refs = Vec::new();
    let mut content_index = 0;
    for mi in 0..15 {
        let module_id = format!("module-{}", 15 - mi);
        module_refs.push(module_id.clone());
        let mut refs = Vec::new();
        for li in 0..if mi == 14 { 2 } else { 4 } {
            let lesson_id = format!("lesson-{}-{}", mi, li);
            refs.push(lesson_id.clone());
            lessons.insert(
                lesson_id.clone(),
                json!({
                    "id": lesson_id, "name": format!("Synthetic lesson {}", content_index),
                    "course_id": "internal-course", "module_id": module_id,
                    "published": content_index < 21,
                    "user_data": {"visible": true, "expired": false}
                }),
            );
            content_index += 1;
        }
        modules.insert(
            module_id.clone(),
            json!({
                "id": module_id, "name": format!("Synthetic module {}", mi),
                "course_id": "internal-course", "lessons": refs,
                "user_data": {"visible": true}
            }),
        );
    }
    json!({"data": {
        "all_courses": {"internal-course": {"id": "internal-course", "modules": module_refs}},
        "all_modules": modules, "all_lessons": lessons
    }})
}

#[test]
fn normalized_graph_resolves_15_modules_58_contents_in_reference_order() {
    let payload = normalized_fixture();
    let modules = parse_course_modules(&payload).unwrap();
    assert_eq!(modules.len(), 15);
    assert_eq!(modules.iter().map(|m| m.lessons.len()).sum::<usize>(), 58);
    assert_eq!(
        modules
            .iter()
            .flat_map(|m| &m.lessons)
            .filter(|l| l.available)
            .count(),
        21
    );
    assert_eq!(modules[0].id, "module-15");
    assert_eq!(modules[14].id, "module-1");
    assert_eq!(modules[0].lessons[0].id, "lesson-0-0");
    assert!(modules
        .iter()
        .flat_map(|m| &m.lessons)
        .all(|l| l.course_id.as_deref() == Some("internal-course")));
    assert_eq!(parse_course_modules(&payload["data"]).unwrap().len(), 15);
}

#[test]
fn normalized_graph_ignores_orphans_and_deduplicates_references() {
    let mut payload = normalized_fixture();
    payload["data"]["all_modules"]["orphan"] = json!({"id": "orphan", "lessons": []});
    payload["data"]["all_courses"]["internal-course"]["modules"]
        .as_array_mut()
        .unwrap()
        .push(json!("module-15"));
    payload["data"]["all_modules"]["module-15"]["lessons"]
        .as_array_mut()
        .unwrap()
        .push(json!("lesson-0-0"));
    let modules = parse_course_modules(&payload).unwrap();
    assert_eq!(modules.len(), 15);
    assert_eq!(modules[0].lessons.len(), 4);
}

#[test]
fn normalized_graph_rejects_dangling_and_cross_course_references() {
    for (field, value) in [
        ("module_id", "wrong-module"),
        ("course_id", "wrong-course"),
        ("id", "wrong-id"),
    ] {
        let mut payload = normalized_fixture();
        payload["data"]["all_lessons"]["lesson-0-0"][field] = json!(value);
        assert!(parse_course_modules(&payload)
            .unwrap_err()
            .to_string()
            .contains("mismatched"));
    }
    let mut payload = normalized_fixture();
    payload["data"]["all_lessons"]
        .as_object_mut()
        .unwrap()
        .remove("lesson-0-0");
    assert!(parse_course_modules(&payload)
        .unwrap_err()
        .to_string()
        .contains("unresolved lesson"));
}

#[test]
fn duplicate_module_in_another_course_cannot_bypass_parent_validation() {
    let mut payload = normalized_fixture();
    payload["data"]["all_courses"]["second-course"] = json!({
        "id": "second-course", "modules": ["module-15"]
    });
    assert!(parse_course_modules(&payload)
        .unwrap_err()
        .to_string()
        .contains("mismatched course_id"));
}

#[test]
fn normalized_graph_rejects_wrong_shapes_and_accepts_empty_catalog() {
    for field in ["all_courses", "all_modules", "all_lessons"] {
        let mut payload = normalized_fixture();
        payload["data"][field] = json!([]);
        assert!(parse_course_modules(&payload).is_err());
    }
    assert!(parse_course_modules(
        &json!({"data": {"all_courses": {}, "all_modules": {}, "all_lessons": {}}})
    )
    .unwrap()
    .is_empty());
    assert!(
        parse_course_modules(&json!({"error": "private response must not be echoed"})).is_err()
    );
}

#[test]
fn permission_flags_propagate_without_losing_catalog_entries() {
    for (path, value) in [
        (
            vec![
                "data",
                "all_courses",
                "internal-course",
                "user_data",
                "visible",
            ],
            json!(false),
        ),
        (
            vec!["data", "all_modules", "module-15", "user_data", "visible"],
            json!(false),
        ),
        (
            vec!["data", "all_lessons", "lesson-0-0", "locked"],
            json!(true),
        ),
        (
            vec!["data", "all_lessons", "lesson-0-0", "user_data", "expired"],
            json!(true),
        ),
    ] {
        let mut payload = normalized_fixture();
        let mut target = &mut payload;
        for key in path {
            target = &mut target[key];
        }
        *target = value;
        let modules = parse_course_modules(&payload).unwrap();
        assert_eq!(modules.len(), 15);
        assert!(!modules[0].lessons[0].available);
    }
}

#[test]
fn legacy_maps_and_wrappers_keep_ids_order_and_permissions() {
    let payload = json!({"data": {"modules": {
        "module-b": {"name": "Later", "order": 2, "lessons": {}},
        "module-a": {"name": "First", "order": 1, "classes": {
            "lesson-b": {"title": "Later", "order": 2, "expired": true},
            "lesson-a": {"title": "First", "order": 1}
        }}
    }}});
    let modules = parse_course_modules(&payload).unwrap();
    assert_eq!(modules[0].id, "module-a");
    assert_eq!(modules[0].lessons[0].id, "lesson-a");
    assert!(modules[0].lessons[0].available);
    assert!(!modules[0].lessons[1].available);
}

#[test]
fn locked_lesson_detail_stays_unavailable_even_with_media_fields() {
    let detail = parse_lesson_detail(
        &json!({"data": {"lesson": {
            "id": "lesson", "locked": true, "video": {"stream_link": "/synthetic/master.m3u8"}
        }}}),
        "fallback",
    );
    assert!(!detail.available);
    assert_eq!(detail.id, "lesson");
}

async fn mock_server(
    responses: Vec<(u16, String)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut data = Vec::new();
            while !data.ends_with(b"\r\n\r\n") {
                let mut byte = [0u8; 1];
                socket.read_exact(&mut byte).await.unwrap();
                data.push(byte[0]);
                assert!(data.len() < 16384);
            }
            requests.push(String::from_utf8(data).unwrap());
            let reply = format!("HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", status, body.len(), body);
            socket.write_all(reply.as_bytes()).await.unwrap();
        }
        requests
    });
    (base, task)
}

fn mock_session() -> KiwifySession {
    KiwifySession {
        token: "synthetic-token".into(),
        email: String::new(),
        client: build_client("synthetic-token").unwrap(),
    }
}

#[tokio::test]
async fn http_content_to_download_plan_resolves_normalized_club_graph() {
    let (base, task) = mock_server(vec![(200, normalized_fixture().to_string())]).await;
    let modules = get_course_content_at(&mock_session(), "club-id", &base)
        .await
        .unwrap();
    assert_eq!(modules.len(), 15);
    assert_eq!(
        modules
            .iter()
            .flat_map(|m| &m.lessons)
            .filter(|l| l.available)
            .count(),
        21
    );
    let requests = task.await.unwrap();
    assert!(requests[0].starts_with("GET /clubs/club-id/content?caipirinha=true "));
}

#[tokio::test]
async fn http_legacy_fallback_only_for_missing_route() {
    let payload = json!({"course": {"modules": []}});
    let (base, task) = mock_server(vec![(404, "{}".into()), (200, payload.to_string())]).await;
    assert!(get_course_content_at(&mock_session(), "course", &base)
        .await
        .unwrap()
        .is_empty());
    let requests = task.await.unwrap();
    assert!(requests[1].starts_with("GET /courses/course "));
}

#[tokio::test]
async fn http_auth_permission_rate_limit_and_server_errors_do_not_fallback_or_leak_body() {
    for status in [401, 403, 429, 500] {
        let (base, task) = mock_server(vec![(status, "SECRET-RESPONSE".into())]).await;
        let error = get_course_content_at(&mock_session(), "course", &base)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(&status.to_string()));
        assert!(!error.contains("SECRET"));
        assert!(!error.contains(&base));
        assert_eq!(task.await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn http_malformed_success_response_is_not_retried_as_legacy() {
    let (base, task) = mock_server(vec![(200, "not json".into())]).await;
    assert!(get_course_content_at(&mock_session(), "course", &base)
        .await
        .unwrap_err()
        .to_string()
        .contains("Invalid JSON"));
    assert_eq!(task.await.unwrap().len(), 1);
}

#[tokio::test]
async fn http_pagination_loads_all_pages_and_rejects_partial_success() {
    let first = json!({"count": 2, "page_size": 1, "courses": [{"course_info": {"id": "course-a", "name": "A"}}]});
    let second = json!({"count": 2, "page_size": 1, "courses": [{"course_info": {"id": "course-b", "name": "B"}}]});
    let (base, task) = mock_server(vec![(200, first.to_string()), (200, second.to_string())]).await;
    assert_eq!(
        list_courses_at(&mock_session(), &base).await.unwrap().len(),
        2
    );
    assert!(task.await.unwrap()[1].contains("page=2&archived=false"));
    let (base, task) = mock_server(vec![(200, first.to_string()), (503, "private".into())]).await;
    assert!(list_courses_at(&mock_session(), &base).await.is_err());
    task.await.unwrap();
}

#[tokio::test]
async fn http_repeated_pagination_page_is_an_error_instead_of_silent_truncation() {
    let page = json!({"count": 2, "page_size": 1, "courses": [{"id": "a"}]}).to_string();
    let (base, task) = mock_server(vec![(200, page.clone()), (200, page)]).await;
    assert!(list_courses_at(&mock_session(), &base)
        .await
        .unwrap_err()
        .to_string()
        .contains("repeated"));
    task.await.unwrap();
}

#[tokio::test]
async fn media_client_never_sends_api_authorization() {
    let (base, task) = mock_server(vec![(200, "{}".into())]).await;
    build_media_client()
        .unwrap()
        .get(base)
        .send()
        .await
        .unwrap();
    let requests = task.await.unwrap();
    assert!(!requests[0].to_ascii_lowercase().contains("authorization:"));
    assert!(requests[0].contains("referer: https://admin.kiwify.com.br/"));
}

#[test]
fn invalid_token_header_returns_error_without_panicking() {
    assert!(build_client("invalid\nheader").is_err());
}
