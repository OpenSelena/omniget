pub mod traits;

use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Hotmart,
    YouTube,
    Instagram,
    TikTok,
    Twitter,
    Reddit,
    Twitch,
    Pinterest,
    Bluesky,
    Telegram,
    Vimeo,
    Udemy,
    Bilibili,
    Other(String),
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Platform::Hotmart => "hotmart",
            Platform::YouTube => "youtube",
            Platform::Instagram => "instagram",
            Platform::TikTok => "tiktok",
            Platform::Twitter => "twitter",
            Platform::Reddit => "reddit",
            Platform::Twitch => "twitch",
            Platform::Pinterest => "pinterest",
            Platform::Bluesky => "bluesky",
            Platform::Telegram => "telegram",
            Platform::Vimeo => "vimeo",
            Platform::Udemy => "udemy",
            Platform::Bilibili => "bilibili",
            Platform::Other(ref name) => name.as_str(),
        };
        write!(f, "{}", name)
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hotmart" => Ok(Platform::Hotmart),
            "youtube" | "yt" => Ok(Platform::YouTube),
            "instagram" | "ig" => Ok(Platform::Instagram),
            "tiktok" | "tt" => Ok(Platform::TikTok),
            "twitter" | "x" => Ok(Platform::Twitter),
            "reddit" => Ok(Platform::Reddit),
            "twitch" => Ok(Platform::Twitch),
            "pinterest" => Ok(Platform::Pinterest),
            "bluesky" | "bsky" => Ok(Platform::Bluesky),
            "telegram" | "tg" => Ok(Platform::Telegram),
            "vimeo" => Ok(Platform::Vimeo),
            "udemy" => Ok(Platform::Udemy),
            "bilibili" | "b站" => Ok(Platform::Bilibili),
            _ => Err(format!("Unknown platform: {}", s)),
        }
    }
}

