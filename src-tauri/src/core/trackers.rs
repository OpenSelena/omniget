// Curated set of long-lived public BitTorrent trackers. Many magnet links
// carry few or dead trackers; injecting these stable open trackers materially
// improves peer discovery so downloads actually start. Kept as a static list
// (no runtime fetch) to avoid an external dependency, scheduler, and the
// SSRF/privacy surface of pulling a remote list at runtime.
const PUBLIC_TRACKERS: &[&str] = &[
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.tracker.cl:1337/announce",
    "udp://tracker.openbittorrent.com:6969/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://tracker.dler.org:6969/announce",
    "udp://explodie.org:6969/announce",
    "https://tracker.tamersunion.org:443/announce",
    "udp://opentracker.io:6969/announce",
];

pub fn extra_trackers() -> Vec<String> {
    PUBLIC_TRACKERS.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_non_empty() {
        assert!(!extra_trackers().is_empty());
    }

    #[test]
    fn all_have_supported_scheme() {
        for t in extra_trackers() {
            assert!(
                t.starts_with("udp://") || t.starts_with("http://") || t.starts_with("https://"),
                "unexpected tracker scheme: {t}"
            );
        }
    }

    #[test]
    fn all_end_with_announce() {
        for t in extra_trackers() {
            assert!(t.ends_with("/announce"), "tracker missing /announce: {t}");
        }
    }

    #[test]
    fn no_duplicates() {
        let mut v = extra_trackers();
        let before = v.len();
        v.sort();
        v.dedup();
        assert_eq!(before, v.len(), "tracker list has duplicates");
    }
}
