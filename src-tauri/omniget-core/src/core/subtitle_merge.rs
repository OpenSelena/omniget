struct SubBlock {
    timing: String,
    text: String,
}

fn parse_srt(content: &str) -> Vec<SubBlock> {
    let normalized = content.replace('\r', "");
    let mut blocks = Vec::new();
    for part in normalized.split("\n\n") {
        let lines: Vec<&str> = part.trim().split('\n').collect();
        if lines.len() >= 3 {
            blocks.push(SubBlock {
                timing: lines[1].to_string(),
                text: lines[2..].join("\n"),
            });
        }
    }
    blocks
}

fn parse_vtt(content: &str) -> Vec<SubBlock> {
    let normalized = content.replace('\r', "");
    let mut blocks = Vec::new();
    for part in normalized.split("\n\n") {
        let lines: Vec<&str> = part.trim().split('\n').collect();
        if lines.is_empty() {
            continue;
        }
        if let Some(idx) = lines.iter().position(|l| l.contains("-->")) {
            if idx + 1 < lines.len() {
                blocks.push(SubBlock {
                    timing: lines[idx].to_string(),
                    text: lines[idx + 1..].join("\n"),
                });
            }
        }
    }
    blocks
}

fn merge(primary: &[SubBlock], secondary: &[SubBlock], header: &[&str], numbered: bool) -> String {
    let max_len = primary.len().max(secondary.len());
    let mut out: Vec<String> = header.iter().map(|s| s.to_string()).collect();
    for i in 0..max_len {
        if numbered {
            out.push((i + 1).to_string());
        }
        match (primary.get(i), secondary.get(i)) {
            (Some(pb), sb) => {
                out.push(pb.timing.clone());
                out.push(pb.text.clone());
                if let Some(sb) = sb {
                    out.push(sb.text.clone());
                }
            }
            (None, Some(sb)) => {
                out.push(sb.timing.clone());
                out.push(sb.text.clone());
            }
            (None, None) => {}
        }
        out.push(String::new());
    }
    out.join("\r\n")
}

pub fn merge_bilingual_srt(primary: &str, secondary: &str) -> String {
    merge(&parse_srt(primary), &parse_srt(secondary), &[], true)
}

pub fn merge_bilingual_vtt(primary: &str, secondary: &str) -> String {
    merge(
        &parse_vtt(primary),
        &parse_vtt(secondary),
        &["WEBVTT", ""],
        false,
    )
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

fn parse_ts(s: &str) -> Option<u64> {
    // Accepts HH:MM:SS,mmm / HH:MM:SS.mmm / MM:SS.mmm
    let s = s.trim();
    let (hms, ms_part) = if let Some((a, b)) = s.split_once(',') {
        (a, b)
    } else if let Some((a, b)) = s.rsplit_once('.') {
        (a, b)
    } else {
        (s, "0")
    };
    let ms: u64 = ms_part
        .chars()
        .take(3)
        .collect::<String>()
        .parse()
        .ok()
        .map(|v: u64| v * 10u64.pow(3u32.saturating_sub(ms_part.len().min(3) as u32)))
        .unwrap_or(0);
    let parts: Vec<&str> = hms.split(':').collect();
    let (h, m, sec): (u64, u64, u64) = match parts.as_slice() {
        [h, m, s] => (
            h.trim().parse::<u64>().ok()?,
            m.trim().parse::<u64>().ok()?,
            s.trim().parse::<u64>().ok()?,
        ),
        [m, s] => (
            0,
            m.trim().parse::<u64>().ok()?,
            s.trim().parse::<u64>().ok()?,
        ),
        _ => return None,
    };
    Some(((h * 3600 + m * 60 + sec) * 1000) + ms)
}

fn fmt_ts(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let milli = ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, milli)
}

pub fn parse_cues(content: &str) -> Vec<Cue> {
    let normalized = content.replace('\r', "");
    let mut cues = Vec::new();
    for part in normalized.split("\n\n") {
        let lines: Vec<&str> = part.trim().split('\n').collect();
        let Some(idx) = lines.iter().position(|l| l.contains("-->")) else {
            continue;
        };
        let timing = lines[idx];
        let Some((a, b)) = timing.split_once("-->") else {
            continue;
        };
        let (Some(start_ms), Some(end_ms)) = (
            parse_ts(a),
            parse_ts(b.split_whitespace().next().unwrap_or(b)),
        ) else {
            continue;
        };
        let text = lines[idx + 1..].join("\n").trim().to_string();
        if text.is_empty() {
            continue;
        }
        cues.push(Cue {
            start_ms,
            end_ms,
            text,
        });
    }
    cues
}