impl Platform {
    pub fn from_url(url_str: &str) -> Option<Self> {
        // P2P share codes: "p2p:word-word-word-word"
        if url_str.starts_with("p2p:") {
            return Some(Platform::Other("p2p".to_string()));
        }

        // Magnet links have no hostname, detect by scheme prefix
        // .torrent URLs are also handled by the magnet downloader
        if url_str.starts_with("magnet:") || url_str.ends_with(".torrent") {
            return Some(Platform::Other("magnet".to_string()));
        }

        let parsed = url::Url::parse(url_str).ok()?;
        let host = parsed.host_str()?.to_lowercase();

        let matches = |domain: &str| -> bool {
            host == domain || host.ends_with(&format!(".{}", domain))
        };

        if matches("hotmart.com") {
            Some(Platform::Hotmart)
        } else if matches("youtube.com") || matches("youtube-nocookie.com") || host == "youtu.be" {
            Some(Platform::YouTube)
        } else if matches("instagram.com") || matches("ddinstagram.com") {
            Some(Platform::Instagram)
        } else if matches("tiktok.com") {
            Some(Platform::TikTok)
        } else if matches("twitter.com") || matches("x.com") || matches("vxtwitter.com") || matches("fixvx.com") {
            Some(Platform::Twitter)
        } else if matches("reddit.com") || host == "v.redd.it" || host == "redd.it" {
            Some(Platform::Reddit)
        } else if matches("twitch.tv") {
            Some(Platform::Twitch)
        } else if host == "pin.it" || host.contains("pinterest.") {
            Some(Platform::Pinterest)
        } else if host == "bsky.app" || host.ends_with(".bsky.app") {
            Some(Platform::Bluesky)
        } else if host == "t.me" || matches("telegram.me") || matches("telegram.org") {
            Some(Platform::Telegram)
        } else if matches("vimeo.com") {
            Some(Platform::Vimeo)
        } else if matches("udemy.com") {
            Some(Platform::Udemy)
        } else if matches("bilibili.com") || host == "b23.tv" {
            Some(Platform::Bilibili)
        } else if matches("kiwify.com.br") {
            Some(Platform::Other("kiwify".to_string()))
        } else if matches("gumroad.com") {
            Some(Platform::Other("gumroad".to_string()))
        } else if matches("teachable.com") {
            Some(Platform::Other("teachable".to_string()))
        } else if matches("kajabi.com") {
            Some(Platform::Other("kajabi".to_string()))
        } else if matches("skool.com") {
            Some(Platform::Other("skool".to_string()))
        } else if matches("pluralsight.com") {
            Some(Platform::Other("pluralsight".to_string()))
        } else if matches("thegreatcoursesplus.com") || matches("wondrium.com") {
            Some(Platform::Other("greatcourses".to_string()))
        } else if matches("masterclass.com") {
            Some(Platform::Other("masterclass".to_string()))
        } else if matches("thinkific.com") {
            Some(Platform::Other("thinkific".to_string()))
        } else if matches("curseduca.pro") || matches("curseduca.com") {
            Some(Platform::Other("curseduca".to_string()))
        } else if matches("cademi.com.br") {
            Some(Platform::Other("cademi".to_string()))
        } else if host == "app.cakto.com.br" {
            Some(Platform::Other("caktomembers".to_string()))
        } else if matches("cakto.com.br") {
            Some(Platform::Other("cakto".to_string()))
        } else if matches("kirvano.com") {
            Some(Platform::Other("kirvano".to_string()))
        } else if matches("memberkit.com.br") {
            Some(Platform::Other("memberkit".to_string()))
        } else if matches("rocketseat.com.br") {
            Some(Platform::Other("rocketseat".to_string()))
        } else if matches("grancursosonline.com.br") {
            Some(Platform::Other("grancursos".to_string()))
        } else if matches("fluency.io") {
            Some(Platform::Other("fluencyacademy".to_string()))
        } else if matches("datascienceacademy.com.br") {
            Some(Platform::Other("datascienceacademy".to_string()))
        } else if matches("medcel.com.br") {
            Some(Platform::Other("medcel".to_string()))
        } else if matches("medcof.com.br") {
            Some(Platform::Other("medcof".to_string()))
        } else if matches("medway.com.br") {
            Some(Platform::Other("medway".to_string()))
        } else if matches("afya.com.br") {
            Some(Platform::Other("afyainternato".to_string()))
        } else if matches("alpaclass.com") {
            Some(Platform::Other("alpaclass".to_string()))
        } else if matches("areademembros.com") {
            Some(Platform::Other("areademembros".to_string()))
        } else if matches("astronmembers.com") {
            Some(Platform::Other("astronmembers".to_string()))
        } else if matches("nutror.com") || matches("eduzz.com") {
            Some(Platform::Other("eduzznutror".to_string()))
        } else if matches("entregadigital.app.br") {
            Some(Platform::Other("entregadigital".to_string()))
        } else if matches("greenn.club") {
            Some(Platform::Other("greennclub".to_string()))
        } else if matches("themembers.com.br") {
            Some(Platform::Other("themembers".to_string()))
        } else if matches("voompplay.com.br") {
            Some(Platform::Other("voompplay".to_string()))
        } else if matches("estrategiaconcursos.com.br") {
            Some(Platform::Other("estrategia_concursos".to_string()))
        } else if host == "concursos.estrategia.com" {
            Some(Platform::Other("estrategia_ldi".to_string()))
        } else if host == "militares.estrategia.com" {
            Some(Platform::Other("estrategia_militares".to_string()))
        } else if matches("estrategia.com") && parsed.path().contains("concursos") {
            Some(Platform::Other("estrategia_concursos".to_string()))
        } else if matches("matematicaprapassar.com.br") {
            Some(Platform::Other("matematicaprapassar".to_string()))
        } else if matches("douyin.com") {
            Some(Platform::Other("douyin".to_string()))
        } else if matches("kuaishou.com") {
            Some(Platform::Other("kuaishou".to_string()))
        } else if matches("xiaohongshu.com") || host == "xhslink.com" {
            Some(Platform::Other("xiaohongshu".to_string()))
        } else if host == "v.qq.com" || (matches("qq.com") && parsed.path().starts_with("/x/")) {
            Some(Platform::Other("tencentvideo".to_string()))
        } else if matches("iqiyi.com") {
            Some(Platform::Other("iqiyi".to_string()))
        } else if matches("mgtv.com") {
            Some(Platform::Other("mgtv".to_string()))
        } else if matches("youku.com") {
            Some(Platform::Other("youku".to_string()))
        } else {
            None
        }
    }

    pub fn all() -> &'static [Platform] {
        &[
            Platform::Hotmart,
            Platform::YouTube,
            Platform::Instagram,
            Platform::TikTok,
            Platform::Twitter,
            Platform::Reddit,
            Platform::Twitch,
            Platform::Pinterest,
            Platform::Bluesky,
            Platform::Telegram,
            Platform::Vimeo,
            Platform::Udemy,
            Platform::Bilibili,
        ]
    }
}
