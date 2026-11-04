//! Detects DRM-protected media so a download stops with a clear message
//! instead of a generic failure.
//!
//! Detection only: OmniGet never decrypts, strips or bypasses DRM. When a
//! manifest or yt-dlp says the media is protected, the download is refused.

/// Fixed English message. The frontend maps it to `errors.drm_protected`.
pub const DRM_ERROR: &str = "This content is protected by DRM (Widevine, PlayReady or FairPlay). OmniGet cannot download DRM-protected media.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrmSystem {
    Widevine,
    PlayReady,
    FairPlay,
    /// Protected (CENC, SAMPLE-AES, ClearKey…) without a known system id.
    Other,
}

impl DrmSystem {
    pub fn name(self) -> &'static str {
        match self {
            DrmSystem::Widevine => "Widevine",
            DrmSystem::PlayReady => "PlayReady",
            DrmSystem::FairPlay => "FairPlay",
            DrmSystem::Other => "unknown DRM",
        }
    }
}

const WIDEVINE_ID: &str = "edef8ba9-79d6-4ace-a3c8-27dcd51d21ed";
const PLAYREADY_ID: &str = "9a04f079-9840-4286-ab92-e65be0885f95";
const FAIRPLAY_ID: &str = "94ce86fb-07ff-4f43-adb8-93d2fa968ca2";

fn system_from_marker(lower: &str) -> Option<DrmSystem> {
    if lower.contains(WIDEVINE_ID) || lower.contains("com.widevine") {
        Some(DrmSystem::Widevine)
    } else if lower.contains(PLAYREADY_ID) || lower.contains("com.microsoft.playready") {
        Some(DrmSystem::PlayReady)
    } else if lower.contains(FAIRPLAY_ID)
        || lower.contains("com.apple.streamingkeydelivery")
        || lower.contains("skd://")
    {
        Some(DrmSystem::FairPlay)
    } else {
        None
    }
}

/// Value of `ATTR=` in an HLS attribute list, without quotes.
fn hls_attr<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let upper = line.to_ascii_uppercase();
    let needle = format!("{name}=");
    let mut from = 0;
    while let Some(pos) = upper[from..].find(&needle) {
        let start = from + pos;
        let boundary = start == 0 || matches!(upper.as_bytes()[start - 1], b':' | b',');
        let value_start = start + needle.len();
        if boundary {
            let rest = &line[value_start..];
            return Some(if let Some(quoted) = rest.strip_prefix('"') {
                quoted.split('"').next().unwrap_or("")
            } else {
                rest.split(',').next().unwrap_or("")
            });
        }
        from = value_start;
    }
    None
}

fn hls_drm(text: &str) -> Option<DrmSystem> {
    for line in text.lines() {
        let line = line.trim();
        if !(line.starts_with("#EXT-X-KEY:") || line.starts_with("#EXT-X-SESSION-KEY:")) {
            continue;
        }
        let method = hls_attr(line, "METHOD").unwrap_or("").to_ascii_uppercase();
        if method == "NONE" {
            continue;
        }
        let keyformat = hls_attr(line, "KEYFORMAT").unwrap_or("").to_lowercase();
        let uri = hls_attr(line, "URI").unwrap_or("").to_lowercase();
        if let Some(system) = system_from_marker(&format!("{keyformat} {uri}")) {
            return Some(system);
        }
        // Plain AES-128 with an identity key is ordinary encryption that the
        // HLS downloader handles; anything else is a DRM scheme.
        if method != "AES-128" {
            return Some(DrmSystem::Other);
        }
    }
    None
}

fn xml_drm(text: &str, element: &str, id_attr: &str) -> Option<DrmSystem> {
    let lower = text.to_lowercase();
    let open = format!("<{}", element.to_lowercase());
    let mut found = None;
    let mut from = 0;
    while let Some(pos) = lower[from..].find(&open) {
        let start = from + pos;
        let end = lower[start..]
            .find('>')
            .map(|e| start + e)
            .unwrap_or(lower.len());
        let tag = &lower[start..end];
        let id = tag
            .find(&format!("{}=", id_attr.to_lowercase()))
            .map(|p| &tag[p..])
            .unwrap_or(tag);
        match system_from_marker(id) {
            Some(system) => return Some(system),
            None => found = Some(DrmSystem::Other),
        }
        from = end;
    }
    found
}

