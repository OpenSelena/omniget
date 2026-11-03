//! Allowlist for user-supplied yt-dlp flags.
//!
//! Users can add their own yt-dlp flags (Settings → "extra yt-dlp flags" and the
//! "custom arguments" download). A blocklist is not enough: yt-dlp exposes several
//! ways to run arbitrary programs (`--exec`, `--use-postprocessor Exec:...`,
//! `--netrc-cmd`, `--plugin-dirs`, ...) and accepts any unambiguous abbreviation of
//! a long option (`--netrc-c` == `--netrc-cmd`). This module only lets through a
//! curated set of options that cannot start a process, expands abbreviations to
//! the canonical option name so yt-dlp never re-interprets them, and drops the
//! value that belongs to a rejected option instead of leaving it behind as a stray
//! positional argument (which yt-dlp would treat as a URL).

/// How an allowed option consumes its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arity {
    /// Boolean switch, no value.
    Flag,
    /// Takes exactly one value (`--opt value` or `--opt=value`).
    Value,
}

use Arity::{Flag, Value};

/// Long options that are safe to forward. None of them can execute a program,
/// load code, read a config file or change where OmniGet writes files.
const ALLOWED_LONG: &[(&str, Arity)] = &[
    // Format selection
    ("--format", Value),
    ("--format-sort", Value),
    ("--format-sort-force", Flag),
    ("--no-format-sort-force", Flag),
    ("--merge-output-format", Value),
    ("--remux-video", Value),
    ("--recode-video", Value),
    ("--prefer-free-formats", Flag),
    ("--no-prefer-free-formats", Flag),
    ("--check-formats", Flag),
    ("--no-check-formats", Flag),
    ("--video-multistreams", Flag),
    ("--no-video-multistreams", Flag),
    ("--audio-multistreams", Flag),
    ("--no-audio-multistreams", Flag),
    // Audio
    ("--extract-audio", Flag),
    ("--audio-format", Value),
    ("--audio-quality", Value),
    ("--keep-video", Flag),
    ("--no-keep-video", Flag),
    // Subtitles, thumbnails, metadata
    ("--write-subs", Flag),
    ("--no-write-subs", Flag),
    ("--write-auto-subs", Flag),
    ("--no-write-auto-subs", Flag),
    ("--sub-langs", Value),
    ("--sub-format", Value),
    ("--convert-subs", Value),
    ("--embed-subs", Flag),
    ("--no-embed-subs", Flag),
    ("--write-thumbnail", Flag),
    ("--no-write-thumbnail", Flag),
    ("--convert-thumbnails", Value),
    ("--embed-thumbnail", Flag),
    ("--no-embed-thumbnail", Flag),
    ("--embed-metadata", Flag),
    ("--no-embed-metadata", Flag),
    ("--embed-chapters", Flag),
    ("--no-embed-chapters", Flag),
    ("--write-description", Flag),
    ("--write-info-json", Flag),
    ("--no-write-info-json", Flag),
    ("--parse-metadata", Value),
    ("--replace-in-metadata", Value),
    ("--xattrs", Flag),
    // SponsorBlock, sections, chapters
    ("--sponsorblock-remove", Value),
    ("--sponsorblock-mark", Value),
    ("--no-sponsorblock", Flag),
    ("--download-sections", Value),
    ("--force-keyframes-at-cuts", Flag),
    ("--no-force-keyframes-at-cuts", Flag),
    ("--split-chapters", Flag),
    ("--no-split-chapters", Flag),
    // Network
    ("--limit-rate", Value),
    ("--throttled-rate", Value),
    ("--retries", Value),
    ("--fragment-retries", Value),
    ("--extractor-retries", Value),
    ("--retry-sleep", Value),
    ("--concurrent-fragments", Value),
    ("--http-chunk-size", Value),
    ("--sleep-interval", Value),
    ("--min-sleep-interval", Value),
    ("--max-sleep-interval", Value),
    ("--sleep-requests", Value),
    ("--sleep-subtitles", Value),
    ("--socket-timeout", Value),
    ("--source-address", Value),
    ("--force-ipv4", Flag),
    ("--force-ipv6", Flag),
    ("--proxy", Value),
    ("--geo-bypass", Flag),
    ("--no-geo-bypass", Flag),
    ("--geo-bypass-country", Value),
    ("--xff", Value),
    ("--impersonate", Value),
    ("--user-agent", Value),
    ("--referer", Value),
    ("--add-headers", Value),
    ("--cookies-from-browser", Value),
    ("--no-cookies-from-browser", Flag),
    ("--legacy-server-connect", Flag),
    ("--no-check-certificates", Flag),
    // Extractors
    ("--extractor-args", Value),
    ("--js-runtimes", Value),
    ("--live-from-start", Flag),
    ("--no-live-from-start", Flag),
    ("--wait-for-video", Value),
    ("--no-wait-for-video", Flag),
    ("--hls-use-mpegts", Flag),
    ("--no-hls-use-mpegts", Flag),
    ("--mark-watched", Flag),
    ("--no-mark-watched", Flag),
    // Selection
    ("--no-playlist", Flag),
    ("--yes-playlist", Flag),
    ("--playlist-items", Value),
    ("--match-filters", Value),
    ("--no-match-filters", Flag),
    ("--age-limit", Value),
    ("--min-filesize", Value),
    ("--max-filesize", Value),
    ("--date", Value),
    ("--datebefore", Value),
    ("--dateafter", Value),
    ("--max-downloads", Value),
    // Filesystem behaviour (never the output location)
    ("--no-part", Flag),
    ("--part", Flag),
    ("--no-mtime", Flag),
    ("--mtime", Flag),
    ("--no-overwrites", Flag),
    ("--force-overwrites", Flag),
    ("--no-force-overwrites", Flag),
    ("--continue", Flag),
    ("--no-continue", Flag),
    ("--restrict-filenames", Flag),
    ("--no-restrict-filenames", Flag),
    ("--windows-filenames", Flag),
    ("--no-windows-filenames", Flag),
    ("--trim-filenames", Value),
    // Verbosity / error handling
    ("--verbose", Flag),
    ("--quiet", Flag),
    ("--no-warnings", Flag),
    ("--ignore-errors", Flag),
    ("--abort-on-error", Flag),
    ("--no-abort-on-error", Flag),
];

