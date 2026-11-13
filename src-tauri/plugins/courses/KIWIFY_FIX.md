# Kiwify normalized course content

## Root cause

The successful `viewer/clubs/{id}/content?caipirinha=true` response can contain
`data.all_courses`, `data.all_modules` and `data.all_lessons`, indexed by UUID.
The previous parser only searched `course.modules`, `content.modules`, `modules`
and `content`, so it returned `No modules found in course content response`
before requesting any lesson or starting a download.

The normalized payload is a graph. Each course's `modules` array and each
module's `lessons` array contain ordered references, rather than embedded
objects. The club ID in the course listing can also differ from the internal
course ID on modules and lessons.

## Behavior

- Resolve references from the normalized maps and preserve reference order.
  Exclude orphan records, deduplicate repeated references, and reject dangling
  references or conflicting parent/record IDs instead of silently truncating.
- Preserve the existing nested and legacy content formats, including a `data`
  wrapper. Keep all catalog records and check visible lesson candidates.
  Honor visibility/access restrictions and always use the authoritative
  lesson detail before saving content. A publication flag by itself neither
  grants nor denies access to media returned to the authenticated user.
- Carry the internal course ID into lesson and attachment requests.
- Fall back to the legacy content endpoint only for a missing/unsupported club
  route (404/405). Surface authentication, permission, rate limit, server and
  JSON failures. Do not turn a later pagination failure into a partial success.
- Reuse a validated API client for token login without panicking on an invalid
  header. Use a separate media client without the API bearer token for direct
  CDN requests. Do not include authenticated response bodies in API errors.
- Recheck the catalog on subsequent runs, continue processing attachments when
  a video already exists, and report failed items instead of marking a failed
  batch complete. Write local per-lesson JSON/text reports and return a partial
  result when any catalog entry is unavailable, failed or unprocessed; only a
  fully saved catalog receives `.complete`. Use a deterministic yt-dlp filename while preserving its
  actual container extension and literal percent signs in lesson titles.
- Resolve authentication before registering an active download, avoiding a
  stale active-download entry when no session is available.

## Automated validation

`api_tests.rs` uses synthetic fixtures only, including a 15-module/58-content
graph with mixed publication flags. All 58 accessible catalog candidates must
have their lesson details checked rather than being filtered by publication metadata.
It covers graph membership/order, permissions,
legacy payloads, malformed/dangling/cross-course references, HTTP fallback,
401/403/429/500 responses, invalid JSON, pagination and bearer isolation.
The downloader regression checks that failures remove an old success marker
and a successful retry can write one.

On the Windows validation machine:

- Courses plugin: 43 tests passed, including 20 new regressions.
- Other workspace crates: 1,689 passed; 63 explicitly ignored by the existing
  test configuration.
- Frontend: 142 passed. A local CRLF shebang in `scripts/lint-ui.mjs` caused
  two Vitest import failures until normalized to LF; its Git content is unchanged.
- Documentation verification harness: 60 passed.
- Clippy baseline gate passed: 121 existing workspace warnings, no new warnings.
- Windows executable compiled with the embedded production frontend.

The old catalog-dispatch test executed real logout/session handlers against the
developer's AppData. It is replaced with a download-payload boundary test that
fails before authentication, filesystem or network work; catalog support is
still checked separately. The full plugin suite was rerun and an existing local
session's hash was unchanged.

The repository's Windows CI excludes the Courses test executable because Tauri
requires the Common Controls v6 activation manifest. For this local run, the
Windows SDK `mt.exe` embedded that manifest into the generated test executable
before execution. No compiler, test assertion or platform security setting was
disabled.

## Opt-in live validation

Use the normal OmniGet interface to sign in personally. The example reads only
the session already saved by OmniGet and runs the same downloader as the desktop
command. It does not print account identifiers, course names, raw responses or
signed URLs.

From `src-tauri`:

```text
cargo run -p omniget-plugin-courses --example kiwify_validate -- --catalog
cargo run -p omniget-plugin-courses --example kiwify_validate -- --one OUTPUT_DIR
cargo run -p omniget-plugin-courses --example kiwify_validate -- --batch OUTPUT_DIR
```

Downloads require exactly one accessible candidate. If there are several, set
`OMNIGET_KIWIFY_COURSE_ID` locally to the intended listing ID. The `--one` mode
stops after processing the first lesson without marking the course complete;
check the resulting media separately with FFprobe and a full FFmpeg decode.
On Windows the standalone example needs the same Common Controls manifest as
the test executable. Batch validation exits with code 2 for a partial result
and writes `download-report.json` and `download-report.txt` inside the course
directory. Exit code 0 means the entire catalog was saved; other failures exit 1.

Live validation confirmed authenticated listing, the normalized catalog and a
successfully downloaded and fully decoded H.264/AAC MP4. Follow-up auditing checked
every catalog lesson: some returned `locked=true` without media. The revised runner
processed the entire catalog, reused saved media and wrote a partial report with
zero unprocessed entries; it removed the previous success marker. A completed
available subset is not described as a complete course. Private course files,
credentials, response fixtures and signed media URLs are not part of this change.

The user subsequently confirmed in the browser that the missing lessons are
scheduled for future release. This matches the authoritative locked responses
and closes the reported discrepancy: currently available content was saved,
while unreleased lessons remain correctly unavailable.

## Limits

This change does not renew expired sessions or grant access to scheduled,
locked, expired or DRM-protected content. The platform remains the
authority for access. Existing nonempty local files are reused as before; the
runtime does not perform a full decode of every existing file. The API is
undocumented, so further incompatible schema changes will require new evidence
and corresponding regression fixtures.