/// Looks for DRM signalling in an HLS, DASH or Smooth Streaming manifest.
pub fn manifest_drm(text: &str) -> Option<DrmSystem> {
    let head = text.trim_start_matches('\u{feff}').trim_start();
    if head.starts_with("#EXTM3U") {
        return hls_drm(text);
    }
    if head.starts_with('<') {
        return xml_drm(text, "ContentProtection", "schemeIdUri")
            .or_else(|| xml_drm(text, "ProtectionHeader", "SystemID"));
    }
    None
}

/// True when yt-dlp's error output says the media is DRM protected.
pub fn ytdlp_reports_drm(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.lines().any(|line| {
        !line.contains("warning")
            && (line.contains("drm protected")
                || line.contains("drm-protected")
                || line.contains("protected by drm"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hls_drm_systems_are_recognised() {
        let widevine = "#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"data:text/plain;base64,AAAA\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\"\n#EXTINF:4,\nseg1.mp4\n";
        assert_eq!(manifest_drm(widevine), Some(DrmSystem::Widevine));

        let fairplay = "#EXTM3U\n#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES,URI=\"skd://key42\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n";
        assert_eq!(manifest_drm(fairplay), Some(DrmSystem::FairPlay));

        let playready = "#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"data:x\",KEYFORMAT=\"com.microsoft.playready\"\n";
        assert_eq!(manifest_drm(playready), Some(DrmSystem::PlayReady));

        let sample_aes = "#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"https://k.example/key\"\n";
        assert_eq!(manifest_drm(sample_aes), Some(DrmSystem::Other));
    }

    #[test]
    fn plain_hls_encryption_is_not_drm() {
        let aes = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"https://k.example/key\",IV=0x1234\n#EXTINF:4,\nseg1.ts\n";
        assert_eq!(manifest_drm(aes), None);
        let none = "#EXTM3U\n#EXT-X-KEY:METHOD=NONE\n#EXTINF:4,\nseg1.ts\n";
        assert_eq!(manifest_drm(none), None);
        let clear = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1,RESOLUTION=1280x720\nv.m3u8\n";
        assert_eq!(manifest_drm(clear), None);
    }

    #[test]
    fn dash_and_smooth_drm_are_recognised() {
        let dash = r#"<?xml version="1.0"?><MPD><Period><AdaptationSet>
            <ContentProtection schemeIdUri="urn:mpeg:dash:mp4protection:2011" value="cenc"/>
            <ContentProtection schemeIdUri="urn:uuid:9A04F079-9840-4286-AB92-E65BE0885F95"/>
            </AdaptationSet></Period></MPD>"#;
        assert_eq!(manifest_drm(dash), Some(DrmSystem::PlayReady));

        let widevine = r#"<MPD><ContentProtection schemeIdUri="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"/></MPD>"#;
        assert_eq!(manifest_drm(widevine), Some(DrmSystem::Widevine));

        let cenc_only =
            r#"<MPD><ContentProtection schemeIdUri="urn:mpeg:dash:mp4protection:2011"/></MPD>"#;
        assert_eq!(manifest_drm(cenc_only), Some(DrmSystem::Other));

        let smooth = r#"<SmoothStreamingMedia><Protection><ProtectionHeader SystemID="9a04f079-9840-4286-ab92-e65be0885f95">AAAA</ProtectionHeader></Protection></SmoothStreamingMedia>"#;
        assert_eq!(manifest_drm(smooth), Some(DrmSystem::PlayReady));

        let clear = r#"<MPD><Period><AdaptationSet mimeType="video/mp4"/></Period></MPD>"#;
        assert_eq!(manifest_drm(clear), None);
    }

    #[test]
    fn ytdlp_drm_errors_are_recognised() {
        assert!(ytdlp_reports_drm(
            "ERROR: [generic] abc: This video is DRM protected"
        ));
        assert!(ytdlp_reports_drm(
            "ERROR: [Netflix] 123: The requested format is DRM-protected"
        ));
        assert!(!ytdlp_reports_drm(
            "WARNING: [youtube] x: Some formats are possibly DRM protected and were skipped"
        ));
        assert!(!ytdlp_reports_drm("ERROR: HTTP Error 404: Not Found"));
    }
}