/// Single-dash aliases mapped to their long form.
const SHORT_ALIASES: &[(&str, &str)] = &[
    ("-f", "--format"),
    ("-S", "--format-sort"),
    ("-x", "--extract-audio"),
    ("-k", "--keep-video"),
    ("-r", "--limit-rate"),
    ("-R", "--retries"),
    ("-N", "--concurrent-fragments"),
    ("-I", "--playlist-items"),
    ("-w", "--no-overwrites"),
    ("-c", "--continue"),
    ("-i", "--ignore-errors"),
    ("-v", "--verbose"),
    ("-q", "--quiet"),
    ("-4", "--force-ipv4"),
    ("-6", "--force-ipv6"),
];

/// Options that can run code or read arbitrary config. An abbreviation that could
/// stand for one of these is always rejected, even if it would also match an
/// allowed option.
const DANGEROUS_LONG: &[&str] = &[
    "--exec",
    "--exec-before-download",
    "--no-exec",
    "--use-postprocessor",
    "--postprocessor-args",
    "--external-downloader",
    "--external-downloader-args",
    "--downloader",
    "--downloader-args",
    "--netrc-cmd",
    "--netrc-location",
    "--plugin-dirs",
    "--config-locations",
    "--config-location",
    "--alias",
    "--print-to-file",
    "--batch-file",
    "--load-info-json",
    "--ffmpeg-location",
    "--output",
    "--paths",
    "--cookies",
    "--download-archive",
    "--cache-dir",
    "--update-to",
    "--update",
];

/// Options that make yt-dlp start a program, load code or read another config.
/// Code-generated (internal) flags are checked against this narrower list, since
/// OmniGet itself legitimately passes `--cookies`, `--ffmpeg-location`, `--paths`.
const EXEC_LONG: &[&str] = &[
    "--exec",
    "--exec-before-download",
    "--use-postprocessor",
    "--external-downloader",
    "--external-downloader-args",
    "--downloader",
    "--downloader-args",
    "--netrc-cmd",
    "--plugin-dirs",
    "--config-locations",
    "--config-location",
    "--alias",
    "--print-to-file",
    "--batch-file",
];

/// JavaScript runtimes yt-dlp knows about; `--js-runtimes` may point at one of
/// these binaries but never at an arbitrary executable.
const JS_RUNTIME_NAMES: &[&str] = &["deno", "node", "bun", "quickjs"];
const JS_RUNTIME_BINARIES: &[&str] = &[
    "deno",
    "deno.exe",
    "node",
    "node.exe",
    "bun",
    "bun.exe",
    "qjs",
    "qjs.exe",
    "quickjs",
    "quickjs.exe",
];

