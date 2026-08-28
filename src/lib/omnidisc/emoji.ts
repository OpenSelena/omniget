export interface EmojiEntry {
  char: string;
  name: string;
  group: string;
}

export const EMOJI_GROUPS = ["reactions", "people", "nature", "food", "activity", "objects", "symbols"] as const;

export type EmojiGroup = (typeof EMOJI_GROUPS)[number];

export const EMOJIS: EmojiEntry[] = [
  { char: "👍", name: "thumbs up yes ok", group: "reactions" },
  { char: "👎", name: "thumbs down no", group: "reactions" },
  { char: "❤️", name: "heart love", group: "reactions" },
  { char: "🔥", name: "fire hot", group: "reactions" },
  { char: "🎉", name: "party tada celebrate", group: "reactions" },
  { char: "😂", name: "joy laugh lol", group: "reactions" },
  { char: "😮", name: "wow surprised", group: "reactions" },
  { char: "😢", name: "sad cry", group: "reactions" },
  { char: "🙏", name: "pray thanks please", group: "reactions" },
  { char: "👀", name: "eyes looking", group: "reactions" },
  { char: "✅", name: "check done ok", group: "reactions" },
  { char: "❌", name: "cross no wrong", group: "reactions" },
  { char: "😀", name: "grin smile happy", group: "people" },
  { char: "😅", name: "sweat smile nervous", group: "people" },
  { char: "🙂", name: "slight smile", group: "people" },
  { char: "😉", name: "wink", group: "people" },
  { char: "😍", name: "heart eyes love", group: "people" },
  { char: "🤔", name: "thinking hmm", group: "people" },
  { char: "😴", name: "sleep tired", group: "people" },
  { char: "😎", name: "cool sunglasses", group: "people" },
  { char: "🤝", name: "handshake deal", group: "people" },
  { char: "👋", name: "wave hello bye", group: "people" },
  { char: "💪", name: "muscle strong", group: "people" },
  { char: "🧠", name: "brain smart", group: "people" },
  { char: "🐶", name: "dog puppy", group: "nature" },
  { char: "🐱", name: "cat kitten", group: "nature" },
  { char: "🦊", name: "fox", group: "nature" },
  { char: "🐢", name: "turtle slow", group: "nature" },
  { char: "🌱", name: "seedling plant grow", group: "nature" },
  { char: "🌊", name: "wave water sea", group: "nature" },
  { char: "☀️", name: "sun sunny", group: "nature" },
  { char: "🌙", name: "moon night", group: "nature" },
  { char: "⭐", name: "star", group: "nature" },
  { char: "🍕", name: "pizza food", group: "food" },
  { char: "🍔", name: "burger food", group: "food" },
  { char: "🍟", name: "fries food", group: "food" },
  { char: "🍰", name: "cake dessert", group: "food" },
  { char: "☕", name: "coffee cafe", group: "food" },
  { char: "🍺", name: "beer drink", group: "food" },
  { char: "🥤", name: "soda drink", group: "food" },
  { char: "🎮", name: "game controller play", group: "activity" },
  { char: "🎧", name: "headphones music listen", group: "activity" },
  { char: "🎵", name: "music note song", group: "activity" },
  { char: "🏆", name: "trophy win", group: "activity" },
  { char: "⚽", name: "soccer football", group: "activity" },
  { char: "🎬", name: "clapper movie film", group: "activity" },
  { char: "🚀", name: "rocket ship launch", group: "activity" },
  { char: "💻", name: "laptop computer code", group: "objects" },
  { char: "📱", name: "phone mobile", group: "objects" },
  { char: "🖥️", name: "desktop screen", group: "objects" },
  { char: "🎥", name: "camera video record", group: "objects" },
  { char: "📎", name: "paperclip attach", group: "objects" },
  { char: "📌", name: "pin pinned", group: "objects" },
  { char: "🔒", name: "lock secure private", group: "objects" },
  { char: "🔑", name: "key access", group: "objects" },
  { char: "🐛", name: "bug issue", group: "objects" },
  { char: "⚡", name: "zap fast lightning", group: "symbols" },
  { char: "💡", name: "idea bulb light", group: "symbols" },
  { char: "⚠️", name: "warning caution", group: "symbols" },
  { char: "♻️", name: "recycle repeat", group: "symbols" },
  { char: "➕", name: "plus add", group: "symbols" },
  { char: "➖", name: "minus remove", group: "symbols" },
  { char: "❓", name: "question help", group: "symbols" },
  { char: "❗", name: "exclamation important", group: "symbols" },
  { char: "💯", name: "hundred perfect", group: "symbols" },
];

const RECENT_KEY = "omnidisc.emoji.recent";
const RECENT_MAX = 12;

export function recentEmojis(): string[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((e): e is string => typeof e === "string").slice(0, RECENT_MAX) : [];
  } catch {
    return [];
  }
}

export function rememberEmoji(char: string): string[] {
  const next = [char, ...recentEmojis().filter((e) => e !== char)].slice(0, RECENT_MAX);
  if (typeof localStorage !== "undefined") {
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(next));
    } catch {
      return next;
    }
  }
  return next;
}

export function searchEmojis(query: string): EmojiEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return EMOJIS;
  return EMOJIS.filter((e) => e.name.includes(q) || e.char === q);
}
