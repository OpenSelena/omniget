//! Opt-in live validation using only the session already saved by OmniGet.
//! No credentials, course names, identifiers, content or signed URLs are printed.
//! Run with --catalog, --one OUTPUT_DIR, or --batch OUTPUT_DIR.
//! If several courses are accessible, select one with OMNIGET_KIWIFY_COURSE_ID.
use omniget_plugin_courses::platforms::kiwify::{api, downloader};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    if validate().await.is_err() {
        // Download engine errors may contain signed URLs. Keep them private.
        eprintln!("Kiwify validation failed. Check sign-in, course access and the local output directory.");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

async fn validate() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mode = args.first().map(String::as_str).unwrap_or("--catalog");
    anyhow::ensure!(
        ["--catalog", "--one", "--batch"].contains(&mode),
        "Use --catalog, --one OUTPUT_DIR, or --batch OUTPUT_DIR"
    );
    let session = api::load_session()
        .await?
        .ok_or_else(|| anyhow::anyhow!("No saved Kiwify session. Sign in using OmniGet first."))?;
    anyhow::ensure!(
        api::validate_token(&session).await?,
        "Saved session has expired. Sign in using OmniGet first."
    );
    let courses = api::list_courses(&session).await?;
    println!("session_valid=true listed_courses={}", courses.len());
    let mut selected = None;
    let mut candidates = 0;
    let requested_id = std::env::var("OMNIGET_KIWIFY_COURSE_ID").ok();
    for (index, course) in courses.iter().enumerate() {
        let modules = api::get_course_content(&session, &course.id).await?;
        let contents = modules.iter().map(|m| m.lessons.len()).sum::<usize>();
        let available = modules
            .iter()
            .flat_map(|m| &m.lessons)
            .filter(|l| l.available)
            .count();
        println!(
            "course_index={} modules={} contents={} eligible={} unavailable={}",
            index,
            modules.len(),
            contents,
            available,
            contents - available
        );
        if available > 0 && requested_id.as_ref().is_none_or(|id| id == &course.id) {
            candidates += 1;
            selected = Some(course);
        }
    }
    if mode == "--catalog" {
        return Ok(());
    }
    anyhow::ensure!(
        candidates == 1,
        "Select exactly one accessible course using OMNIGET_KIWIFY_COURSE_ID"
    );
    let output = args
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("Output directory required"))?;
    let course =
        selected.ok_or_else(|| anyhow::anyhow!("No course has accessible published lessons"))?;
    let cancel = CancellationToken::new();
    let mut processed = 0;
    let mut failed_items = 0;
    let result = downloader::download_course(&session, course, output, cancel.clone(), |p| {
        processed = p.completed_lessons;
        failed_items = p.failed_items;
        println!(
            "progress processed={} total={} modules={} bytes={}",
            p.completed_lessons, p.total_lessons, p.total_modules, p.downloaded_bytes
        );
        if mode == "--one" && processed == 1 {
            cancel.cancel();
        }
    })
    .await;
    if mode == "--one" && processed == 1 && cancel.is_cancelled() {
        anyhow::ensure!(
            failed_items == 0,
            "The first lesson failed; single lesson validation unsuccessful"
        );
        anyhow::ensure!(
            result
                .as_ref()
                .is_err_and(|e| e.to_string().contains("cancelled")),
            "Single lesson validation did not stop as expected"
        );
        println!("single_lesson_processed=true cancelled_before_next=true");
        return Ok(());
    }
    result?;
    println!("batch_success=true processed={}", processed);
    Ok(())
}