pub fn cues_to_srt(cues: &[Cue]) -> String {
    let mut out = String::new();
    for (i, c) in cues.iter().enumerate() {
        out.push_str(&(i + 1).to_string());
        out.push('\n');
        out.push_str(&fmt_ts(c.start_ms));
        out.push_str(" --> ");
        out.push_str(&fmt_ts(c.end_ms));
        out.push('\n');
        out.push_str(&c.text);
        out.push_str("\n\n");
    }
    out
}

pub fn cues_to_vtt(cues: &[Cue]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for c in cues {
        out.push_str(&fmt_ts(c.start_ms).replace(',', "."));
        out.push_str(" --> ");
        out.push_str(&fmt_ts(c.end_ms).replace(',', "."));
        out.push('\n');
        out.push_str(&c.text);
        out.push_str("\n\n");
    }
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

// Flattens an SRT/VTT subtitle into plain transcript text. yt-dlp auto-subs
// roll up and repeat lines heavily, so consecutive duplicate lines are
// collapsed. Used to feed transcripts to the AI summarizer.
pub fn extract_transcript(content: &str) -> String {
    let blocks = if content.trim_start().starts_with("WEBVTT") {
        parse_vtt(content)
    } else {
        parse_srt(content)
    };
    let mut lines: Vec<String> = Vec::new();
    for b in blocks {
        for raw in b.text.split('\n') {
            let line = strip_tags(raw).trim().to_string();
            if line.is_empty() {
                continue;
            }
            if lines.last().map(|l| l == &line).unwrap_or(false) {
                continue;
            }
            lines.push(line);
        }
    }
    lines.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_blocks_parsed_index_aligned() {
        let p =
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nWorld";
        let s = "1\n00:00:01,000 --> 00:00:02,000\nOlá\n\n2\n00:00:03,000 --> 00:00:04,000\nMundo";
        let merged = merge_bilingual_srt(p, s);
        assert!(merged.contains("Hello\r\nOlá"));
        assert!(merged.contains("World\r\nMundo"));
        assert!(merged.starts_with("1\r\n"));
    }

    #[test]
    fn vtt_has_header_and_no_index() {
        let p = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHi";
        let s = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nOi";
        let merged = merge_bilingual_vtt(p, s);
        assert!(merged.starts_with("WEBVTT\r\n"));
        assert!(merged.contains("Hi\r\nOi"));
        assert!(!merged.contains("\r\n1\r\n"));
    }

    #[test]
    fn uneven_lengths_fall_back_to_secondary_timing() {
        let p = "1\n00:00:01,000 --> 00:00:02,000\nOne";
        let s = "1\n00:00:01,000 --> 00:00:02,000\nUm\n\n2\n00:00:05,000 --> 00:00:06,000\nDois";
        let merged = merge_bilingual_srt(p, s);
        assert!(merged.contains("One\r\nUm"));
        assert!(merged.contains("00:00:05,000 --> 00:00:06,000\r\nDois"));
    }

    #[test]
    fn empty_inputs_are_safe() {
        assert_eq!(merge_bilingual_srt("", ""), "");
        assert_eq!(merge_bilingual_vtt("", ""), "WEBVTT\r\n");
    }

    #[test]
    fn transcript_strips_tags_and_dedupes() {
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n<c>Hello</c> world\n\n00:00:02.000 --> 00:00:03.000\nHello world\n\n00:00:03.000 --> 00:00:04.000\nNext line";
        let t = extract_transcript(vtt);
        assert_eq!(t, "Hello world Next line");
    }

    #[test]
    fn transcript_srt_plain() {
        let srt =
            "1\n00:00:01,000 --> 00:00:02,000\nFirst\n\n2\n00:00:03,000 --> 00:00:04,000\nSecond";
        assert_eq!(extract_transcript(srt), "First Second");
    }

    #[test]
    fn parse_cues_srt_roundtrip() {
        let srt =
            "1\n00:00:01,500 --> 00:00:02,250\nHello\n\n2\n00:01:00,000 --> 00:01:02,000\nWorld";
        let cues = parse_cues(srt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 1500);
        assert_eq!(cues[0].end_ms, 2250);
        assert_eq!(cues[1].start_ms, 60000);
        assert_eq!(cues[0].text, "Hello");
        let back = cues_to_srt(&cues);
        let again = parse_cues(&back);
        assert_eq!(again[1].start_ms, 60000);
        assert_eq!(again[1].text, "World");
    }

    #[test]
    fn parse_cues_vtt() {
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHi there";
        let cues = parse_cues(vtt);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start_ms, 1000);
        assert_eq!(cues[0].text, "Hi there");
        assert!(cues_to_vtt(&cues).starts_with("WEBVTT"));
    }
}
