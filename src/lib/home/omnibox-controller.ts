export type OmniState =
  | { kind: "idle" }
  | { kind: "detecting" }
  | { kind: "detected"; info: PlatformInfo }
  | { kind: "unsupported" }
  | { kind: "preparing"; platform: string }
  | { kind: "batch"; urls: string[] }
  | { kind: "searching" }
  | { kind: "search-results"; results: SearchResult[] }
  | { kind: "search-empty" }
  | { kind: "error"; message: string; originalUrl: string; platform: string };

export type PlatformInfo = {
  platform: string;
  supported: boolean;
  content_id: string | null;
  content_type: string | null;
};

export type SearchResult = {
  id: string;
  title: string;
  author: string;
  duration: number | null;
  thumbnail_url: string | null;
  url: string;
  platform: string;
};

export type HomeInputMode = "url" | "batch" | "torrent" | "p2p";

export function isUrl(value: string): boolean {
  return (
    value.startsWith("http://") ||
    value.startsWith("https://") ||
    value.startsWith("magnet:") ||
    value.startsWith("p2p:") ||
    value.endsWith(".torrent")
  );
}

const URL_SCHEME_REGEX =
  /(?:https?:\/\/[^\s\u3000\u4e00-\u9fff，。！？、“”‘’（）《》〈〉【】^<>]+|magnet:\?xt=[^\s]+|p2p:[^\s]+)/gi;
const TRAILING_PUNCT_REGEX = /[.,;:!?'")\]>]+$/;

export function extractUrls(text: string): string[] {
  const trimmed = text.trim();
  if (!trimmed) return [];

  if (isUrl(trimmed)) {
    return [trimmed];
  }

  const matches = trimmed.match(URL_SCHEME_REGEX) ?? [];
  const results: string[] = [];

  for (const m of matches) {
    const cleaned = m.replace(TRAILING_PUNCT_REGEX, "").trim();
    if (cleaned && isUrl(cleaned) && !results.includes(cleaned)) {
      results.push(cleaned);
    }
  }

  if (results.length === 0) {
    const tokens = trimmed.split(/[\s\n]+/).filter(isUrl);
    for (const t of tokens) {
      if (!results.includes(t)) {
        results.push(t);
      }
    }
  }

  return results;
}

export function showInspectorForState(state: OmniState): boolean {
  return (
    state.kind === "detected" ||
    state.kind === "preparing" ||
    state.kind === "error"
  );
}

export function showOmniboxForState(state: OmniState): boolean {
  return (
    state.kind === "idle" ||
    state.kind === "detecting" ||
    state.kind === "detected" ||
    state.kind === "unsupported" ||
    state.kind === "batch" ||
    state.kind === "searching" ||
    state.kind === "search-results" ||
    state.kind === "search-empty"
  );
}