/// Result of filtering a list of user tokens.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FilteredFlags {
    /// Canonical tokens safe to hand to yt-dlp.
    pub accepted: Vec<String>,
    /// The original tokens that were refused (for logs and error messages).
    pub rejected: Vec<String>,
}

fn lookup_exact(name: &str) -> Option<(&'static str, Arity)> {
    ALLOWED_LONG
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, a)| (*n, *a))
}

/// Resolve a long option (possibly abbreviated) to its canonical allowed name.
fn resolve_long(name: &str) -> Option<(&'static str, Arity)> {
    if !name.starts_with("--") || name.len() < 3 {
        return None;
    }
    if DANGEROUS_LONG.contains(&name) {
        return None;
    }
    if let Some(hit) = lookup_exact(name) {
        return Some(hit);
    }
    // Abbreviation: refuse anything that could also mean a dangerous option.
    if DANGEROUS_LONG.iter().any(|d| d.starts_with(name)) {
        return None;
    }
    let mut matches = ALLOWED_LONG.iter().filter(|(n, _)| n.starts_with(name));
    let first = matches.next()?;
    if matches.next().is_some() {
        return None; // ambiguous
    }
    Some((first.0, first.1))
}

fn resolve_option(name: &str) -> Option<(&'static str, Arity)> {
    if name.starts_with("--") {
        return resolve_long(name);
    }
    SHORT_ALIASES
        .iter()
        .find(|(short, _)| *short == name)
        .and_then(|(_, long)| lookup_exact(long))
}

fn is_valid_js_runtime(value: &str) -> bool {
    let (name, path) = match value.split_once(':') {
        Some((n, p)) => (n, Some(p)),
        None => (value, None),
    };
    if !JS_RUNTIME_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        return false;
    }
    match path {
        None => true,
        Some(p) => {
            let file = p
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            !p.is_empty() && JS_RUNTIME_BINARIES.contains(&file.as_str())
        }
    }
}

fn is_valid_value(option: &str, value: &str) -> bool {
    if value.is_empty() || value.contains('\0') || value.contains('\n') {
        return false;
    }
    match option {
        "--js-runtimes" => is_valid_js_runtime(value),
        _ => true,
    }
}

/// Filter user-supplied yt-dlp tokens through the allowlist.
///
/// Tokens are the same shape as on a command line: options and their values may
/// be separate items (`["--format", "best"]`) or joined (`["--format=best"]`).
pub fn filter_user_flags(tokens: &[String]) -> FilteredFlags {
    let mut out = FilteredFlags::default();
    let cleaned: Vec<&str> = tokens
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();
    let mut i = 0;
    while i < cleaned.len() {
        let token = cleaned[i];
        i += 1;
        if !token.starts_with('-') || token == "-" || token == "--" {
            // Stray value or bare separator: never forward (yt-dlp would read it
            // as a URL or stop parsing options).
            out.rejected.push(token.to_string());
            continue;
        }
        let (raw_name, inline_value) = match token.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (token, None),
        };
        let name = if raw_name.starts_with("--") {
            raw_name.to_ascii_lowercase()
        } else {
            raw_name.to_string()
        };
        let Some((canonical, arity)) = resolve_option(&name) else {
            out.rejected.push(token.to_string());
            // Swallow the value that belonged to the refused option.
            if inline_value.is_none() && i < cleaned.len() && !cleaned[i].starts_with('-') {
                out.rejected.push(cleaned[i].to_string());
                i += 1;
            }
            continue;
        };
        match arity {
            Flag => {
                if inline_value.is_some() {
                    out.rejected.push(token.to_string());
                } else {
                    out.accepted.push(canonical.to_string());
                }
            }
            Value => {
                let value = match inline_value {
                    Some(v) => Some(v.to_string()),
                    None if i < cleaned.len() && !cleaned[i].starts_with("--") => {
                        i += 1;
                        Some(cleaned[i - 1].to_string())
                    }
                    None => None,
                };
                match value {
                    Some(v) if is_valid_value(canonical, &v) => {
                        out.accepted.push(canonical.to_string());
                        out.accepted.push(v);
                    }
                    Some(v) => {
                        out.rejected.push(token.to_string());
                        if inline_value.is_none() {
                            out.rejected.push(v);
                        }
                    }
                    None => out.rejected.push(token.to_string()),
                }
            }
        }
    }
    out
}

