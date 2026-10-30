# OmniGet Domain Glossary

This document serves as the single source of truth for domain vocabulary across OmniGet. Terms defined here represent canonical concepts; avoid synonyms or divergent naming in code, tests, and documentation.

---

### Core Concepts

#### StudyPlugin
The native Rust plugin crate (`omniget-plugin-study`) statically linked into the OmniGet desktop application. Implements the `OmnigetPlugin` trait to provide local persistence, playback tracking, note indexing, and reading tools for downloaded media.

#### LessonProgress
The recorded playback and completion state for a single lecture or media item within a downloaded course. Tracks the current playback position in seconds, total duration, completion flag, and the last accessed timestamp.

#### StudyDatabase
The dedicated SQLite database (`study.db`) located in the user's application data directory (`core::paths::app_data_dir()`). Isolates study progress, notes, annotations, and flashcard decks from core download queue operations.

#### CatchAllDispatcher
The command handling safety mechanism within `StudyPlugin::handle_command`. When the frontend invokes a command string that is not yet fully implemented or is planned for a future milestone, the dispatcher logs the call at trace level and returns a safe empty JSON response (`Ok(json!({}))`) rather than raising an unhandled rejection error.
