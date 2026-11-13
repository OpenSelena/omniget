use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct LiveChatMessage {
    pub idx: u64,
    pub time: String,
    pub timestamp_usec: i64,
    pub author: String,
    pub channel_id: String,
    pub message: String,
    pub msg_type: String,
    pub amount: String,
}

fn runs_text(v: &Value) -> String {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn str_at<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or("")
}

fn parse_renderer(renderer: &Value, msg_type: &str, idx: u64) -> LiveChatMessage {
    let message = {
        let m = renderer.pointer("/message/runs");
        if m.is_some() {
            runs_text(m.unwrap())
        } else if let Some(h) = renderer.pointer("/headerSubtext/runs") {
            runs_text(h)
        } else {
            String::new()
        }
    };
    LiveChatMessage {
        idx,
        time: str_at(renderer, "/timestampText/simpleText").to_string(),
        timestamp_usec: str_at(renderer, "/timestampUsec")
            .parse::<i64>()
            .unwrap_or(0),
        author: str_at(renderer, "/authorName/simpleText").to_string(),
        channel_id: renderer
            .get("authorExternalChannelId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        message,
        msg_type: msg_type.to_string(),
        amount: str_at(renderer, "/purchaseAmountText/simpleText").to_string(),
    }
}

/// Parses a yt-dlp `*.live_chat.json` file (newline-delimited JSON, one
/// replay-chat action wrapper per line) into a flat message list.
pub fn parse_live_chat(ndjson: &str) -> Vec<LiveChatMessage> {
    let mut out = Vec::new();
    let mut idx: u64 = 0;
    for line in ndjson.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(actions) = root
            .pointer("/replayChatItemAction/actions")
            .and_then(|a| a.as_array())
        else {
            continue;
        };
        for action in actions {
            let Some(item) = action.pointer("/addChatItemAction/item") else {
                continue;
            };
            if let Some(r) = item.get("liveChatTextMessageRenderer") {
                idx += 1;
                out.push(parse_renderer(r, "text", idx));
            } else if let Some(r) = item.get("liveChatPaidMessageRenderer") {
                idx += 1;
                out.push(parse_renderer(r, "paid", idx));
            } else if let Some(r) = item.get("liveChatMembershipItemRenderer") {
                idx += 1;
                out.push(parse_renderer(r, "membership", idx));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_message() {
        let line = r#"{"replayChatItemAction":{"actions":[{"addChatItemAction":{"item":{"liveChatTextMessageRenderer":{"message":{"runs":[{"text":"hello "},{"text":"world"}]},"authorName":{"simpleText":"Alice"},"authorExternalChannelId":"UC123","timestampUsec":"1690000000000000","timestampText":{"simpleText":"1:23"}}}}}]}}"#;
        let msgs = parse_live_chat(line);
        assert_eq!(msgs.len(), 1);
        let m = &msgs[0];
        assert_eq!(m.author, "Alice");
        assert_eq!(m.message, "hello world");
        assert_eq!(m.time, "1:23");
        assert_eq!(m.channel_id, "UC123");
        assert_eq!(m.timestamp_usec, 1690000000000000);
        assert_eq!(m.msg_type, "text");
        assert_eq!(m.idx, 1);
    }

    #[test]
    fn parses_paid_message_with_amount() {
        let line = r#"{"replayChatItemAction":{"actions":[{"addChatItemAction":{"item":{"liveChatPaidMessageRenderer":{"message":{"runs":[{"text":"thanks"}]},"authorName":{"simpleText":"Bob"},"purchaseAmountText":{"simpleText":"$5.00"}}}}}]}}"#;
        let msgs = parse_live_chat(line);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type, "paid");
        assert_eq!(msgs[0].amount, "$5.00");
        assert_eq!(msgs[0].message, "thanks");
    }

    #[test]
    fn skips_invalid_and_unknown_lines() {
        let input = "not json\n{}\n{\"replayChatItemAction\":{\"actions\":[{\"addChatItemAction\":{\"item\":{\"liveChatViewerEngagementMessageRenderer\":{}}}}]}}";
        assert!(parse_live_chat(input).is_empty());
    }

    #[test]
    fn membership_uses_header_subtext_fallback() {
        let line = r#"{"replayChatItemAction":{"actions":[{"addChatItemAction":{"item":{"liveChatMembershipItemRenderer":{"headerSubtext":{"runs":[{"text":"Member for 6 months"}]},"authorName":{"simpleText":"Carol"}}}}}]}}"#;
        let msgs = parse_live_chat(line);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type, "membership");
        assert_eq!(msgs[0].message, "Member for 6 months");
    }
}