/// Strict variant used where the user explicitly typed arguments for one
/// download: any refused token is an error instead of being silently dropped.
pub fn validate_user_flags(tokens: &[String]) -> Result<Vec<String>, String> {
    let filtered = filter_user_flags(tokens);
    if filtered.rejected.is_empty() {
        Ok(filtered.accepted)
    } else {
        Err(format!(
            "Disallowed yt-dlp argument for security: {}",
            filtered.rejected.join(" ")
        ))
    }
}

/// True when an internal (code-generated) token could make yt-dlp run a program,
/// including abbreviated spellings such as `--exec-befo` or `--netrc-c`.
pub fn is_dangerous_option(token: &str) -> bool {
    let lower = token.trim().to_ascii_lowercase();
    let name = lower.split('=').next().unwrap_or(&lower);
    if !name.starts_with("--") || name.len() < 4 {
        return false;
    }
    EXEC_LONG.iter().any(|d| *d == name || d.starts_with(name)) && lookup_exact(name).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn execution_vectors_are_rejected() {
        let vectors: &[&[&str]] = &[
            &["--exec", "touch /tmp/x"],
            &["--exec=touch /tmp/x"],
            &["--exec-befo", "touch /tmp/x"],
            &[
                "--use-postprocessor",
                "Exec:exec_cmd=touch /tmp/x;when=pre_process",
            ],
            &["--use-post", "Exec:exec_cmd=id"],
            &["--netrc-cmd", "id"],
            &["--netrc-c", "id"],
            &["--plugin-dirs", "/tmp/evil"],
            &["--config-locations", "/tmp/conf"],
            &["--ffmpeg-location", "/bin/sh"],
            &["--batch-file", "/etc/passwd"],
            &["-a", "/etc/passwd"],
            &["--js-runtimes", "node:/tmp/evil.sh"],
            &["--js-runtimes", "evil:/usr/bin/node"],
            &["-o", "/etc/cron.d/x"],
        ];
        for tokens in vectors {
            let out = filter_user_flags(&v(tokens));
            assert!(
                out.accepted.is_empty(),
                "{tokens:?} must be refused, got {:?}",
                out.accepted
            );
            assert_eq!(
                out.rejected.len(),
                tokens.len(),
                "{tokens:?}: value must be dropped too"
            );
        }
    }

    #[test]
    fn safe_flags_are_kept_and_abbreviations_expanded() {
        let out = filter_user_flags(&v(&[
            "--format",
            "bv*+ba/b",
            "--embed-chap",
            "--limit-rate=2M",
            "-N",
            "4",
            "--sponsorblock-remove",
            "sponsor",
            "--js-runtimes",
            "node:C:\\node.exe",
        ]));
        assert!(out.rejected.is_empty(), "{:?}", out.rejected);
        assert_eq!(
            out.accepted,
            v(&[
                "--format",
                "bv*+ba/b",
                "--embed-chapters",
                "--limit-rate",
                "2M",
                "--concurrent-fragments",
                "4",
                "--sponsorblock-remove",
                "sponsor",
                "--js-runtimes",
                "node:C:\\node.exe",
            ])
        );
    }

    #[test]
    fn ambiguous_abbreviation_is_rejected() {
        // "--embed" matches several allowed options.
        let out = filter_user_flags(&v(&["--embed"]));
        assert!(out.accepted.is_empty());
    }

    #[test]
    fn stray_values_are_not_forwarded() {
        let out = filter_user_flags(&v(&["https://evil.example/x", "--no-mtime"]));
        assert_eq!(out.accepted, v(&["--no-mtime"]));
        assert_eq!(out.rejected, v(&["https://evil.example/x"]));
    }

    #[test]
    fn strict_validation_reports_rejections() {
        assert!(validate_user_flags(&v(&["--netrc-c", "id"])).is_err());
        assert_eq!(
            validate_user_flags(&v(&["--write-subs"])).unwrap(),
            v(&["--write-subs"])
        );
    }

    #[test]
    fn dangerous_option_detection_handles_abbreviations() {
        assert!(is_dangerous_option("--exec-befo"));
        assert!(is_dangerous_option("--netrc-c=id"));
        assert!(is_dangerous_option("--use-postprocessor"));
        assert!(!is_dangerous_option("--no-overwrites"));
        assert!(!is_dangerous_option("--add-headers"));
        assert!(!is_dangerous_option("--download-sections"));
        assert!(!is_dangerous_option("--cookies"));
        assert!(!is_dangerous_option("--ffmpeg-location"));
    }
}
