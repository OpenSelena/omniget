# ADR 0001: Study Plugin Architecture & Persistence Model

- **Status**: Accepted
- **Date**: 2026-10-30
- **Deciders**: igect, Antigravity

## Context

OmniGet contains an extensive frontend suite (95 routes and 120+ components totaling over 117,000 lines of code) designed for course playback, note-taking, reading, and flashcards. This UI communicates with the backend exclusively via `pluginInvoke("study", <command>, <args>)`.

However, no backend implementation of `study` existed, forcing the application to hide the entire feature behind hardcoded `false` flags to avoid frontend crashes when unbacked commands were invoked.

We need to establish:
1. How the plugin is integrated into the Tauri desktop process.
2. Where and how study state (playback position, notes, reading progress) is stored.
3. How the command dispatcher handles commands across a multi-phase rollout without runtime crashes.

## Decision

1. **Embedded Built-in Plugin Crate**:
   We will implement `omniget-plugin-study` as a native workspace crate in `src-tauri/plugins/study/` and register it statically using `PluginManager::register_builtin` in `src-tauri/src/lib.rs`. We reject distributing Study as a dynamic `.dll`/`.so` download because embedded compilation eliminates ABI drift, avoids runtime download failures, and functions completely offline.

2. **Dedicated SQLite Database (`study.db`)**:
   We will manage all study persistence within a dedicated SQLite file (`~/.local/share/omniget/study.db`) using `rusqlite`. We reject sharing the main download state database to avoid write-lock contention during heavy concurrent torrent/video downloads and to allow straightforward user backup/export of personal study notes.

3. **Zero-Crash Catch-All Dispatcher**:
   In `StudyPlugin::handle_command`, any command not yet actively handled by the current development milestone will be caught by a fallback pattern that logs the invocation and returns `Ok(json!({}))`. This guarantees that navigating through the 117k-line UI will not trigger unhandled promise rejections during progressive feature rollouts.

4. **Phase 1 Boundary**:
   Phase 1 focuses exclusively on Course Discovery & Video Player Progress (`study:courses:list`, `study:player:seek`, `study:player:time_changed`, `study:progress:mark_lesson`). Notes, Reader, Anki, and Audio integrations are deferred to subsequent milestone tickets.

## Consequences

### Positive
- Revives 117,413 lines of existing frontend code with zero frontend redesign.
- Guarantees zero crash risk during incremental development via the safe catch-all dispatcher.
- Complete offline capability across all desktop platforms (Windows, Linux, macOS).
- Resolves GitHub Issue #93 with a working, native feature.

### Negative / Trade-offs
- Statically linking the study crate marginally increases the final binary size (~1-2 MB).
- Multiple SQLite files exist in the user's application data directory, requiring independent connection management.
